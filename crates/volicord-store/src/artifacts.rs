use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Component, Path, PathBuf},
};

use rusqlite::{params, Transaction};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_types::values::UtcTimestamp;

use crate::{
    core_pipeline::{advance_project_utc_floor_tx, CoreProjectStore},
    sqlite::{begin_immediate_transaction, ARTIFACTS_DIR, ARTIFACTS_TMP_DIR},
    StoreError, StoreResult,
};

/// Placement marker for future artifact-store plumbing.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactStoreBoundary;

/// Current-byte verification outcome for a persistent artifact body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistentArtifactVerificationStatus {
    /// Stored integrity facts and current artifact bytes match.
    VerifiedCurrent,
    /// The stored availability or body path indicates a missing artifact.
    Missing,
    /// Current bytes are present but no longer match stored integrity facts.
    IntegrityFailed,
    /// The artifact body could not be accessed as a usable regular file.
    Unavailable,
    /// The stored path or resolved body escapes the artifact-store boundary.
    BoundaryViolation,
}

/// Result of verifying a persistent artifact body without mutating storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistentArtifactVerification {
    pub status: PersistentArtifactVerificationStatus,
    pub actual_sha256: Option<String>,
    pub actual_size_bytes: Option<u64>,
}

/// Stored facts needed for persistent artifact body verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersistentArtifactBodySpec<'a> {
    pub body_path: Option<&'a str>,
    pub sha256: Option<&'a str>,
    pub size_bytes: Option<u64>,
    pub content_type: Option<&'a str>,
    pub integrity_status: &'a str,
    pub availability_status: &'a str,
}

/// Storage representation for a staged payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagedPayloadKind {
    /// Safe textual body bytes may be stored.
    SafeTextBody,
    /// Only a safe textual notice is being stored for the source material.
    SafeNotice,
}

impl StagedPayloadKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::SafeTextBody => "safe_text_body",
            Self::SafeNotice => "safe_notice",
        }
    }
}

/// Input for creating one transient artifact staging row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactStagingInsert {
    pub handle_id: String,
    pub task_id: String,
    pub created_by_actor_source: String,
    pub display_name: String,
    pub content_type: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub redaction_state: String,
    pub relation_hint: Option<String>,
    pub payload_kind: StagedPayloadKind,
    pub safe_bytes_or_notice: Vec<u8>,
    pub created_at: String,
    pub expires_at: String,
}

/// Stored staged-handle facts returned after staging creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactStagingRecord {
    pub handle_id: String,
    pub task_id: String,
    pub created_by_actor_source: String,
    pub content_type: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub redaction_state: String,
    pub expires_at: String,
    pub tmp_path: String,
}

impl CoreProjectStore<'_> {
    /// Creates a transient `artifact_staging` row and stores safe staged bytes.
    ///
    /// This operation is storage-owned staging. It does not update
    /// `project_state.state_version`, append `authority_events`, create
    /// `tool_invocations`, or insert persistent `artifacts` rows.
    pub fn create_artifact_staging(
        &mut self,
        input: ArtifactStagingInsert,
    ) -> StoreResult<ArtifactStagingRecord> {
        self.require_mutation_context()?;
        let created_at = validate_insert(&input)?;

        let tmp_dir = self
            .project
            .project_home
            .join(ARTIFACTS_DIR)
            .join(ARTIFACTS_TMP_DIR);
        fs::create_dir_all(&tmp_dir)?;

        let tx = begin_immediate_transaction(&mut self.conn)?;
        let clock_floor = advance_project_utc_floor_tx(&tx, &self.project.project_id, &created_at)?;
        let result = insert_artifact_staging_tx(&tx, &self.project.project_id, &tmp_dir, input);

        match result {
            Ok((record, write_path)) => match tx.commit() {
                Ok(()) => {
                    self.remember_clock_sample(&clock_floor);
                    Ok(record)
                }
                Err(error) => {
                    let _ = fs::remove_file(write_path);
                    Err(StoreError::from(error))
                }
            },
            Err(error) => Err(error),
        }
    }
}

/// Verifies the current bytes for a persistent artifact under the artifact store.
///
/// The supplied `body_path` is artifact-store-relative. The verifier resolves
/// symlinks with `canonicalize`, requires the final target to remain inside the
/// artifact store, and hashes the current regular-file bytes without mutating
/// any artifact row.
pub fn verify_persistent_artifact_body(
    artifact_store_root: &Path,
    spec: &PersistentArtifactBodySpec<'_>,
) -> StoreResult<PersistentArtifactVerification> {
    match spec.integrity_status {
        "verified" => {}
        "corrupt" => {
            return Ok(verification(
                PersistentArtifactVerificationStatus::IntegrityFailed,
            ))
        }
        _ => {
            return Err(StoreError::schema_invariant(
                "project_state",
                "artifact integrity_status is outside the owner-defined value set",
            ));
        }
    }

    match spec.availability_status {
        "available" => {}
        "missing" => return Ok(verification(PersistentArtifactVerificationStatus::Missing)),
        "integrity_failed" => {
            return Ok(verification(
                PersistentArtifactVerificationStatus::IntegrityFailed,
            ));
        }
        "unavailable" => {
            return Ok(verification(
                PersistentArtifactVerificationStatus::Unavailable,
            ))
        }
        _ => {
            return Err(StoreError::schema_invariant(
                "project_state",
                "artifact status is outside the owner-defined value set",
            ));
        }
    }

    if spec
        .content_type
        .is_none_or(|value| value.trim().is_empty())
        || spec
            .sha256
            .is_none_or(|value| !is_lowercase_sha256_hex(value))
        || spec.size_bytes.is_none()
    {
        return Ok(verification(
            PersistentArtifactVerificationStatus::IntegrityFailed,
        ));
    }

    let Some(body_path) = spec.body_path else {
        return Ok(verification(
            PersistentArtifactVerificationStatus::Unavailable,
        ));
    };
    if body_path.trim().is_empty() {
        return Ok(verification(
            PersistentArtifactVerificationStatus::BoundaryViolation,
        ));
    }
    let Some(candidate) = persistent_body_candidate_path(artifact_store_root, body_path) else {
        return Ok(verification(
            PersistentArtifactVerificationStatus::BoundaryViolation,
        ));
    };
    let Some(canonical_root) = canonicalize_store_root(artifact_store_root)? else {
        return Ok(verification(
            PersistentArtifactVerificationStatus::Unavailable,
        ));
    };
    let canonical_body = match candidate.canonicalize() {
        Ok(path) => path,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(verification(PersistentArtifactVerificationStatus::Missing));
        }
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            return Ok(verification(
                PersistentArtifactVerificationStatus::Unavailable,
            ));
        }
        Err(error) => return Err(StoreError::Io(error)),
    };

    if !canonical_body.starts_with(&canonical_root) {
        return Ok(verification(
            PersistentArtifactVerificationStatus::BoundaryViolation,
        ));
    }

    let metadata = match fs::metadata(&canonical_body) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(verification(PersistentArtifactVerificationStatus::Missing));
        }
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            return Ok(verification(
                PersistentArtifactVerificationStatus::Unavailable,
            ));
        }
        Err(error) => return Err(StoreError::Io(error)),
    };
    if !metadata.is_file() {
        return Ok(verification(
            PersistentArtifactVerificationStatus::Unavailable,
        ));
    }

    let (actual_sha256, actual_size_bytes) = match hash_file(&canonical_body) {
        Ok(result) => result,
        Err(StoreError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(verification(PersistentArtifactVerificationStatus::Missing));
        }
        Err(StoreError::Io(error)) if error.kind() == io::ErrorKind::PermissionDenied => {
            return Ok(verification(
                PersistentArtifactVerificationStatus::Unavailable,
            ));
        }
        Err(error) => return Err(error),
    };
    let expected_sha256 = spec.sha256.expect("validated sha256 is present");
    let expected_size_bytes = spec.size_bytes.expect("validated size is present");
    if actual_size_bytes != expected_size_bytes || actual_sha256 != expected_sha256 {
        return Ok(PersistentArtifactVerification {
            status: PersistentArtifactVerificationStatus::IntegrityFailed,
            actual_sha256: Some(actual_sha256),
            actual_size_bytes: Some(actual_size_bytes),
        });
    }

    Ok(PersistentArtifactVerification {
        status: PersistentArtifactVerificationStatus::VerifiedCurrent,
        actual_sha256: Some(actual_sha256),
        actual_size_bytes: Some(actual_size_bytes),
    })
}

fn verification(status: PersistentArtifactVerificationStatus) -> PersistentArtifactVerification {
    PersistentArtifactVerification {
        status,
        actual_sha256: None,
        actual_size_bytes: None,
    }
}

pub(crate) fn persistent_body_path_from_staging_tmp_path(tmp_path: &str) -> StoreResult<String> {
    let components = normal_relative_path_components(tmp_path).ok_or_else(|| {
        StoreError::schema_invariant(
            "project_state",
            "staged artifact body path is not a safe relative path",
        )
    })?;
    if components.first().map(String::as_str) != Some(ARTIFACTS_DIR) {
        return Err(StoreError::schema_invariant(
            "project_state",
            "staged artifact body path is outside the artifact store",
        ));
    }

    let persistent_components = &components[1..];
    if persistent_components.is_empty() {
        return Err(StoreError::schema_invariant(
            "project_state",
            "staged artifact body path does not name a persistent artifact-store path",
        ));
    }
    if persistent_components.first().map(String::as_str) == Some(ARTIFACTS_DIR) {
        return Err(StoreError::schema_invariant(
            "project_state",
            "persistent artifact body path uses the project-home artifact-store prefix",
        ));
    }

    Ok(persistent_components.join("/"))
}

fn canonicalize_store_root(artifact_store_root: &Path) -> StoreResult<Option<PathBuf>> {
    match artifact_store_root.canonicalize() {
        Ok(path) => Ok(Some(path)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => Ok(None),
        Err(error) => Err(StoreError::Io(error)),
    }
}

fn persistent_body_candidate_path(
    artifact_store_root: &Path,
    stored_body_path: &str,
) -> Option<PathBuf> {
    let components = normal_relative_path_components(stored_body_path)?;
    if components.first().map(String::as_str) == Some(ARTIFACTS_DIR) {
        return None;
    }
    let relative = path_buf_from_components(&components);
    Some(artifact_store_root.join(relative))
}

fn normal_relative_path_components(value: &str) -> Option<Vec<String>> {
    if value.trim().is_empty() || value.contains('\\') || has_windows_drive_prefix(value) {
        return None;
    }

    let mut components = Vec::new();
    for component in Path::new(value).components() {
        match component {
            Component::Normal(value) => components.push(value.to_str()?.to_owned()),
            Component::CurDir
            | Component::ParentDir
            | Component::Prefix(_)
            | Component::RootDir => return None,
        }
    }

    if components.is_empty() {
        None
    } else {
        Some(components)
    }
}

fn has_windows_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn path_buf_from_components(components: &[String]) -> PathBuf {
    let mut path = PathBuf::new();
    for component in components {
        path.push(component);
    }
    path
}

fn hash_file(path: &Path) -> StoreResult<(String, u64)> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut size_bytes = 0u64;
    let mut buffer = [0u8; 8192];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        size_bytes = size_bytes
            .checked_add(u64::try_from(read).map_err(|_| StoreError::InvalidInput {
                detail: "artifact body read size does not fit in u64".to_owned(),
            })?)
            .ok_or_else(|| StoreError::InvalidInput {
                detail: "artifact body size does not fit in u64".to_owned(),
            })?;
    }

    let digest = hasher.finalize();
    Ok((lowercase_sha256_digest(&digest), size_bytes))
}

fn lowercase_sha256_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

pub(crate) fn insert_artifact_staging_tx(
    tx: &Transaction<'_>,
    project_id: &str,
    tmp_dir: &std::path::Path,
    input: ArtifactStagingInsert,
) -> StoreResult<(ArtifactStagingRecord, PathBuf)> {
    let handle_id = input.handle_id;
    let file_name = format!("{}.txt", path_component(&handle_id));
    let relative_tmp_path = format!("{ARTIFACTS_DIR}/{ARTIFACTS_TMP_DIR}/{file_name}");
    let write_path = tmp_dir.join(&file_name);
    let artifact_json = json_text(json!({
        "display_name": input.display_name,
        "relation_hint": input.relation_hint
    }))?;
    let safe_metadata_json = json_text(json!({
        "payload_kind": input.payload_kind.as_str(),
        "stored_representation": input.payload_kind.as_str()
    }))?;
    let metadata_json = json_text(json!({
        "staging_ttl_hours": 24
    }))?;
    let size_bytes = u64_to_i64("artifact_staging.size_bytes", input.size_bytes)?;

    fs::write(&write_path, &input.safe_bytes_or_notice)?;

    let insert_result = tx.execute(
        "INSERT INTO artifact_staging (
            project_id,
            handle_id,
            task_id,
            created_by_actor_source,
            artifact_json,
            safe_metadata_json,
            tmp_path,
            sha256,
            size_bytes,
            content_type,
            redaction_state,
            status,
            expires_at,
            consumed_by_run_id,
            promoted_artifact_id,
            consumed_at,
            created_at,
            metadata_json
        )
        VALUES (
            ?1,
            ?2,
            ?3,
            ?4,
            ?5,
            ?6,
            ?7,
            ?8,
            ?9,
            ?10,
            ?11,
            'staged',
            ?12,
            NULL,
            NULL,
            NULL,
            ?13,
            ?14
        )",
        params![
            project_id,
            handle_id,
            input.task_id,
            input.created_by_actor_source,
            artifact_json,
            safe_metadata_json,
            relative_tmp_path,
            input.sha256,
            size_bytes,
            input.content_type,
            input.redaction_state,
            input.expires_at,
            input.created_at,
            metadata_json
        ],
    );

    if let Err(error) = insert_result {
        let _ = fs::remove_file(&write_path);
        return Err(StoreError::from(error));
    }

    Ok((
        ArtifactStagingRecord {
            handle_id,
            task_id: input.task_id,
            created_by_actor_source: input.created_by_actor_source,
            content_type: input.content_type,
            sha256: input.sha256,
            size_bytes: input.size_bytes,
            redaction_state: input.redaction_state,
            expires_at: input.expires_at,
            tmp_path: relative_tmp_path,
        },
        write_path,
    ))
}

pub(crate) fn validate_insert(input: &ArtifactStagingInsert) -> StoreResult<UtcTimestamp> {
    validate_identifier("handle_id", &input.handle_id)?;
    validate_identifier("task_id", &input.task_id)?;
    validate_identifier("created_by_actor_source", &input.created_by_actor_source)?;
    validate_nonempty_text("display_name", &input.display_name)?;
    validate_nonempty_text("content_type", &input.content_type)?;
    validate_artifact_sha256("sha256", &input.sha256)?;
    validate_identifier("redaction_state", &input.redaction_state)?;
    let created_at = validate_timestamp("created_at", &input.created_at)?;
    let expires_at = validate_timestamp("expires_at", &input.expires_at)?;
    if created_at >= expires_at {
        return Err(StoreError::InvalidInput {
            detail: "expires_at must be later than created_at".to_owned(),
        });
    }
    Ok(created_at)
}

fn path_component(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
            output.push(ch);
        } else {
            output.push('_');
        }
    }
    let trimmed = output.trim_matches(['_', '.', '-']);
    if trimmed.is_empty() {
        "staged_artifact".to_owned()
    } else {
        trimmed.chars().take(120).collect()
    }
}

fn validate_identifier(field: &'static str, value: &str) -> StoreResult<()> {
    if value.trim().is_empty() {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not be empty"),
        })
    } else {
        Ok(())
    }
}

fn validate_timestamp(field: &'static str, value: &str) -> StoreResult<UtcTimestamp> {
    UtcTimestamp::parse(value)
        .and_then(|timestamp| {
            timestamp
                .ensure_canonical_rfc3339_representable()
                .map(|()| timestamp)
                .map_err(|_| volicord_types::values::UtcTimestampParseError)
        })
        .map_err(|_| StoreError::InvalidInput {
            detail: format!("{field} must be a valid RFC 3339 timestamp"),
        })
}

fn validate_nonempty_text(field: &'static str, value: &str) -> StoreResult<()> {
    if value.trim().is_empty() {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} must not be empty"),
        });
    }
    if value.chars().any(char::is_control) {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} must not contain control characters"),
        });
    }
    Ok(())
}

fn validate_artifact_sha256(field: &'static str, value: &str) -> StoreResult<()> {
    if !is_lowercase_sha256_hex(value) {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} must be a lowercase 64-character SHA-256 hex string"),
        });
    }
    Ok(())
}

fn is_lowercase_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn json_text(value: Value) -> StoreResult<String> {
    serde_json::to_string(&value).map_err(|error| StoreError::InvalidInput {
        detail: format!("artifact staging JSON could not be encoded: {error}"),
    })
}

fn u64_to_i64(field: &'static str, value: u64) -> StoreResult<i64> {
    i64::try_from(value).map_err(|_| StoreError::InvalidInput {
        detail: format!("{field} does not fit in SQLite integer"),
    })
}

#[cfg(test)]
mod tests {
    use std::{error::Error, fs};

    use chrono::{DateTime, Utc};
    use volicord_test_support::{core_fixtures::CoreFixture, TempRuntimeHome};
    use volicord_types::ids::ProjectId;

    use super::*;
    use crate::mutation::TestRuntimeHomeAdmission;

    #[test]
    fn artifact_staging_advances_utc_floor_without_advancing_state_version(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("artifact-staging-clock-floor")?;
        let mutation = TestRuntimeHomeAdmission::shared(fixture.runtime_home_path())?;
        let context = mutation.context()?;
        let mut store =
            CoreProjectStore::open_for_mutation(&context, &ProjectId::new(fixture.project_id()))?;
        store.conn.execute(
            "INSERT INTO tasks (
                project_id, task_id, created_by_actor_source, mode,
                requested_control_level, effective_control_level, control_level_reason, work_phase,
                acceptance_policy, acceptance_policy_reason, carry_forward_json,
                lifecycle_phase, created_at, updated_at
            ) VALUES (
                ?1, 'task_staging_floor', ?2, 'work',
                'tracked', 'tracked', 'Artifact staging fixture control.', 'implementation',
                'required', 'Staging floor fixture.', '[]', 'implementation',
                '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'
            )",
            params![fixture.project_id(), fixture.actor_source()],
        )?;
        store.conn.execute(
            "UPDATE project_state SET updated_at = '2026-01-01T00:00:00Z'
              WHERE project_id = ?1",
            [fixture.project_id()],
        )?;
        let before_state_version = store.project_state()?.state_version;
        let bytes = b"safe staged payload".to_vec();
        let created_at = "2999-07-13T12:34:56.789Z";
        store.create_artifact_staging(ArtifactStagingInsert {
            handle_id: "staged_clock_floor".to_owned(),
            task_id: "task_staging_floor".to_owned(),
            created_by_actor_source: fixture.actor_source(),
            display_name: "clock-floor.txt".to_owned(),
            content_type: "text/plain".to_owned(),
            sha256: sha256_hex(&bytes),
            size_bytes: u64::try_from(bytes.len())?,
            redaction_state: "redacted".to_owned(),
            relation_hint: None,
            payload_kind: StagedPayloadKind::SafeTextBody,
            safe_bytes_or_notice: bytes,
            created_at: created_at.to_owned(),
            expires_at: "2999-07-13T13:34:56.789Z".to_owned(),
        })?;

        let state = store.project_state()?;
        assert_eq!(state.state_version, before_state_version);
        assert_eq!(state.updated_at, created_at);
        assert!(
            UtcTimestamp::parse(&store.current_timestamp()?)? >= UtcTimestamp::parse(created_at)?
        );
        drop(store);
        let reopened = CoreProjectStore::open_read_only(
            fixture.runtime_home_path(),
            &ProjectId::new(fixture.project_id()),
        )?;
        assert_eq!(reopened.project_state()?.updated_at, created_at);
        Ok(())
    }

    #[test]
    fn artifact_staging_rejects_invalid_clock_bounds_without_side_effects(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("artifact-staging-invalid-clock-bounds")?;
        let mutation = TestRuntimeHomeAdmission::shared(fixture.runtime_home_path())?;
        let context = mutation.context()?;
        let mut store =
            CoreProjectStore::open_for_mutation(&context, &ProjectId::new(fixture.project_id()))?;
        store.conn.execute(
            "INSERT INTO tasks (
                project_id, task_id, created_by_actor_source, mode,
                requested_control_level, effective_control_level, control_level_reason, work_phase,
                acceptance_policy, acceptance_policy_reason, carry_forward_json,
                lifecycle_phase, created_at, updated_at
            ) VALUES (
                ?1, 'task_staging_invalid_clock', ?2, 'work',
                'tracked', 'tracked', 'Artifact staging fixture control.', 'implementation',
                'required', 'Invalid staging clock fixture.', '[]', 'implementation',
                '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'
            )",
            params![fixture.project_id(), fixture.actor_source()],
        )?;
        store.conn.execute(
            "UPDATE project_state SET updated_at = '2026-01-01T00:00:00Z'
              WHERE project_id = ?1",
            [fixture.project_id()],
        )?;
        let before_state = store.project_state()?;
        let tmp_dir = store
            .project
            .project_home
            .join(ARTIFACTS_DIR)
            .join(ARTIFACTS_TMP_DIR);
        assert!(!tmp_dir.exists());

        let year_10000 = "9999-12-31T23:59:59-23:59";
        let parsed_year_10000 =
            UtcTimestamp::parse(year_10000).expect("the offset normalizes into UTC year +10000");
        assert!(parsed_year_10000.to_string().starts_with("+10000-"));
        assert!(parsed_year_10000
            .ensure_canonical_rfc3339_representable()
            .is_err());
        let chrono_max = UtcTimestamp::from_datetime(DateTime::<Utc>::MAX_UTC).to_string();
        assert!(UtcTimestamp::parse(&chrono_max).is_err());

        for (handle_id, created_at, expires_at, expected_detail) in [
            (
                "staged_year_10000",
                year_10000.to_owned(),
                "2999-07-13T13:34:56.789Z".to_owned(),
                "created_at must be a valid RFC 3339 timestamp",
            ),
            (
                "staged_chrono_max_expiry",
                "2999-07-13T12:34:56.789Z".to_owned(),
                chrono_max,
                "expires_at must be a valid RFC 3339 timestamp",
            ),
            (
                "staged_equal_expiry",
                "2999-07-13T12:34:56.789Z".to_owned(),
                "2999-07-13T12:34:56.789Z".to_owned(),
                "expires_at must be later than created_at",
            ),
            (
                "staged_reversed_expiry",
                "2999-07-13T12:34:56.789Z".to_owned(),
                "2999-07-13T11:34:56.789Z".to_owned(),
                "expires_at must be later than created_at",
            ),
        ] {
            let bytes = b"safe staged payload".to_vec();
            let error = store
                .create_artifact_staging(ArtifactStagingInsert {
                    handle_id: handle_id.to_owned(),
                    task_id: "task_staging_invalid_clock".to_owned(),
                    created_by_actor_source: fixture.actor_source(),
                    display_name: format!("{handle_id}.txt"),
                    content_type: "text/plain".to_owned(),
                    sha256: sha256_hex(&bytes),
                    size_bytes: u64::try_from(bytes.len())?,
                    redaction_state: "redacted".to_owned(),
                    relation_hint: None,
                    payload_kind: StagedPayloadKind::SafeTextBody,
                    safe_bytes_or_notice: bytes,
                    created_at,
                    expires_at,
                })
                .expect_err("invalid staging clocks must fail before storage effects");
            match error {
                StoreError::InvalidInput { detail } => assert_eq!(detail, expected_detail),
                other => panic!("unexpected staging error: {other}"),
            }

            let row_count: i64 = store.conn.query_row(
                "SELECT COUNT(*) FROM artifact_staging
                  WHERE project_id = ?1 AND handle_id = ?2",
                params![fixture.project_id(), handle_id],
                |row| row.get(0),
            )?;
            assert_eq!(row_count, 0, "{handle_id} must not leave a staging row");
            assert_eq!(store.project_state()?, before_state);
            assert!(!tmp_dir.exists(), "{handle_id} must not create the tmp dir");
        }
        Ok(())
    }

    #[test]
    fn read_only_store_cannot_stage_artifact_bytes() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("artifact-staging-read-only")?;
        let mut store = CoreProjectStore::open_read_only(
            fixture.runtime_home_path(),
            &ProjectId::new(fixture.project_id()),
        )?;
        let tmp_dir = store
            .project
            .project_home
            .join(ARTIFACTS_DIR)
            .join(ARTIFACTS_TMP_DIR);
        let bytes = b"must not be staged".to_vec();

        let error = store
            .create_artifact_staging(ArtifactStagingInsert {
                handle_id: "staged_read_only".to_owned(),
                task_id: "task_read_only".to_owned(),
                created_by_actor_source: fixture.actor_source(),
                display_name: "read-only.txt".to_owned(),
                content_type: "text/plain".to_owned(),
                sha256: sha256_hex(&bytes),
                size_bytes: u64::try_from(bytes.len())?,
                redaction_state: "redacted".to_owned(),
                relation_hint: None,
                payload_kind: StagedPayloadKind::SafeTextBody,
                safe_bytes_or_notice: bytes,
                created_at: "2026-07-25T00:00:00Z".to_owned(),
                expires_at: "2026-07-25T01:00:00Z".to_owned(),
            })
            .expect_err("read-only Store must not authorize artifact staging");

        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert!(!tmp_dir.exists());
        Ok(())
    }

    #[test]
    fn staging_tmp_path_converts_to_artifact_store_relative_body_path() -> Result<(), Box<dyn Error>>
    {
        assert_eq!(
            persistent_body_path_from_staging_tmp_path("artifacts/tmp/body.txt")?,
            "tmp/body.txt"
        );
        assert_eq!(
            persistent_body_path_from_staging_tmp_path("artifacts/tmp/nested/body.txt")?,
            "tmp/nested/body.txt"
        );

        for invalid in [
            "",
            "tmp/body.txt",
            "artifacts",
            "artifacts/../tmp/body.txt",
            "artifacts/artifacts/tmp/body.txt",
            "/artifacts/tmp/body.txt",
            "C:artifacts/tmp/body.txt",
            "artifacts\\tmp\\body.txt",
        ] {
            assert!(
                persistent_body_path_from_staging_tmp_path(invalid).is_err(),
                "{invalid:?} should not convert"
            );
        }

        Ok(())
    }

    #[test]
    fn persistent_body_verifier_uses_artifact_store_relative_path() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("persistent-body-canonical")?;
        let artifact_store_root = fixture.path().join("artifacts");
        let body_dir = artifact_store_root.join("tmp");
        fs::create_dir_all(&body_dir)?;
        let bytes = b"{\"result\":\"ok\"}";
        fs::write(body_dir.join("body.txt"), bytes)?;
        let sha256 = sha256_hex(bytes);

        let verification = verify_persistent_artifact_body(
            &artifact_store_root,
            &verified_body_spec("tmp/body.txt", &sha256, bytes.len() as u64),
        )?;

        assert_eq!(
            verification.status,
            PersistentArtifactVerificationStatus::VerifiedCurrent
        );
        assert_eq!(verification.actual_sha256.as_deref(), Some(sha256.as_str()));
        assert_eq!(verification.actual_size_bytes, Some(bytes.len() as u64));
        Ok(())
    }

    #[test]
    fn persistent_body_verifier_rejects_project_home_relative_prefix() -> Result<(), Box<dyn Error>>
    {
        let fixture = TempRuntimeHome::new("persistent-body-noncanonical-prefix")?;
        let artifact_store_root = fixture.path().join("artifacts");
        let body_dir = artifact_store_root.join("tmp");
        fs::create_dir_all(&body_dir)?;
        let bytes = b"{\"result\":\"ok\"}";
        fs::write(body_dir.join("body.txt"), bytes)?;
        let sha256 = sha256_hex(bytes);

        let verification = verify_persistent_artifact_body(
            &artifact_store_root,
            &verified_body_spec("artifacts/tmp/body.txt", &sha256, bytes.len() as u64),
        )?;

        assert_eq!(
            verification.status,
            PersistentArtifactVerificationStatus::BoundaryViolation
        );
        Ok(())
    }

    #[test]
    fn persistent_body_verifier_rejects_unsafe_stored_path_shapes() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("persistent-body-invalid-paths")?;
        let artifact_store_root = fixture.path().join("artifacts");
        fs::create_dir_all(artifact_store_root.join("tmp"))?;
        let bytes = b"{\"result\":\"ok\"}";
        let sha256 = sha256_hex(bytes);

        for invalid in [
            "",
            "/tmp/body.txt",
            "../tmp/body.txt",
            "tmp/../body.txt",
            "tmp/body\\name.txt",
            "C:tmp/body.txt",
            "artifacts/tmp/body.txt",
        ] {
            let verification = verify_persistent_artifact_body(
                &artifact_store_root,
                &verified_body_spec(invalid, &sha256, bytes.len() as u64),
            )?;
            assert_eq!(
                verification.status,
                PersistentArtifactVerificationStatus::BoundaryViolation,
                "{invalid:?} should be rejected"
            );
        }

        Ok(())
    }

    fn verified_body_spec<'a>(
        body_path: &'a str,
        sha256: &'a str,
        size_bytes: u64,
    ) -> PersistentArtifactBodySpec<'a> {
        PersistentArtifactBodySpec {
            body_path: Some(body_path),
            sha256: Some(sha256),
            size_bytes: Some(size_bytes),
            content_type: Some("application/json"),
            integrity_status: "verified",
            availability_status: "available",
        }
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        let digest = Sha256::digest(bytes);
        lowercase_sha256_digest(&digest)
    }
}
