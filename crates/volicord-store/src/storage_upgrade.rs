//! Explicit offline copy conversion from `baseline_sqlite_v6` to v7.
//!
//! Normal database opening remains strict. This module is the single owner-
//! documented exception: it reads a complete v6 Runtime Home through read-only
//! connections and builds a separate, freshly initialized v7 Runtime Home.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{
    params, params_from_iter, types::Value as SqlValue, Connection, OptionalExtension, Transaction,
    TransactionBehavior,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_types::{canonical_json_sha256, canonical_json_string, WriteTicketAttemptScope};

use crate::{
    artifacts::{
        verify_persistent_artifact_body, PersistentArtifactBodySpec,
        PersistentArtifactVerificationStatus,
    },
    diagnostics::{
        ensure_current_diagnostics_schema, DIAGNOSTICS_DB_FILE, DIAGNOSTICS_SCHEMA_V1_SQL,
        DIAGNOSTICS_SCHEMA_V2_SQL,
    },
    schema::{PROJECT_STATE_SCHEMA_SQL, REGISTRY_SCHEMA_SQL, STORAGE_PROFILE},
    sqlite::{
        open_project_state_database, open_read_only_database, open_registry_database,
        project_state_db_path, registry_db_path, validate_project_state_schema,
        validate_registry_schema, ARTIFACTS_DIR, PROJECTS_DIR, PROJECT_STATE_DB_FILE,
    },
    StoreError, StoreResult,
};

/// Only source storage profile accepted by the offline converter.
pub const STORAGE_UPGRADE_SOURCE_PROFILE: &str = "baseline_sqlite_v6";
/// Content-free report written inside a successfully converted Runtime Home.
pub const STORAGE_UPGRADE_REPORT_FILE: &str = "storage-upgrade-report.json";

const DATABASE_KIND: &str = "storage_upgrade";
const INCOMPLETE_MARKER_FILE: &str = ".volicord-storage-upgrade-incomplete.json";
const LEGACY_PROJECT_POLICY_FILE: &str = ".volicord/policy.json";
const MAX_PROJECT_POLICY_BYTES: u64 = 1024 * 1024;
const V6_PROJECT_SCHEMA_SHA256: &str =
    "b2fbcc6a22d7990874dd830fdffd0752b5c21315430285fee75f7754c39a158a";
const V6_REGISTRY_SCHEMA_SHA256: &str =
    "4dfd1017f3a292dd47a73aad209d64e708e831e1fd23ba7c706c28edf4ab2dc6";

const V1_POLICY_TOP_LEVEL_KEYS: &[&str] = &[
    "schema",
    "managed_by",
    "storage_scope",
    "connection_intent",
    "host",
    "repo_root",
    "connection_id",
    "guard_installation_id",
    "selected_profile",
    "mcp",
    "host_hook",
];
const POLICY_MCP_KEYS: &[&str] = &["command", "args", "env"];
const POLICY_MCP_ENV_KEYS: &[&str] = &[
    "VOLICORD_HOME",
    "VOLICORD_MCP_LAUNCH",
    "VOLICORD_MCP_HOST",
    "VOLICORD_MCP_CONNECTION_ID",
    "VOLICORD_MCP_PROJECT_ID",
];
const POLICY_HOST_HOOK_KEYS: &[&str] = &["enabled", "commands"];
const POLICY_HOOK_PHASE_KEYS: &[&str] = &[
    "session_start",
    "pre_tool",
    "post_tool",
    "prompt_capture",
    "stop",
];
const POLICY_HOOK_COMMAND_KEYS: &[&str] = &["command", "args"];

const V7_TASK_CONTROL_COLUMNS: &str = "  requested_control_level TEXT NOT NULL CHECK (requested_control_level IN ('auto', 'observe', 'light', 'tracked', 'sensitive')),\n  effective_control_level TEXT NOT NULL CHECK (effective_control_level IN ('observe', 'light', 'tracked', 'sensitive')),\n  control_level_reason TEXT NOT NULL CHECK (length(trim(control_level_reason)) > 0),\n";

const V7_WRITE_TICKETS_SCHEMA: &str = r#"CREATE TABLE write_tickets (
  project_id TEXT NOT NULL,
  write_ticket_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version > 0),
  status TEXT NOT NULL CHECK (status IN ('active', 'consumed', 'invalidated', 'revoked')),
  validity_basis_json TEXT NOT NULL,
  allowed_path_prefixes_json TEXT NOT NULL DEFAULT '[]',
  denied_path_prefixes_json TEXT NOT NULL DEFAULT '[]',
  attempt_scope_json TEXT NOT NULL DEFAULT '{}',
  created_by_actor_source TEXT NOT NULL,
  created_by_user_action_resolution_id TEXT,
  idle_expires_at TEXT,
  invalidation_reason TEXT CHECK (
    invalidation_reason IS NULL OR invalidation_reason IN (
      'scope_revision_changed', 'change_unit_changed', 'baseline_changed',
      'workspace_changed', 'approval_basis_changed', 'idle_timeout',
      'task_closed', 'explicit_revoke'
    )
  ),
  consumed_by_run_id TEXT,
  consumed_at TEXT,
  revoked_at TEXT,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, write_ticket_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, created_by_user_action_resolution_id)
    REFERENCES user_action_resolutions (project_id, user_action_resolution_id),
  FOREIGN KEY (project_id, consumed_by_run_id)
    REFERENCES runs (project_id, run_id)
    DEFERRABLE INITIALLY DEFERRED
);
"#;

const V6_WRITE_TICKETS_SCHEMA: &str = r#"CREATE TABLE write_tickets (
  project_id TEXT NOT NULL,
  write_ticket_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version > 0),
  status TEXT NOT NULL CHECK (status IN ('active', 'consumed', 'expired', 'stale', 'revoked')),
  attempt_scope_json TEXT NOT NULL DEFAULT '{}',
  created_by_actor_source TEXT NOT NULL,
  created_by_user_action_resolution_id TEXT,
  expires_at TEXT NOT NULL,
  consumed_by_run_id TEXT,
  consumed_at TEXT,
  revoked_at TEXT,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, write_ticket_id),
  UNIQUE (project_id, task_id, basis_state_version),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, created_by_user_action_resolution_id)
    REFERENCES user_action_resolutions (project_id, user_action_resolution_id),
  FOREIGN KEY (project_id, consumed_by_run_id)
    REFERENCES runs (project_id, run_id)
    DEFERRABLE INITIALLY DEFERRED
);
"#;

const V7_UNRECORDED_CONFIDENCE_COLUMN: &str =
    "  confidence TEXT NOT NULL CHECK (confidence IN ('confirmed', 'suspected')),\n";
const V7_AUTHORITY_EVENT_TASK_ID_COLUMN: &str =
    "  task_id TEXT,\n  change_unit_id TEXT,\n  payload_json TEXT NOT NULL DEFAULT '{}',\n";
const V6_AUTHORITY_EVENT_TASK_ID_COLUMN: &str =
    "  task_id TEXT NOT NULL,\n  change_unit_id TEXT,\n  payload_json TEXT NOT NULL DEFAULT '{}',\n";
const V7_PROJECT_SCOPED_AUTHORITY_EVENT_CHECK: &str = "  CHECK (\n    (event_type = 'project_workflow_policy_applied'\n      AND task_id IS NULL AND change_unit_id IS NULL)\n    OR (event_type <> 'project_workflow_policy_applied' AND task_id IS NOT NULL)\n  ),\n";
const V7_TASK_EVENTS_FILTER: &str =
    "FROM authority_events\nWHERE task_id IS NOT NULL;\n\nCREATE TABLE tool_invocations";
const V6_TASK_EVENTS_FILTER: &str = "FROM authority_events;\n\nCREATE TABLE tool_invocations";
const V7_PROJECT_ONLY_SCHEMA_START: &str = "\nCREATE TABLE project_workflow_policies (";

const REGISTRY_TABLES: &[&str] = &[
    "runtime_home",
    "installation_profile",
    "projects",
    "project_aliases",
    "agent_connections",
    "connection_projects",
    "host_capability_verifications",
    "host_capability_state",
    "guard_installations",
];

const PROJECT_V6_TABLES: &[&str] = &[
    "project_state",
    "tasks",
    "acceptance_criteria",
    "evidence_claims",
    "change_units",
    "evidence_capture_intents",
    "user_action_requests",
    "user_action_resolutions",
    "project_continuity_records",
    "write_tickets",
    "runs",
    "artifact_staging",
    "evidence_capture_receipts",
    "evidence_capture_source_claims",
    "artifacts",
    "artifact_links",
    "evidence_summaries",
    "evidence_observations",
    "evidence_producers",
    "blockers",
    "authority_events",
    "tool_invocations",
    "agent_sessions",
    "guard_events",
    "prompt_captures",
    "unrecorded_changes",
    "expected_writes",
    "session_watch_baselines",
    "session_watch_observations",
    "user_action_channel_tokens",
];

const PROJECT_GENERIC_TABLES: &[&str] = &[
    "acceptance_criteria",
    "evidence_claims",
    "change_units",
    "evidence_capture_intents",
    "user_action_requests",
    "user_action_resolutions",
    "project_continuity_records",
    "runs",
    "artifact_staging",
    "evidence_capture_receipts",
    "evidence_capture_source_claims",
    "artifacts",
    "artifact_links",
    "evidence_summaries",
    "evidence_observations",
    "evidence_producers",
    "blockers",
    "authority_events",
    "tool_invocations",
    "agent_sessions",
    "prompt_captures",
    "expected_writes",
    "session_watch_baselines",
    "session_watch_observations",
    "user_action_channel_tokens",
];

/// Bounded aggregate report returned by the offline conversion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageUpgradeReport {
    pub status: String,
    pub source_profile: String,
    pub destination_profile: String,
    pub source_home: String,
    pub destination_home: String,
    pub completed_stages: Vec<String>,
    pub preserved_record_counts: BTreeMap<String, u64>,
    pub canonical_hash_check_count: u64,
    pub source_unchanged: bool,
    pub destination_ready: bool,
    pub activation_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IncompleteUpgradeMarker {
    status: String,
    source_profile: String,
    destination_profile: String,
    source_home: String,
    destination_home: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceProject {
    project_id: String,
    repo_root: PathBuf,
    source_project_home: PathBuf,
    source_state_db_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConvertedProjectPolicy {
    canonical_json: String,
    fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileManifestEntry {
    relative_path: PathBuf,
    kind: &'static str,
    size_bytes: u64,
    sha256: Option<String>,
    link_target: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SchemaObject {
    object_type: String,
    name: String,
    table_name: String,
    definition: Option<String>,
}

/// Converts a complete v6 Runtime Home into a separate, fresh v7 Runtime Home.
pub fn upgrade_runtime_home_v6_to_v7(
    source_home: impl AsRef<Path>,
    destination_home: impl AsRef<Path>,
) -> StoreResult<StorageUpgradeReport> {
    let source_home = canonical_source_home(source_home.as_ref())?;
    let destination_home = canonical_destination_home(destination_home.as_ref())?;
    validate_distinct_runtime_homes(&source_home, &destination_home)?;
    validate_destination_is_available(&destination_home)?;

    let staging_home = staging_home_path(&destination_home)?;
    validate_distinct_runtime_homes(&source_home, &staging_home)?;
    prepare_owned_staging_home(&staging_home, &source_home, &destination_home)?;

    let source_manifest_before = runtime_home_manifest(&source_home)?;
    let source_registry_path = registry_db_path(&source_home);
    let source_registry = open_read_only_database(&source_registry_path)?;
    validate_v6_registry(&source_registry, &source_home)?;
    let source_projects = read_source_projects(&source_registry, &source_home)?;
    let converted_project_policies = source_projects
        .iter()
        .map(|project| {
            Ok((
                project.project_id.clone(),
                converted_project_policy(&source_registry, &source_home, project)?,
            ))
        })
        .collect::<StoreResult<BTreeMap<_, _>>>()?;
    let detective_guard_installations = detective_guard_installation_ids(&source_registry)?;
    validate_source_database_inventory(&source_home, &source_projects, &source_manifest_before)?;

    let marker = IncompleteUpgradeMarker {
        status: "incomplete".to_owned(),
        source_profile: STORAGE_UPGRADE_SOURCE_PROFILE.to_owned(),
        destination_profile: STORAGE_PROFILE.to_owned(),
        source_home: path_text(&source_home, "source_home")?,
        destination_home: path_text(&destination_home, "destination_home")?,
    };
    fs::create_dir(&staging_home)?;
    write_json_file(&staging_home.join(INCOMPLETE_MARKER_FILE), &marker)?;

    let converted_at =
        DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::Millis, true);
    let mut preserved_record_counts = BTreeMap::new();
    copy_registry(
        &source_registry,
        &staging_home,
        &destination_home,
        &mut preserved_record_counts,
    )?;

    let mut canonical_hash_check_count = 0u64;
    for project in &source_projects {
        let source_conn = open_read_only_database(&project.source_state_db_path)?;
        validate_v6_project_state(&source_conn, &project.project_id)?;
        let destination_state_path = project_state_db_path(&staging_home, &project.project_id);
        if let Some(parent) = destination_state_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut destination_conn = open_project_state_database(&destination_state_path)?;
        copy_project_state(
            &source_conn,
            &mut destination_conn,
            &project.project_id,
            &converted_at,
            &detective_guard_installations,
            converted_project_policies
                .get(&project.project_id)
                .ok_or_else(|| upgrade_invariant("converted project policy is missing"))?,
            &mut preserved_record_counts,
        )?;
        validate_project_state_schema(&destination_conn)?;
        canonical_hash_check_count = canonical_hash_check_count
            .checked_add(verify_preserved_project_records(
                &source_conn,
                &destination_conn,
            )?)
            .ok_or_else(|| upgrade_invariant("canonical hash check count overflow"))?;
        copy_project_artifacts(project, &staging_home)?;
        verify_artifact_records(&source_conn, &project.source_project_home)?;
        verify_artifact_records(
            &destination_conn,
            &staging_home.join(PROJECTS_DIR).join(&project.project_id),
        )?;
    }

    copy_optional_diagnostics(&source_home, &staging_home)?;
    validate_destination_registry(
        &staging_home,
        &destination_home,
        &source_projects,
        &preserved_record_counts,
    )?;

    validate_v6_registry(&source_registry, &source_home)?;
    if read_source_projects(&source_registry, &source_home)? != source_projects {
        return Err(upgrade_invariant(
            "source project profile coordinates changed during conversion",
        ));
    }
    for project in &source_projects {
        if converted_project_policy(&source_registry, &source_home, project)?
            != *converted_project_policies
                .get(&project.project_id)
                .ok_or_else(|| upgrade_invariant("converted project policy is missing"))?
        {
            return Err(upgrade_invariant(format!(
                "v6 project {} managed policy changed during conversion",
                project.project_id
            )));
        }
    }
    let source_manifest_after = runtime_home_manifest(&source_home)?;
    if source_manifest_before != source_manifest_after {
        return Err(upgrade_invariant(
            "source Runtime Home changed during read-only conversion",
        ));
    }

    let report = StorageUpgradeReport {
        status: "completed".to_owned(),
        source_profile: STORAGE_UPGRADE_SOURCE_PROFILE.to_owned(),
        destination_profile: STORAGE_PROFILE.to_owned(),
        source_home: path_text(&source_home, "source_home")?,
        destination_home: path_text(&destination_home, "destination_home")?,
        completed_stages: vec![
            "source_validated_read_only".to_owned(),
            "destination_initialized".to_owned(),
            "records_transformed".to_owned(),
            "foreign_keys_and_counts_verified".to_owned(),
            "canonical_hashes_verified".to_owned(),
            "conversion_report_written".to_owned(),
            "source_immutability_rechecked".to_owned(),
        ],
        preserved_record_counts,
        canonical_hash_check_count,
        source_unchanged: true,
        destination_ready: true,
        activation_action: "administrator_rebind_destination_home_and_activate_separately"
            .to_owned(),
    };
    write_json_file(&staging_home.join(STORAGE_UPGRADE_REPORT_FILE), &report)?;
    fs::remove_file(staging_home.join(INCOMPLETE_MARKER_FILE))?;
    promote_staging_home(&staging_home, &destination_home)?;
    Ok(report)
}

fn canonical_source_home(path: &Path) -> StoreResult<PathBuf> {
    if !path.exists() || !path.is_dir() {
        return Err(StoreError::NotFound {
            entity: "source_runtime_home",
            id: path.display().to_string(),
        });
    }
    fs::canonicalize(path).map_err(Into::into)
}

fn canonical_destination_home(path: &Path) -> StoreResult<PathBuf> {
    if !path.is_absolute() {
        return Err(StoreError::InvalidInput {
            detail: "destination Runtime Home path must be absolute".to_owned(),
        });
    }
    reject_non_normal_path(path, "destination Runtime Home")?;
    if path.exists() {
        return fs::canonicalize(path).map_err(Into::into);
    }
    let parent = path.parent().ok_or_else(|| StoreError::InvalidInput {
        detail: "destination Runtime Home must have an existing parent directory".to_owned(),
    })?;
    let file_name = path.file_name().ok_or_else(|| StoreError::InvalidInput {
        detail: "destination Runtime Home must name a directory".to_owned(),
    })?;
    let parent = fs::canonicalize(parent)?;
    Ok(parent.join(file_name))
}

fn reject_non_normal_path(path: &Path, field: &str) -> StoreResult<()> {
    if path
        .components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} path must be normalized without . or .. components"),
        });
    }
    Ok(())
}

fn validate_distinct_runtime_homes(source: &Path, destination: &Path) -> StoreResult<()> {
    if source == destination || source.starts_with(destination) || destination.starts_with(source) {
        return Err(StoreError::InvalidInput {
            detail:
                "source and destination Runtime Homes must be separate non-overlapping locations"
                    .to_owned(),
        });
    }
    Ok(())
}

fn validate_destination_is_available(destination: &Path) -> StoreResult<()> {
    if !destination.exists() {
        return Ok(());
    }
    if !destination.is_dir() {
        return Err(destination_conflict(destination));
    }
    if fs::read_dir(destination)?.next().transpose()?.is_some() {
        return Err(destination_conflict(destination));
    }
    Ok(())
}

fn destination_conflict(destination: &Path) -> StoreError {
    StoreError::Conflict {
        entity: "destination_runtime_home",
        id: destination.display().to_string(),
        detail: "destination must be absent or an empty directory".to_owned(),
    }
}

fn staging_home_path(destination: &Path) -> StoreResult<PathBuf> {
    let name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| StoreError::InvalidInput {
            detail: "destination Runtime Home name must be valid UTF-8".to_owned(),
        })?;
    Ok(destination.with_file_name(format!(".{name}.volicord-v6-to-v7-incomplete")))
}

fn prepare_owned_staging_home(
    staging: &Path,
    source: &Path,
    destination: &Path,
) -> StoreResult<()> {
    if !staging.exists() {
        return Ok(());
    }
    if fs::symlink_metadata(staging)?.file_type().is_symlink() {
        return Err(staging_conflict(staging));
    }
    if !staging.is_dir() {
        return Err(staging_conflict(staging));
    }
    let source_text = path_text(source, "source_home")?;
    let destination_text = path_text(destination, "destination_home")?;
    let marker_path = staging.join(INCOMPLETE_MARKER_FILE);
    let report_path = staging.join(STORAGE_UPGRADE_REPORT_FILE);
    let owned = if marker_path.is_file() {
        let marker: IncompleteUpgradeMarker = read_json_file(&marker_path)?;
        marker.status == "incomplete"
            && marker.source_profile == STORAGE_UPGRADE_SOURCE_PROFILE
            && marker.destination_profile == STORAGE_PROFILE
            && marker.source_home == source_text
            && marker.destination_home == destination_text
    } else if report_path.is_file() {
        let report: StorageUpgradeReport = read_json_file(&report_path)?;
        report.status == "completed"
            && report.source_profile == STORAGE_UPGRADE_SOURCE_PROFILE
            && report.destination_profile == STORAGE_PROFILE
            && report.source_home == source_text
            && report.destination_home == destination_text
            && report.destination_ready
    } else {
        false
    };
    if !owned {
        return Err(staging_conflict(staging));
    }
    fs::remove_dir_all(staging)?;
    Ok(())
}

fn staging_conflict(staging: &Path) -> StoreError {
    StoreError::Conflict {
        entity: "storage_upgrade_staging",
        id: staging.display().to_string(),
        detail: "existing staging data is not an exact incomplete state owned by this conversion"
            .to_owned(),
    }
}

fn promote_staging_home(staging: &Path, destination: &Path) -> StoreResult<()> {
    if destination.exists() {
        fs::remove_dir(destination).map_err(|error| {
            if error.kind() == std::io::ErrorKind::DirectoryNotEmpty {
                destination_conflict(destination)
            } else {
                StoreError::Io(error)
            }
        })?;
    }
    fs::rename(staging, destination)?;
    Ok(())
}

fn path_text(path: &Path, field: &'static str) -> StoreResult<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| StoreError::InvalidInput {
            detail: format!("{field} path must be valid UTF-8 for the conversion report"),
        })
}

fn write_json_file(path: &Path, value: &impl Serialize) -> StoreResult<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| StoreError::InvalidInput {
        detail: format!("storage upgrade report cannot be serialized: {error}"),
    })?;
    fs::write(path, bytes)?;
    Ok(())
}

fn read_json_file<T: for<'de> Deserialize<'de>>(path: &Path) -> StoreResult<T> {
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(|_| upgrade_invariant("staging marker is malformed"))
}

fn upgrade_invariant(detail: impl Into<String>) -> StoreError {
    StoreError::SchemaInvariant {
        database_kind: DATABASE_KIND,
        detail: detail.into(),
    }
}

fn runtime_home_manifest(root: &Path) -> StoreResult<Vec<FileManifestEntry>> {
    let mut entries = Vec::new();
    collect_manifest_entries(root, root, &mut entries)?;
    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(entries)
}

fn collect_manifest_entries(
    root: &Path,
    directory: &Path,
    output: &mut Vec<FileManifestEntry>,
) -> StoreResult<()> {
    let mut children = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    children.sort_by_key(|entry| entry.file_name());
    for child in children {
        let path = child.path();
        let relative_path = path
            .strip_prefix(root)
            .map_err(|_| upgrade_invariant("source manifest path escaped its Runtime Home"))?
            .to_path_buf();
        let metadata = fs::symlink_metadata(&path)?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            output.push(FileManifestEntry {
                relative_path,
                kind: "symbolic_link",
                size_bytes: metadata.len(),
                sha256: None,
                link_target: Some(fs::read_link(&path)?),
            });
            continue;
        }
        if file_type.is_dir() {
            output.push(FileManifestEntry {
                relative_path,
                kind: "directory",
                size_bytes: 0,
                sha256: None,
                link_target: None,
            });
            collect_manifest_entries(root, &path, output)?;
        } else if file_type.is_file() {
            output.push(FileManifestEntry {
                relative_path,
                kind: "file",
                size_bytes: metadata.len(),
                sha256: Some(file_sha256(&path)?),
                link_target: None,
            });
        } else {
            return Err(upgrade_invariant(format!(
                "source Runtime Home contains unsupported special file {}",
                relative_path.display()
            )));
        }
    }
    Ok(())
}

fn file_sha256(path: &Path) -> StoreResult<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn validate_v6_registry(conn: &Connection, source_home: &Path) -> StoreResult<()> {
    validate_schema_source_hash(
        REGISTRY_SCHEMA_SQL,
        V6_REGISTRY_SCHEMA_SHA256,
        "v6 registry",
    )?;
    validate_exact_schema(conn, REGISTRY_SCHEMA_SQL, "v6 registry")?;
    validate_database_integrity(conn, "v6 registry")?;
    validate_json_columns(conn, REGISTRY_TABLES, false)?;

    let rows = conn.query_row("SELECT COUNT(*) FROM runtime_home", [], |row| {
        row.get::<_, i64>(0)
    })?;
    if rows != 1 {
        return Err(upgrade_invariant(
            "v6 registry must contain exactly one runtime_home row",
        ));
    }
    let (singleton_id, runtime_home_path, stored_registry_path, storage_profile): (
        i64,
        String,
        String,
        String,
    ) = conn.query_row(
        "SELECT singleton_id, runtime_home_path, registry_db_path, storage_profile
           FROM runtime_home",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    if singleton_id != 1 {
        return Err(upgrade_invariant(
            "v6 registry runtime_home singleton is malformed",
        ));
    }
    if storage_profile != STORAGE_UPGRADE_SOURCE_PROFILE {
        return Err(StoreError::UnsupportedStorageProfile {
            database_kind: "registry",
            actual_storage_profile: storage_profile,
            expected_storage_profile: STORAGE_UPGRADE_SOURCE_PROFILE,
        });
    }
    if !stored_path_resolves_to(&runtime_home_path, source_home)?
        || !stored_path_resolves_to(&stored_registry_path, &registry_db_path(source_home))?
    {
        return Err(upgrade_invariant(
            "v6 registry coordinates do not match the source Runtime Home",
        ));
    }
    Ok(())
}

fn read_source_projects(conn: &Connection, source_home: &Path) -> StoreResult<Vec<SourceProject>> {
    let mut statement = conn.prepare(
        "SELECT project_internal_id, repo_root, project_home, state_db_path
           FROM projects
          ORDER BY project_internal_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    let mut projects = Vec::new();
    for row in rows {
        let (project_id, repo_root, project_home, state_db_path) = row?;
        validate_project_path_component(&project_id)?;
        let repo_root = PathBuf::from(repo_root);
        if !repo_root.is_absolute() || !repo_root.is_dir() {
            return Err(upgrade_invariant(format!(
                "v6 project {project_id} has a missing or non-absolute Product Repository"
            )));
        }
        let repo_root = fs::canonicalize(repo_root)?;
        let expected_home = source_home.join(PROJECTS_DIR).join(&project_id);
        let expected_state = expected_home.join(PROJECT_STATE_DB_FILE);
        if !stored_path_resolves_to(&project_home, &expected_home)?
            || !stored_path_resolves_to(&state_db_path, &expected_state)?
        {
            return Err(upgrade_invariant(format!(
                "v6 project {project_id} has non-canonical source storage coordinates"
            )));
        }
        if !expected_home.is_dir() || !expected_state.is_file() {
            return Err(upgrade_invariant(format!(
                "v6 project {project_id} is missing its project home or state database"
            )));
        }
        projects.push(SourceProject {
            project_id,
            repo_root,
            source_project_home: expected_home,
            source_state_db_path: expected_state,
        });
    }
    Ok(projects)
}

fn converted_project_policy(
    registry: &Connection,
    source_home: &Path,
    project: &SourceProject,
) -> StoreResult<ConvertedProjectPolicy> {
    let policy_path = project.repo_root.join(LEGACY_PROJECT_POLICY_FILE);
    let metadata = fs::symlink_metadata(&policy_path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            upgrade_invariant(format!(
                "v6 project {} is missing its managed v1 policy",
                project.project_id
            ))
        } else {
            StoreError::Io(error)
        }
    })?;
    if !metadata.file_type().is_file() {
        return Err(upgrade_invariant(format!(
            "v6 project {} managed policy is not one regular file",
            project.project_id
        )));
    }
    if metadata.len() > MAX_PROJECT_POLICY_BYTES {
        return Err(upgrade_invariant(format!(
            "v6 project {} managed policy exceeds the size limit",
            project.project_id
        )));
    }
    let file = File::open(&policy_path)?;
    if !file.metadata()?.is_file() {
        return Err(upgrade_invariant(format!(
            "v6 project {} managed policy is not one regular file",
            project.project_id
        )));
    }
    let mut bytes = Vec::new();
    file.take(MAX_PROJECT_POLICY_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_PROJECT_POLICY_BYTES {
        return Err(upgrade_invariant(format!(
            "v6 project {} managed policy exceeds the size limit",
            project.project_id
        )));
    }
    let legacy = serde_json::from_slice::<Value>(&bytes).map_err(|_| {
        upgrade_invariant(format!(
            "v6 project {} managed policy contains malformed JSON",
            project.project_id
        ))
    })?;
    let converted = convert_v6_policy_value(legacy)?;
    validate_converted_policy_bindings(registry, source_home, project, &converted)?;
    let canonical_json = canonical_json(&converted, "project_workflow_policies", "policy_json")?;
    let fingerprint = canonical_json_sha256(&converted)
        .map_err(|_| upgrade_invariant("converted workflow policy could not be fingerprinted"))?
        .into_inner();
    Ok(ConvertedProjectPolicy {
        canonical_json,
        fingerprint,
    })
}

fn convert_v6_policy_value(mut policy: Value) -> StoreResult<Value> {
    validate_v1_policy_shape(&policy)?;
    let object = policy
        .as_object_mut()
        .expect("validated v1 policy is an object");
    object.insert(
        "schema".to_owned(),
        Value::String("volicord-policy-v2".to_owned()),
    );
    object.insert("workflow".to_owned(), conservative_workflow_policy_json());
    validate_converted_v2_policy_shape(&policy)?;
    Ok(policy)
}

#[cfg(feature = "test-support")]
#[doc(hidden)]
pub fn convert_v6_policy_value_for_test(policy: Value) -> StoreResult<Value> {
    convert_v6_policy_value(policy)
}

fn validate_v1_policy_shape(policy: &Value) -> StoreResult<()> {
    let object = policy.as_object().ok_or_else(|| {
        upgrade_invariant("v6 managed policy must contain exactly one JSON object")
    })?;
    validate_exact_policy_keys(object, V1_POLICY_TOP_LEVEL_KEYS, "top-level")?;
    for (field, expected) in [
        ("schema", "volicord-policy-v1"),
        ("managed_by", "volicord"),
        ("storage_scope", "local_overlay"),
    ] {
        if object.get(field).and_then(Value::as_str) != Some(expected) {
            return Err(upgrade_invariant(format!(
                "v6 managed policy requires {field}={expected}"
            )));
        }
    }
    if !matches!(
        object.get("connection_intent").and_then(Value::as_str),
        Some("personal" | "shared" | "global")
    ) {
        return Err(upgrade_invariant(
            "v6 managed policy has an unsupported connection intent",
        ));
    }
    for field in [
        "host",
        "repo_root",
        "connection_id",
        "guard_installation_id",
        "selected_profile",
    ] {
        if object
            .get(field)
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err(upgrade_invariant(format!(
                "v6 managed policy requires a non-empty {field} string"
            )));
        }
    }
    let profile = object
        .get("selected_profile")
        .and_then(Value::as_str)
        .expect("selected profile was validated as a string");
    if !matches!(profile, "record" | "detective") {
        return Err(upgrade_invariant(
            "v6 managed policy has an unsupported selected profile",
        ));
    }

    let mcp = policy_object_field(object, "mcp", "mcp")?;
    validate_exact_policy_keys(mcp, POLICY_MCP_KEYS, "mcp")?;
    if mcp
        .get("command")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
        || !mcp
            .get("args")
            .and_then(Value::as_array)
            .is_some_and(|args| args.iter().all(Value::is_string))
    {
        return Err(upgrade_invariant(
            "v6 managed policy requires an MCP command and string arguments",
        ));
    }
    let env = policy_object_field(mcp, "env", "mcp.env")?;
    for (key, value) in env {
        if !POLICY_MCP_ENV_KEYS.contains(&key.as_str()) || !value.is_string() {
            return Err(upgrade_invariant(format!(
                "v6 managed policy has an unsupported MCP environment entry {key}"
            )));
        }
    }

    let host_hook = policy_object_field(object, "host_hook", "host_hook")?;
    validate_exact_policy_keys(host_hook, POLICY_HOST_HOOK_KEYS, "host_hook")?;
    let enabled = host_hook
        .get("enabled")
        .and_then(Value::as_bool)
        .ok_or_else(|| upgrade_invariant("v6 managed policy host_hook.enabled must be a bool"))?;
    if enabled != (profile == "detective") {
        return Err(upgrade_invariant(
            "v6 managed policy hook enablement does not match its selected profile",
        ));
    }
    let commands = policy_object_field(host_hook, "commands", "host_hook.commands")?;
    validate_exact_policy_keys(commands, POLICY_HOOK_PHASE_KEYS, "host_hook.commands")?;
    for (phase, command) in commands {
        let command = command.as_object().ok_or_else(|| {
            upgrade_invariant(format!(
                "v6 managed policy hook command {phase} must be an object"
            ))
        })?;
        validate_exact_policy_keys(command, POLICY_HOOK_COMMAND_KEYS, "host hook command")?;
        if command
            .get("command")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
            || !command
                .get("args")
                .and_then(Value::as_array)
                .is_some_and(|args| args.iter().all(Value::is_string))
        {
            return Err(upgrade_invariant(format!(
                "v6 managed policy hook command {phase} is malformed"
            )));
        }
    }
    Ok(())
}

fn validate_converted_v2_policy_shape(policy: &Value) -> StoreResult<()> {
    let object = policy.as_object().ok_or_else(|| {
        upgrade_invariant("converted project policy must contain exactly one JSON object")
    })?;
    if object.len() != V1_POLICY_TOP_LEVEL_KEYS.len() + 1
        || object
            .keys()
            .any(|key| key != "workflow" && !V1_POLICY_TOP_LEVEL_KEYS.contains(&key.as_str()))
        || !object.contains_key("workflow")
    {
        return Err(upgrade_invariant(
            "converted project policy fields do not match the exact v2 shape",
        ));
    }
    if object.get("schema").and_then(Value::as_str) != Some("volicord-policy-v2")
        || object.get("workflow") != Some(&conservative_workflow_policy_json())
    {
        return Err(upgrade_invariant(
            "converted project policy is not the conservative v2 policy",
        ));
    }
    let mut legacy = policy.clone();
    let legacy = legacy
        .as_object_mut()
        .expect("converted project policy was validated as an object");
    legacy.remove("workflow");
    legacy.insert(
        "schema".to_owned(),
        Value::String("volicord-policy-v1".to_owned()),
    );
    validate_v1_policy_shape(&Value::Object(legacy.clone()))
}

fn policy_object_field<'a>(
    object: &'a serde_json::Map<String, Value>,
    field: &str,
    label: &str,
) -> StoreResult<&'a serde_json::Map<String, Value>> {
    object
        .get(field)
        .and_then(Value::as_object)
        .ok_or_else(|| upgrade_invariant(format!("v6 managed policy {label} must be an object")))
}

fn validate_exact_policy_keys(
    object: &serde_json::Map<String, Value>,
    expected: &[&str],
    label: &str,
) -> StoreResult<()> {
    if object.len() != expected.len()
        || object.keys().any(|key| !expected.contains(&key.as_str()))
        || expected.iter().any(|key| !object.contains_key(*key))
    {
        return Err(upgrade_invariant(format!(
            "v6 managed policy {label} fields do not match the exact v1 shape"
        )));
    }
    Ok(())
}

fn validate_converted_policy_bindings(
    registry: &Connection,
    source_home: &Path,
    project: &SourceProject,
    policy: &Value,
) -> StoreResult<()> {
    let policy_repo = policy["repo_root"]
        .as_str()
        .expect("converted policy retains a validated repo_root");
    if fs::canonicalize(policy_repo).ok().as_deref() != Some(project.repo_root.as_path()) {
        return Err(upgrade_invariant(format!(
            "v6 project {} managed policy repository binding is stale",
            project.project_id
        )));
    }
    let connection_id = policy["connection_id"]
        .as_str()
        .expect("converted policy retains a validated connection_id");
    let connection = registry
        .query_row(
            "SELECT host_kind, intent, enabled
               FROM agent_connections
              WHERE connection_internal_id = ?1",
            [connection_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((host_kind, intent, enabled)) = connection else {
        return Err(upgrade_invariant(format!(
            "v6 project {} managed policy connection is not recorded",
            project.project_id
        )));
    };
    let project_allowed = registry.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM connection_projects
              WHERE connection_internal_id = ?1 AND project_internal_id = ?2
         )",
        params![connection_id, project.project_id],
        |row| row.get::<_, i64>(0),
    )? == 1;
    let public_host = if host_kind == "claude_code" {
        "claude-code"
    } else {
        host_kind.as_str()
    };
    if enabled != 1
        || !project_allowed
        || policy["host"].as_str() != Some(public_host)
        || policy["connection_intent"].as_str() != Some(intent.as_str())
    {
        return Err(upgrade_invariant(format!(
            "v6 project {} managed policy connection binding is stale",
            project.project_id
        )));
    }

    let guard_id = policy["guard_installation_id"]
        .as_str()
        .expect("converted policy retains a validated guard_installation_id");
    let guard = registry
        .query_row(
            "SELECT connection_internal_id, project_internal_id, host_kind, guard_mode
               FROM guard_installations
              WHERE guard_installation_id = ?1",
            [guard_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;
    let Some((guard_connection, guard_project, guard_host, guard_mode)) = guard else {
        return Err(upgrade_invariant(format!(
            "v6 project {} managed policy guard installation is not recorded",
            project.project_id
        )));
    };
    let public_guard_host = if guard_host == "claude_code" {
        "claude-code"
    } else {
        guard_host.as_str()
    };
    if guard_connection != connection_id
        || guard_project.as_deref() != Some(project.project_id.as_str())
        || policy["host"].as_str() != Some(public_guard_host)
        || policy["selected_profile"].as_str() != Some(guard_mode.as_str())
    {
        return Err(upgrade_invariant(format!(
            "v6 project {} managed policy guard binding is stale",
            project.project_id
        )));
    }

    let mcp_env = policy["mcp"]["env"]
        .as_object()
        .expect("converted policy retains a validated MCP environment");
    if let Some(runtime_home) = mcp_env.get("VOLICORD_HOME").and_then(Value::as_str) {
        let portable_claude_forwarding = policy["connection_intent"].as_str() == Some("shared")
            && host_kind == "claude_code"
            && runtime_home == "${VOLICORD_HOME}";
        if !portable_claude_forwarding && !stored_path_resolves_to(runtime_home, source_home)? {
            return Err(upgrade_invariant(format!(
                "v6 project {} managed policy MCP Runtime Home binding is stale",
                project.project_id
            )));
        }
    }
    for (key, expected) in [
        ("VOLICORD_MCP_HOST", host_kind.as_str()),
        ("VOLICORD_MCP_CONNECTION_ID", connection_id),
        ("VOLICORD_MCP_PROJECT_ID", project.project_id.as_str()),
        ("VOLICORD_MCP_LAUNCH", "managed_host"),
    ] {
        if mcp_env
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|actual| actual != expected)
        {
            return Err(upgrade_invariant(format!(
                "v6 project {} managed policy MCP {key} binding is stale",
                project.project_id
            )));
        }
    }
    Ok(())
}

fn conservative_workflow_policy_json() -> Value {
    json!({
        "default_direct_control": "tracked",
        "default_work_control": "tracked",
        "light": {
            "enabled": false,
            "max_intended_paths": 3,
            "allowed_path_patterns": [],
            "denied_path_patterns": [],
            "final_acceptance": "policy_dependent"
        },
        "write_ticket": {"idle_timeout_minutes": Value::Null},
        "detective": {
            "unknown_effect_behavior": "warn",
            "stop_behavior": "allow_with_disclosure"
        }
    })
}

fn detective_guard_installation_ids(conn: &Connection) -> StoreResult<BTreeSet<String>> {
    let mut statement = conn.prepare(
        "SELECT guard_installation_id
           FROM guard_installations
          WHERE guard_mode = 'detective'
          ORDER BY guard_installation_id",
    )?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect::<Result<BTreeSet<_>, _>>().map_err(Into::into)
}

fn stored_path_resolves_to(stored: &str, expected: &Path) -> StoreResult<bool> {
    let stored = Path::new(stored);
    if !stored.is_absolute() {
        return Ok(false);
    }
    match fs::canonicalize(stored) {
        Ok(resolved) => Ok(resolved == expected),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn validate_project_path_component(project_id: &str) -> StoreResult<()> {
    let path = Path::new(project_id);
    let mut components = path.components();
    if project_id.is_empty()
        || !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(upgrade_invariant(
            "v6 project identifier is not one normal path component",
        ));
    }
    Ok(())
}

fn validate_source_database_inventory(
    source_home: &Path,
    projects: &[SourceProject],
    manifest: &[FileManifestEntry],
) -> StoreResult<()> {
    let registry = registry_db_path(source_home);
    reject_sqlite_sidecars(&registry)?;
    let diagnostics = source_home.join(DIAGNOSTICS_DB_FILE);
    if diagnostics.exists() {
        if !diagnostics.is_file() {
            return Err(upgrade_invariant(
                "v6 diagnostics database path is not a regular file",
            ));
        }
        reject_sqlite_sidecars(&diagnostics)?;
    }
    for project in projects {
        reject_sqlite_sidecars(&project.source_state_db_path)?;
    }

    let artifact_roots: Vec<_> = projects
        .iter()
        .map(|project| {
            PathBuf::from(PROJECTS_DIR)
                .join(&project.project_id)
                .join(ARTIFACTS_DIR)
        })
        .collect();
    for entry in manifest
        .iter()
        .filter(|entry| entry.kind == "symbolic_link")
    {
        if !artifact_roots.iter().any(|root| {
            entry.relative_path.starts_with(root) && entry.relative_path.as_path() != root
        }) {
            return Err(upgrade_invariant(format!(
                "source symbolic link {} is outside a project artifact store",
                entry.relative_path.display()
            )));
        }
    }

    let registered: BTreeSet<_> = projects
        .iter()
        .map(|project| project.source_project_home.clone())
        .collect();
    let projects_root = source_home.join(PROJECTS_DIR);
    if projects_root.exists() {
        for entry in fs::read_dir(&projects_root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir()
                && path.join(PROJECT_STATE_DB_FILE).exists()
                && !registered.contains(&path)
            {
                return Err(upgrade_invariant(format!(
                    "unregistered v6 project database found at {}",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}

fn reject_sqlite_sidecars(database_path: &Path) -> StoreResult<()> {
    let text = database_path
        .to_str()
        .ok_or_else(|| upgrade_invariant("database path is not valid UTF-8"))?;
    for suffix in ["-wal", "-shm", "-journal"] {
        let sidecar = PathBuf::from(format!("{text}{suffix}"));
        if sidecar.exists() {
            return Err(upgrade_invariant(format!(
                "offline conversion requires a checkpointed database without {}",
                sidecar.display()
            )));
        }
    }
    Ok(())
}

fn v6_project_schema_sql() -> StoreResult<String> {
    let without_controls = PROJECT_STATE_SCHEMA_SQL.replacen(V7_TASK_CONTROL_COLUMNS, "", 1);
    if without_controls.len() == PROJECT_STATE_SCHEMA_SQL.len() {
        return Err(upgrade_invariant(
            "current project schema no longer exposes the expected v7 control columns",
        ));
    }
    let with_v6_tickets =
        without_controls.replacen(V7_WRITE_TICKETS_SCHEMA, V6_WRITE_TICKETS_SCHEMA, 1);
    if with_v6_tickets.len() == without_controls.len() {
        return Err(upgrade_invariant(
            "current project schema no longer exposes the expected v7 write-ticket table",
        ));
    }
    let without_confidence = with_v6_tickets.replacen(V7_UNRECORDED_CONFIDENCE_COLUMN, "", 1);
    if without_confidence.len() == with_v6_tickets.len() {
        return Err(upgrade_invariant(
            "current project schema no longer exposes the expected v7 confidence column",
        ));
    }
    let with_v6_authority_task = without_confidence.replacen(
        V7_AUTHORITY_EVENT_TASK_ID_COLUMN,
        V6_AUTHORITY_EVENT_TASK_ID_COLUMN,
        1,
    );
    if with_v6_authority_task.len() == without_confidence.len() {
        return Err(upgrade_invariant(
            "current project schema no longer exposes the expected v7 authority-event task column",
        ));
    }
    let without_project_event_check =
        with_v6_authority_task.replacen(V7_PROJECT_SCOPED_AUTHORITY_EVENT_CHECK, "", 1);
    if without_project_event_check.len() == with_v6_authority_task.len() {
        return Err(upgrade_invariant(
            "current project schema no longer exposes the expected v7 project-event constraint",
        ));
    }
    let with_v6_task_events =
        without_project_event_check.replacen(V7_TASK_EVENTS_FILTER, V6_TASK_EVENTS_FILTER, 1);
    if with_v6_task_events.len() == without_project_event_check.len() {
        return Err(upgrade_invariant(
            "current project schema no longer exposes the expected v7 task-events filter",
        ));
    }
    let project_only_start = with_v6_task_events
        .find(V7_PROJECT_ONLY_SCHEMA_START)
        .ok_or_else(|| {
            upgrade_invariant("current project schema no longer has the expected v7-only suffix")
        })?;
    let legacy = with_v6_task_events[..project_only_start].to_owned();
    validate_schema_source_hash(&legacy, V6_PROJECT_SCHEMA_SHA256, "v6 project")?;
    Ok(legacy)
}

fn validate_schema_source_hash(sql: &str, expected_sha256: &str, label: &str) -> StoreResult<()> {
    let actual = format!("{:x}", Sha256::digest(sql.as_bytes()));
    if actual != expected_sha256 {
        return Err(upgrade_invariant(format!(
            "{label} canonical DDL hash {actual} does not match its maintained owner value"
        )));
    }
    Ok(())
}

fn validate_exact_schema(conn: &Connection, canonical_sql: &str, label: &str) -> StoreResult<()> {
    let expected = canonical_schema_inventory(canonical_sql)?;
    let actual = read_schema_inventory(conn)?;
    if actual != expected {
        return Err(upgrade_invariant(format!(
            "{label} schema inventory is incomplete or non-canonical"
        )));
    }
    Ok(())
}

fn canonical_schema_inventory(sql: &str) -> StoreResult<Vec<SchemaObject>> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(sql)?;
    read_schema_inventory(&conn)
}

fn read_schema_inventory(conn: &Connection) -> StoreResult<Vec<SchemaObject>> {
    let mut statement = conn.prepare(
        "SELECT type, name, tbl_name, sql
           FROM sqlite_master
          WHERE type IN ('table', 'index', 'view', 'trigger')
            AND name NOT LIKE 'sqlite_%'
          ORDER BY type, name",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(SchemaObject {
            object_type: row.get(0)?,
            name: row.get(1)?,
            table_name: row.get(2)?,
            definition: row.get(3)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn validate_database_integrity(conn: &Connection, label: &str) -> StoreResult<()> {
    let mut quick_check = conn.prepare("PRAGMA quick_check")?;
    let rows = quick_check.query_map([], |row| row.get::<_, String>(0))?;
    for result in rows {
        if result? != "ok" {
            return Err(upgrade_invariant(format!(
                "{label} failed SQLite quick_check"
            )));
        }
    }
    let mut foreign_key_check = conn.prepare("PRAGMA foreign_key_check")?;
    if foreign_key_check.exists([])? {
        return Err(upgrade_invariant(format!(
            "{label} failed SQLite foreign_key_check"
        )));
    }
    Ok(())
}

type RowValues = BTreeMap<String, SqlValue>;

fn table_columns(conn: &Connection, table: &str) -> StoreResult<Vec<String>> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({})", quoted(table)))?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut columns = rows.collect::<Result<Vec<_>, _>>()?;
    columns.sort_by_key(|(position, _)| *position);
    Ok(columns.into_iter().map(|(_, name)| name).collect())
}

fn primary_key_columns(conn: &Connection, table: &str) -> StoreResult<Vec<String>> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({})", quoted(table)))?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, i64>(5)?, row.get::<_, String>(1)?))
    })?;
    let mut columns = rows
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|(position, _)| *position > 0)
        .collect::<Vec<_>>();
    columns.sort_by_key(|(position, _)| *position);
    Ok(columns.into_iter().map(|(_, name)| name).collect())
}

fn read_table_rows(conn: &Connection, table: &str) -> StoreResult<Vec<RowValues>> {
    let columns = table_columns(conn, table)?;
    let primary_key = primary_key_columns(conn, table)?;
    let select_columns = columns
        .iter()
        .map(|column| quoted(column))
        .collect::<Vec<_>>()
        .join(", ");
    let order_clause = if primary_key.is_empty() {
        String::new()
    } else {
        format!(
            " ORDER BY {}",
            primary_key
                .iter()
                .map(|column| quoted(column))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let mut statement = conn.prepare(&format!(
        "SELECT {select_columns} FROM {}{order_clause}",
        quoted(table)
    ))?;
    let rows = statement.query_map([], |row| {
        let mut values = RowValues::new();
        for (index, column) in columns.iter().enumerate() {
            values.insert(column.clone(), row.get::<_, SqlValue>(index)?);
        }
        Ok(values)
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn copy_table_rows(
    source: &Connection,
    destination: &Transaction<'_>,
    table: &str,
    mut transform: impl FnMut(RowValues) -> StoreResult<RowValues>,
) -> StoreResult<u64> {
    let destination_columns = table_columns(destination, table)?;
    let placeholders = (1..=destination_columns.len())
        .map(|position| format!("?{position}"))
        .collect::<Vec<_>>()
        .join(", ");
    let insert_columns = destination_columns
        .iter()
        .map(|column| quoted(column))
        .collect::<Vec<_>>()
        .join(", ");
    let insert_sql = format!(
        "INSERT INTO {} ({insert_columns}) VALUES ({placeholders})",
        quoted(table)
    );
    let rows = read_table_rows(source, table)?;
    let count = u64::try_from(rows.len())
        .map_err(|_| upgrade_invariant("table row count exceeds the report value range"))?;
    for row in rows {
        let transformed = transform(row)?;
        let values = destination_columns
            .iter()
            .map(|column| {
                transformed.get(column).ok_or_else(|| {
                    upgrade_invariant(format!(
                        "conversion did not supply destination column {table}.{column}"
                    ))
                })
            })
            .collect::<StoreResult<Vec<_>>>()?;
        destination.execute(&insert_sql, params_from_iter(values))?;
    }
    Ok(count)
}

fn quoted(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn set_text(row: &mut RowValues, column: &str, value: impl Into<String>) {
    row.insert(column.to_owned(), SqlValue::Text(value.into()));
}

fn set_optional_text(row: &mut RowValues, column: &str, value: Option<String>) {
    row.insert(
        column.to_owned(),
        value.map_or(SqlValue::Null, SqlValue::Text),
    );
}

fn row_text(row: &RowValues, table: &str, column: &str) -> StoreResult<String> {
    match row.get(column) {
        Some(SqlValue::Text(value)) => Ok(value.clone()),
        _ => Err(upgrade_invariant(format!(
            "{table}.{column} is not stored as required text"
        ))),
    }
}

fn row_optional_text(row: &RowValues, table: &str, column: &str) -> StoreResult<Option<String>> {
    match row.get(column) {
        Some(SqlValue::Null) => Ok(None),
        Some(SqlValue::Text(value)) => Ok(Some(value.clone())),
        _ => Err(upgrade_invariant(format!(
            "{table}.{column} is not stored as nullable text"
        ))),
    }
}

fn row_i64(row: &RowValues, table: &str, column: &str) -> StoreResult<i64> {
    match row.get(column) {
        Some(SqlValue::Integer(value)) => Ok(*value),
        _ => Err(upgrade_invariant(format!(
            "{table}.{column} is not stored as an integer"
        ))),
    }
}

fn add_preserved_count(
    counts: &mut BTreeMap<String, u64>,
    key: String,
    count: u64,
) -> StoreResult<()> {
    let next = counts
        .get(&key)
        .copied()
        .unwrap_or(0)
        .checked_add(count)
        .ok_or_else(|| upgrade_invariant("preserved record count overflow"))?;
    counts.insert(key, next);
    Ok(())
}

fn validate_json_columns(
    conn: &Connection,
    tables: &[&str],
    require_canonical: bool,
) -> StoreResult<u64> {
    let mut check_count = 0u64;
    for table in tables {
        for column in table_columns(conn, table)?
            .into_iter()
            .filter(|column| column.ends_with("_json"))
        {
            let sql = format!(
                "SELECT {} FROM {} WHERE {} IS NOT NULL",
                quoted(&column),
                quoted(table),
                quoted(&column)
            );
            let mut statement = conn.prepare(&sql)?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            for raw in rows {
                let raw = raw?;
                let value: Value = serde_json::from_str(&raw).map_err(|_| {
                    upgrade_invariant(format!("{table}.{column} contains malformed JSON"))
                })?;
                let canonical = canonical_json(&value, table, &column)?;
                if require_canonical && canonical != raw {
                    return Err(upgrade_invariant(format!(
                        "{table}.{column} is not stored in canonical JSON form"
                    )));
                }
                check_count = check_count
                    .checked_add(1)
                    .ok_or_else(|| upgrade_invariant("canonical JSON check count overflow"))?;
            }
        }
    }
    Ok(check_count)
}

fn canonical_json(value: &Value, table: &str, column: &str) -> StoreResult<String> {
    canonical_json_string(value).map_err(|_| {
        upgrade_invariant(format!(
            "{table}.{column} could not be represented as canonical JSON"
        ))
    })
}

fn copy_registry(
    source: &Connection,
    staging_home: &Path,
    destination_home: &Path,
    preserved_counts: &mut BTreeMap<String, u64>,
) -> StoreResult<()> {
    let destination_registry_path = registry_db_path(staging_home);
    let final_registry_path = registry_db_path(destination_home);
    let mut destination = open_registry_database(&destination_registry_path)?;
    let transaction = destination.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.pragma_update(None, "defer_foreign_keys", "ON")?;

    for table in REGISTRY_TABLES {
        let count = copy_table_rows(source, &transaction, table, |mut row| {
            match *table {
                "runtime_home" => {
                    set_text(
                        &mut row,
                        "runtime_home_path",
                        path_text(destination_home, "destination_home")?,
                    );
                    set_text(
                        &mut row,
                        "registry_db_path",
                        path_text(&final_registry_path, "destination_registry")?,
                    );
                    set_text(&mut row, "storage_profile", STORAGE_PROFILE);
                }
                "projects" => {
                    let project_id = row_text(&row, "projects", "project_internal_id")?;
                    validate_project_path_component(&project_id)?;
                    let project_home = destination_home.join(PROJECTS_DIR).join(&project_id);
                    set_text(
                        &mut row,
                        "project_home",
                        path_text(&project_home, "destination_project_home")?,
                    );
                    set_text(
                        &mut row,
                        "state_db_path",
                        path_text(
                            &project_home.join(PROJECT_STATE_DB_FILE),
                            "destination_state_database",
                        )?,
                    );
                }
                _ => {}
            }
            Ok(row)
        })?;
        add_preserved_count(preserved_counts, format!("registry.{table}"), count)?;
    }
    transaction.commit()?;
    validate_registry_schema(&destination)?;
    validate_database_integrity(&destination, "destination registry")?;
    verify_preserved_registry_records(source, &destination)?;
    Ok(())
}

fn verify_preserved_registry_records(
    source: &Connection,
    destination: &Connection,
) -> StoreResult<()> {
    for table in REGISTRY_TABLES {
        let source_rows = read_table_rows(source, table)?;
        let destination_rows = read_table_rows(destination, table)?;
        if source_rows.len() != destination_rows.len() {
            return Err(upgrade_invariant(format!(
                "destination row count for registry table {table} does not match v6 source"
            )));
        }
        let primary_key = primary_key_columns(source, table)?;
        for (source_row, destination_row) in source_rows.iter().zip(&destination_rows) {
            for column in &primary_key {
                if source_row.get(column) != destination_row.get(column) {
                    return Err(upgrade_invariant(format!(
                        "destination primary identifiers for registry table {table} changed"
                    )));
                }
            }
            for column in table_columns(source, table)? {
                if intentionally_transformed_registry_column(table, &column) {
                    continue;
                }
                if source_row.get(&column) != destination_row.get(&column) {
                    return Err(upgrade_invariant(format!(
                        "destination changed preserved registry field {table}.{column}"
                    )));
                }
            }
        }
    }
    Ok(())
}

fn intentionally_transformed_registry_column(table: &str, column: &str) -> bool {
    matches!(
        (table, column),
        (
            "runtime_home",
            "runtime_home_path" | "registry_db_path" | "storage_profile"
        ) | ("projects", "project_home" | "state_db_path")
    )
}

fn validate_v6_project_state(conn: &Connection, project_id: &str) -> StoreResult<()> {
    let v6_sql = v6_project_schema_sql()?;
    validate_exact_schema(conn, &v6_sql, "v6 project state")?;
    validate_database_integrity(conn, "v6 project state")?;
    validate_json_columns(conn, PROJECT_V6_TABLES, false)?;

    let state_rows = conn.query_row("SELECT COUNT(*) FROM project_state", [], |row| {
        row.get::<_, i64>(0)
    })?;
    if state_rows != 1 {
        return Err(upgrade_invariant(
            "v6 project database must contain exactly one project_state row",
        ));
    }
    let (stored_project_id, storage_profile): (String, String) = conn.query_row(
        "SELECT project_id, storage_profile FROM project_state",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if stored_project_id != project_id {
        return Err(upgrade_invariant(format!(
            "v6 project state identity does not match registry project {project_id}"
        )));
    }
    if storage_profile != STORAGE_UPGRADE_SOURCE_PROFILE {
        return Err(StoreError::UnsupportedStorageProfile {
            database_kind: "project_state",
            actual_storage_profile: storage_profile,
            expected_storage_profile: STORAGE_UPGRADE_SOURCE_PROFILE,
        });
    }
    for table in PROJECT_V6_TABLES {
        if table_columns(conn, table)?
            .iter()
            .any(|column| column == "project_id")
        {
            let sql = format!(
                "SELECT COUNT(*) FROM {} WHERE project_id != ?1",
                quoted(table)
            );
            let mismatches = conn.query_row(&sql, [project_id], |row| row.get::<_, i64>(0))?;
            if mismatches != 0 {
                return Err(upgrade_invariant(format!(
                    "v6 table {table} contains a foreign project identity"
                )));
            }
        }
    }
    crate::core_pipeline::validation::validate_v6_typed_owner_state_for_conversion(
        conn, project_id,
    )?;
    crate::evidence_capture::validate_v6_typed_owner_state_for_conversion(conn, project_id)?;
    for row in read_table_rows(conn, "write_tickets")? {
        let attempt_scope_json = row_text(&row, "write_tickets", "attempt_scope_json")?;
        let scope: WriteTicketAttemptScope =
            serde_json::from_str(&attempt_scope_json).map_err(|_| {
                upgrade_invariant("v6 write ticket has malformed typed attempt_scope_json")
            })?;
        let task_id = row_text(&row, "write_tickets", "task_id")?;
        let change_unit_id = row_optional_text(&row, "write_tickets", "change_unit_id")?;
        if scope.task_id.as_str() != task_id
            || change_unit_id.as_deref() != Some(scope.change_unit_id.as_str())
        {
            return Err(upgrade_invariant(
                "v6 write ticket attempt scope does not match its durable task coordinates",
            ));
        }
    }
    validate_authority_event_hash_chain(conn)?;
    Ok(())
}

fn copy_project_state(
    source: &Connection,
    destination: &mut Connection,
    project_id: &str,
    converted_at: &str,
    detective_guard_installations: &BTreeSet<String>,
    converted_policy: &ConvertedProjectPolicy,
    preserved_counts: &mut BTreeMap<String, u64>,
) -> StoreResult<()> {
    let detective_sessions = detective_session_ids(source)?;
    let confirmed_unrecorded_changes = confirmed_unrecorded_change_ids(source)?;
    let transaction = destination.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.pragma_update(None, "defer_foreign_keys", "ON")?;

    let count = copy_table_rows(source, &transaction, "project_state", |mut row| {
        set_text(&mut row, "storage_profile", STORAGE_PROFILE);
        Ok(row)
    })?;
    add_preserved_count(preserved_counts, "project.project_state".to_owned(), count)?;

    let count = copy_table_rows(source, &transaction, "tasks", |mut row| {
        let mode = row_text(&row, "tasks", "mode")?;
        let (requested, effective, reason) = match mode.as_str() {
            "advisor" => (
                "observe",
                "observe",
                "offline_v6_to_v7_advisor_maps_to_observe",
            ),
            "direct" | "work" => (
                "tracked",
                "tracked",
                "offline_v6_to_v7_mutating_mode_maps_to_tracked",
            ),
            _ => {
                return Err(upgrade_invariant(
                    "v6 task mode is outside the owner-defined value set",
                ))
            }
        };
        set_text(&mut row, "requested_control_level", requested);
        set_text(&mut row, "effective_control_level", effective);
        set_text(&mut row, "control_level_reason", reason);
        Ok(row)
    })?;
    add_preserved_count(preserved_counts, "project.tasks".to_owned(), count)?;

    for table in PROJECT_GENERIC_TABLES {
        let count = copy_table_rows(source, &transaction, table, Ok)?;
        add_preserved_count(preserved_counts, format!("project.{table}"), count)?;
    }

    let count = copy_table_rows(source, &transaction, "write_tickets", |mut row| {
        transform_write_ticket(source, &mut row, converted_at)?;
        Ok(row)
    })?;
    add_preserved_count(preserved_counts, "project.write_tickets".to_owned(), count)?;

    let count = copy_table_rows(source, &transaction, "guard_events", |mut row| {
        let session_id = row_optional_text(&row, "guard_events", "session_id")?;
        let guard_installation_id =
            row_optional_text(&row, "guard_events", "guard_installation_id")?;
        if session_id
            .as_ref()
            .is_some_and(|session_id| detective_sessions.contains(session_id))
            || guard_installation_id
                .as_ref()
                .is_some_and(|installation_id| {
                    detective_guard_installations.contains(installation_id)
                })
        {
            let raw = row_text(&row, "guard_events", "result_json")?;
            set_text(
                &mut row,
                "result_json",
                transform_detective_guard_result(&raw)?,
            );
        }
        Ok(row)
    })?;
    add_preserved_count(preserved_counts, "project.guard_events".to_owned(), count)?;

    let count = copy_table_rows(source, &transaction, "unrecorded_changes", |mut row| {
        let id = row_text(&row, "unrecorded_changes", "unrecorded_change_id")?;
        let confidence = if confirmed_unrecorded_changes.contains(&id) {
            "confirmed"
        } else {
            "suspected"
        };
        set_text(&mut row, "confidence", confidence);
        Ok(row)
    })?;
    add_preserved_count(
        preserved_counts,
        "project.unrecorded_changes".to_owned(),
        count,
    )?;

    insert_conservative_project_policy(&transaction, project_id, converted_at, converted_policy)?;
    transaction.commit()?;
    validate_database_integrity(destination, "destination project state")?;
    Ok(())
}

fn detective_session_ids(conn: &Connection) -> StoreResult<BTreeSet<String>> {
    let mut statement = conn.prepare(
        "SELECT session_id FROM agent_sessions WHERE guard_mode = 'detective' ORDER BY session_id",
    )?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect::<Result<BTreeSet<_>, _>>().map_err(Into::into)
}

fn confirmed_unrecorded_change_ids(conn: &Connection) -> StoreResult<BTreeSet<String>> {
    let mut statement = conn.prepare(
        "SELECT unrecorded_change_id, observed_paths_json
           FROM session_watch_observations
          WHERE observation_status = 'linked'
            AND unrecorded_change_id IS NOT NULL
          ORDER BY unrecorded_change_id, watch_observation_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut confirmed = BTreeSet::new();
    for row in rows {
        let (unrecorded_change_id, observed_paths_json) = row?;
        let observed_paths: Vec<String> =
            serde_json::from_str(&observed_paths_json).map_err(|_| {
                upgrade_invariant("v6 linked watch observation has malformed observed_paths_json")
            })?;
        if !observed_paths.is_empty() && observed_paths.iter().all(|path| !path.trim().is_empty()) {
            confirmed.insert(unrecorded_change_id);
        }
    }
    Ok(confirmed)
}

fn transform_write_ticket(
    source: &Connection,
    row: &mut RowValues,
    converted_at: &str,
) -> StoreResult<()> {
    let task_id = row_text(row, "write_tickets", "task_id")?;
    let change_unit_id = row_optional_text(row, "write_tickets", "change_unit_id")?;
    let raw_scope = row_text(row, "write_tickets", "attempt_scope_json")?;
    let scope: WriteTicketAttemptScope = serde_json::from_str(&raw_scope)
        .map_err(|_| upgrade_invariant("v6 write ticket attempt scope is malformed"))?;
    if scope.task_id.as_str() != task_id
        || change_unit_id.as_deref() != Some(scope.change_unit_id.as_str())
    {
        return Err(upgrade_invariant(
            "v6 write ticket coordinates do not match its attempt scope",
        ));
    }
    let scope_revision: i64 = source
        .query_row(
            "SELECT scope_revision FROM tasks WHERE task_id = ?1",
            [&task_id],
            |query_row| query_row.get(0),
        )
        .optional()?
        .ok_or_else(|| upgrade_invariant("v6 write ticket task is missing"))?;
    let scope_revision = u64::try_from(scope_revision)
        .map_err(|_| upgrade_invariant("v6 task scope revision is outside the supported range"))?;
    let validity_basis = json!({
        "task_id": task_id,
        "change_unit_id": scope.change_unit_id.as_str(),
        "scope_revision": scope_revision,
        "baseline_ref": scope.baseline_ref.as_ref().map(|value| value.as_str()),
        "workspace_context_sha256": Value::Null,
        "approval_basis_refs": []
    });
    set_text(
        row,
        "validity_basis_json",
        canonical_json(&validity_basis, "write_tickets", "validity_basis_json")?,
    );
    let allowed_paths = serde_json::to_value(&scope.intended_paths).map_err(|_| {
        upgrade_invariant("v6 write ticket intended paths cannot be converted to JSON")
    })?;
    set_text(
        row,
        "allowed_path_prefixes_json",
        canonical_json(
            &allowed_paths,
            "write_tickets",
            "allowed_path_prefixes_json",
        )?,
    );
    set_text(row, "denied_path_prefixes_json", "[]");

    let old_status = row_text(row, "write_tickets", "status")?;
    let old_expires_at = row_text(row, "write_tickets", "expires_at")?;
    set_optional_text(row, "idle_expires_at", None);
    let raw_metadata = row_text(row, "write_tickets", "metadata_json")?;
    let mut metadata: Value = serde_json::from_str(&raw_metadata)
        .map_err(|_| upgrade_invariant("v6 write ticket metadata contains malformed JSON"))?;
    let metadata = metadata
        .as_object_mut()
        .ok_or_else(|| upgrade_invariant("v6 write ticket metadata must be a JSON object"))?;
    if metadata.contains_key("offline_v6_to_v7_conversion") {
        return Err(upgrade_invariant(
            "v6 write ticket metadata collides with conversion provenance",
        ));
    }
    metadata.insert(
        "offline_v6_to_v7_conversion".to_owned(),
        json!({"legacy_fixed_expires_at": old_expires_at}),
    );
    set_text(
        row,
        "metadata_json",
        canonical_json(
            &Value::Object(metadata.clone()),
            "write_tickets",
            "metadata_json",
        )?,
    );
    match old_status.as_str() {
        "active" => {
            set_text(row, "status", "revoked");
            set_optional_text(
                row,
                "invalidation_reason",
                Some("explicit_revoke".to_owned()),
            );
            set_optional_text(row, "revoked_at", Some(converted_at.to_owned()));
        }
        "consumed" => {
            set_text(row, "status", "consumed");
            set_optional_text(row, "invalidation_reason", None);
        }
        "expired" | "stale" => {
            set_text(row, "status", "revoked");
            set_optional_text(
                row,
                "invalidation_reason",
                Some("explicit_revoke".to_owned()),
            );
            set_optional_text(row, "revoked_at", Some(converted_at.to_owned()));
        }
        "revoked" => {
            set_text(row, "status", "revoked");
            set_optional_text(
                row,
                "invalidation_reason",
                Some("explicit_revoke".to_owned()),
            );
        }
        _ => {
            return Err(upgrade_invariant(
                "v6 write ticket status is outside the owner-defined value set",
            ))
        }
    }
    row.remove("expires_at");
    Ok(())
}

fn transform_detective_guard_result(raw: &str) -> StoreResult<String> {
    let mut value: Value = serde_json::from_str(raw)
        .map_err(|_| upgrade_invariant("v6 guard result contains malformed JSON"))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| upgrade_invariant("v6 guard result must be a JSON object"))?;
    let tool = object.entry("tool").or_insert_with(|| json!({}));
    let tool = tool
        .as_object_mut()
        .ok_or_else(|| upgrade_invariant("v6 guard result tool assessment must be an object"))?;
    let changed_paths = tool.get("changed_paths").and_then(Value::as_array);
    let classification = tool.get("classification").and_then(Value::as_str);
    let structured_path_effect =
        changed_paths
            .filter(|paths| !paths.is_empty())
            .and_then(|paths| {
                let inside_repo = paths
                    .iter()
                    .map(|path| {
                        let path = path.as_object()?;
                        let raw = path.get("raw")?.as_str()?;
                        if raw.trim().is_empty() {
                            return None;
                        }
                        path.get("inside_repo")?.as_bool()
                    })
                    .collect::<Option<Vec<_>>>()?;
                Some(if inside_repo.into_iter().any(|inside| inside) {
                    "product_file_write"
                } else {
                    "external_effect"
                })
            });
    let (effect, confidence) = if let Some(effect) = structured_path_effect {
        (effect, "confirmed")
    } else if classification
        .is_some_and(|value| matches!(value, "read_only" | "read-only" | "read" | "readonly"))
    {
        ("read_only", "structured")
    } else if classification
        .is_some_and(|value| matches!(value, "write" | "mutation" | "mutating" | "product_write"))
    {
        ("product_file_write", "heuristic")
    } else {
        ("unknown", "heuristic")
    };
    tool.insert("effect".to_owned(), Value::String(effect.to_owned()));
    tool.insert(
        "confidence".to_owned(),
        Value::String(confidence.to_owned()),
    );
    canonical_json(&value, "guard_events", "result_json")
}

fn insert_conservative_project_policy(
    transaction: &Transaction<'_>,
    project_id: &str,
    converted_at: &str,
    converted_policy: &ConvertedProjectPolicy,
) -> StoreResult<()> {
    transaction.execute(
        "INSERT INTO project_workflow_policies (
            project_id, policy_schema, policy_version, policy_json,
            policy_fingerprint, source, applied_at, created_at
         ) VALUES (?1, 'volicord-policy-v2', 1, ?2, ?3, ?4, ?5, ?5)",
        params![
            project_id,
            converted_policy.canonical_json,
            converted_policy.fingerprint,
            "offline_v6_to_v7_conversion",
            converted_at
        ],
    )?;
    Ok(())
}

fn validate_authority_event_hash_chain(conn: &Connection) -> StoreResult<()> {
    let rows = read_table_rows(conn, "authority_events")?;
    let mut previous: Option<String> = None;
    for row in rows {
        let stored_previous = row_optional_text(&row, "authority_events", "previous_event_hash")?;
        if stored_previous != previous {
            return Err(upgrade_invariant(
                "v6 authority event previous-hash chain is discontinuous",
            ));
        }
        let computed = authority_event_hash(&row)?;
        let stored = row_text(&row, "authority_events", "event_hash")?;
        if computed != stored {
            return Err(upgrade_invariant(
                "v6 authority event hash does not match its canonical event fields",
            ));
        }
        previous = Some(stored);
    }
    Ok(())
}

fn authority_event_hash(row: &RowValues) -> StoreResult<String> {
    let mut digest = Sha256::new();
    fn update_field(digest: &mut Sha256, field: &str) {
        digest.update(field.len().to_string().as_bytes());
        digest.update(b":");
        digest.update(field.as_bytes());
        digest.update(b"\n");
    }
    for field in [
        row_text(row, "authority_events", "project_id")?,
        row_i64(row, "authority_events", "event_seq")?.to_string(),
        row_text(row, "authority_events", "event_id")?,
        row_i64(row, "authority_events", "state_version")?.to_string(),
        row_text(row, "authority_events", "event_type")?,
        row_text(row, "authority_events", "actor_source")?,
        row_text(row, "authority_events", "operation_category")?,
        row_optional_text(row, "authority_events", "task_id")?.unwrap_or_default(),
        row_optional_text(row, "authority_events", "change_unit_id")?.unwrap_or_default(),
        row_text(row, "authority_events", "payload_json")?,
        row_text(row, "authority_events", "request_hash")?,
        row_optional_text(row, "authority_events", "previous_event_hash")?.unwrap_or_default(),
        row_text(row, "authority_events", "created_at")?,
    ] {
        update_field(&mut digest, &field);
    }
    Ok(format!("sha256:{:x}", digest.finalize()))
}

fn verify_preserved_project_records(
    source: &Connection,
    destination: &Connection,
) -> StoreResult<u64> {
    let mut hash_checks = 0u64;
    for table in PROJECT_V6_TABLES {
        let source_rows = read_table_rows(source, table)?;
        let destination_rows = read_table_rows(destination, table)?;
        if source_rows.len() != destination_rows.len() {
            return Err(upgrade_invariant(format!(
                "destination row count for project table {table} does not match v6 source"
            )));
        }
        let primary_key = primary_key_columns(source, table)?;
        if primary_key.is_empty() {
            return Err(upgrade_invariant(format!(
                "v6 project table {table} has no durable primary key"
            )));
        }
        let common_columns: Vec<_> = table_columns(source, table)?
            .into_iter()
            .filter(|column| {
                destination_rows.first().map_or_else(
                    || {
                        table_columns(destination, table)
                            .map(|columns| columns.contains(column))
                            .unwrap_or(false)
                    },
                    |row| row.contains_key(column),
                )
            })
            .collect();
        for (source_row, destination_row) in source_rows.iter().zip(&destination_rows) {
            for column in &primary_key {
                if source_row.get(column) != destination_row.get(column) {
                    return Err(upgrade_invariant(format!(
                        "destination primary identifiers for project table {table} changed"
                    )));
                }
            }
            for column in &common_columns {
                if intentionally_transformed_column(table, column) {
                    continue;
                }
                if source_row.get(column) != destination_row.get(column) {
                    return Err(upgrade_invariant(format!(
                        "destination changed preserved project field {table}.{column}"
                    )));
                }
            }
            for column in common_columns
                .iter()
                .filter(|column| column.ends_with("_json"))
            {
                let Some(SqlValue::Text(source_raw)) = source_row.get(column) else {
                    continue;
                };
                let Some(SqlValue::Text(destination_raw)) = destination_row.get(column) else {
                    return Err(upgrade_invariant(format!(
                        "destination changed JSON storage type for {table}.{column}"
                    )));
                };
                let source_value: Value = serde_json::from_str(source_raw).map_err(|_| {
                    upgrade_invariant(format!("source {table}.{column} JSON became unreadable"))
                })?;
                let destination_value: Value =
                    serde_json::from_str(destination_raw).map_err(|_| {
                        upgrade_invariant(format!("destination {table}.{column} JSON is malformed"))
                    })?;
                let source_hash = canonical_json_sha256(&source_value).map_err(|_| {
                    upgrade_invariant(format!("source {table}.{column} JSON cannot be hashed"))
                })?;
                let destination_hash = canonical_json_sha256(&destination_value).map_err(|_| {
                    upgrade_invariant(format!(
                        "destination {table}.{column} JSON cannot be hashed"
                    ))
                })?;
                if !intentionally_transformed_column(table, column)
                    && source_hash != destination_hash
                {
                    return Err(upgrade_invariant(format!(
                        "canonical JSON hash changed for preserved field {table}.{column}"
                    )));
                }
                hash_checks = hash_checks
                    .checked_add(1)
                    .ok_or_else(|| upgrade_invariant("canonical hash check count overflow"))?;
            }
            for column in common_columns.iter().filter(|column| {
                column.contains("sha256")
                    || column.ends_with("_hash")
                    || column == &&"event_hash".to_owned()
                    || column == &&"previous_event_hash".to_owned()
            }) {
                if source_row.get(column) != destination_row.get(column) {
                    return Err(upgrade_invariant(format!(
                        "stored canonical hash changed for {table}.{column}"
                    )));
                }
                hash_checks = hash_checks
                    .checked_add(1)
                    .ok_or_else(|| upgrade_invariant("canonical hash check count overflow"))?;
            }
        }
    }
    validate_json_columns(destination, PROJECT_V6_TABLES, false)?;
    validate_converted_project_policy(destination)?;
    validate_authority_event_hash_chain(destination)?;
    Ok(hash_checks)
}

fn intentionally_transformed_column(table: &str, column: &str) -> bool {
    matches!(
        (table, column),
        ("project_state", "storage_profile")
            | ("write_tickets", "status")
            | ("write_tickets", "revoked_at")
            | ("write_tickets", "metadata_json")
            | ("guard_events", "result_json")
    )
}

fn validate_converted_project_policy(conn: &Connection) -> StoreResult<()> {
    let (schema, version, raw, fingerprint, source): (String, i64, String, String, String) = conn
        .query_row(
        "SELECT policy_schema, policy_version, policy_json, policy_fingerprint, source
               FROM project_workflow_policies",
        [],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        },
    )?;
    if schema != "volicord-policy-v2" || version != 1 || source != "offline_v6_to_v7_conversion" {
        return Err(upgrade_invariant(
            "converted project policy identity or version is malformed",
        ));
    }
    let value: Value = serde_json::from_str(&raw)
        .map_err(|_| upgrade_invariant("converted project policy JSON is malformed"))?;
    validate_converted_v2_policy_shape(&value)?;
    if canonical_json(&value, "project_workflow_policies", "policy_json")? != raw {
        return Err(upgrade_invariant(
            "converted project policy is not canonical JSON",
        ));
    }
    let expected = canonical_json_sha256(&value)
        .map_err(|_| upgrade_invariant("converted project policy cannot be fingerprinted"))?
        .as_str()
        .to_owned();
    if fingerprint != expected {
        return Err(upgrade_invariant(
            "converted project policy fingerprint does not match policy JSON",
        ));
    }
    Ok(())
}

fn copy_project_artifacts(project: &SourceProject, staging_home: &Path) -> StoreResult<()> {
    let source_artifacts = project.source_project_home.join(ARTIFACTS_DIR);
    if !source_artifacts.exists() {
        return Ok(());
    }
    if !source_artifacts.is_dir() {
        return Err(upgrade_invariant(format!(
            "v6 project {} artifact store is not a directory",
            project.project_id
        )));
    }
    let destination_artifacts = staging_home
        .join(PROJECTS_DIR)
        .join(&project.project_id)
        .join(ARTIFACTS_DIR);
    fs::create_dir(&destination_artifacts)?;
    copy_directory_contents(&source_artifacts, &destination_artifacts)?;
    if projected_artifact_manifest(&source_artifacts)?
        != projected_artifact_manifest(&destination_artifacts)?
    {
        return Err(upgrade_invariant(format!(
            "copied artifact bytes for v6 project {} do not match source",
            project.project_id
        )));
    }
    Ok(())
}

fn copy_directory_contents(source: &Path, destination: &Path) -> StoreResult<()> {
    let canonical_root = fs::canonicalize(source)?;
    let mut active_directories = BTreeSet::new();
    copy_materialized_directory(
        source,
        destination,
        &canonical_root,
        &mut active_directories,
    )
}

fn copy_materialized_directory(
    source: &Path,
    destination: &Path,
    canonical_root: &Path,
    active_directories: &mut BTreeSet<PathBuf>,
) -> StoreResult<()> {
    let canonical_source = fs::canonicalize(source)?;
    if !canonical_source.starts_with(canonical_root) {
        return Err(upgrade_invariant(
            "artifact directory resolves outside its artifact store",
        ));
    }
    if !active_directories.insert(canonical_source.clone()) {
        return Err(upgrade_invariant(
            "artifact directory symbolic links contain a cycle",
        ));
    }
    let mut entries = fs::read_dir(source)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.file_type().is_symlink() {
            let target = fs::canonicalize(&source_path)?;
            if !target.starts_with(canonical_root) {
                return Err(upgrade_invariant(
                    "artifact symbolic link resolves outside its artifact store",
                ));
            }
            let target_metadata = fs::metadata(&target)?;
            if target_metadata.is_dir() {
                fs::create_dir(&destination_path)?;
                copy_materialized_directory(
                    &target,
                    &destination_path,
                    canonical_root,
                    active_directories,
                )?;
            } else if target_metadata.is_file() {
                copy_regular_file(&target, &destination_path, target_metadata.len())?;
            } else {
                return Err(upgrade_invariant(
                    "artifact symbolic link resolves to a special file",
                ));
            }
        } else if metadata.is_dir() {
            fs::create_dir(&destination_path)?;
            copy_materialized_directory(
                &source_path,
                &destination_path,
                canonical_root,
                active_directories,
            )?;
        } else if metadata.is_file() {
            copy_regular_file(&source_path, &destination_path, metadata.len())?;
        } else {
            return Err(upgrade_invariant(
                "artifact store contains an unsupported special file",
            ));
        }
    }
    active_directories.remove(&canonical_source);
    Ok(())
}

fn copy_regular_file(source: &Path, destination: &Path, expected_size: u64) -> StoreResult<()> {
    let copied = fs::copy(source, destination)?;
    if copied != expected_size {
        return Err(upgrade_invariant(
            "artifact copy byte count does not match source metadata",
        ));
    }
    Ok(())
}

fn projected_artifact_manifest(root: &Path) -> StoreResult<Vec<FileManifestEntry>> {
    let canonical_root = fs::canonicalize(root)?;
    let mut active_directories = BTreeSet::new();
    let mut output = Vec::new();
    collect_projected_artifact_entries(
        root,
        Path::new(""),
        &canonical_root,
        &mut active_directories,
        &mut output,
    )?;
    output.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(output)
}

fn collect_projected_artifact_entries(
    directory: &Path,
    relative_directory: &Path,
    canonical_root: &Path,
    active_directories: &mut BTreeSet<PathBuf>,
    output: &mut Vec<FileManifestEntry>,
) -> StoreResult<()> {
    let canonical_directory = fs::canonicalize(directory)?;
    if !canonical_directory.starts_with(canonical_root) {
        return Err(upgrade_invariant(
            "artifact directory resolves outside its artifact store",
        ));
    }
    if !active_directories.insert(canonical_directory.clone()) {
        return Err(upgrade_invariant(
            "artifact directory symbolic links contain a cycle",
        ));
    }
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let relative_path = relative_directory.join(entry.file_name());
        let link_metadata = fs::symlink_metadata(&path)?;
        let (resolved, metadata) = if link_metadata.file_type().is_symlink() {
            let target = fs::canonicalize(&path)?;
            if !target.starts_with(canonical_root) {
                return Err(upgrade_invariant(
                    "artifact symbolic link resolves outside its artifact store",
                ));
            }
            let metadata = fs::metadata(&target)?;
            (target, metadata)
        } else {
            (path.clone(), link_metadata)
        };
        if metadata.is_dir() {
            output.push(FileManifestEntry {
                relative_path: relative_path.clone(),
                kind: "directory",
                size_bytes: 0,
                sha256: None,
                link_target: None,
            });
            collect_projected_artifact_entries(
                &resolved,
                &relative_path,
                canonical_root,
                active_directories,
                output,
            )?;
        } else if metadata.is_file() {
            output.push(FileManifestEntry {
                relative_path,
                kind: "file",
                size_bytes: metadata.len(),
                sha256: Some(file_sha256(&resolved)?),
                link_target: None,
            });
        } else {
            return Err(upgrade_invariant(
                "artifact store contains an unsupported special file",
            ));
        }
    }
    active_directories.remove(&canonical_directory);
    Ok(())
}

fn verify_artifact_records(conn: &Connection, project_home: &Path) -> StoreResult<()> {
    let artifact_store = project_home.join(ARTIFACTS_DIR);
    for row in read_table_rows(conn, "artifacts")? {
        let body_path = row_optional_text(&row, "artifacts", "body_path")?;
        let sha256 = row_optional_text(&row, "artifacts", "sha256")?;
        let content_type = row_optional_text(&row, "artifacts", "content_type")?;
        let size_bytes =
            match row.get("size_bytes") {
                Some(SqlValue::Null) => None,
                Some(SqlValue::Integer(value)) => Some(u64::try_from(*value).map_err(|_| {
                    upgrade_invariant("artifact size is outside the supported range")
                })?),
                _ => {
                    return Err(upgrade_invariant(
                        "artifact size is not stored as a nullable integer",
                    ))
                }
            };
        let integrity_status = row_text(&row, "artifacts", "integrity_status")?;
        let availability_status = row_text(&row, "artifacts", "status")?;
        let verification = verify_persistent_artifact_body(
            &artifact_store,
            &PersistentArtifactBodySpec {
                body_path: body_path.as_deref(),
                sha256: sha256.as_deref(),
                size_bytes,
                content_type: content_type.as_deref(),
                integrity_status: &integrity_status,
                availability_status: &availability_status,
            },
        )?;
        if verification.status == PersistentArtifactVerificationStatus::BoundaryViolation
            || (integrity_status == "verified"
                && availability_status == "available"
                && verification.status != PersistentArtifactVerificationStatus::VerifiedCurrent)
        {
            return Err(upgrade_invariant(
                "available v6 artifact body does not match its canonical hash and size",
            ));
        }
    }
    Ok(())
}

fn copy_optional_diagnostics(source_home: &Path, staging_home: &Path) -> StoreResult<()> {
    let source_path = source_home.join(DIAGNOSTICS_DB_FILE);
    if !source_path.exists() {
        return Ok(());
    }
    reject_sqlite_sidecars(&source_path)?;
    let source = open_read_only_database(&source_path)?;
    validate_database_integrity(&source, "v6 diagnostics")?;
    let version: i64 = source.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let expected_sql = match version {
        1 => DIAGNOSTICS_SCHEMA_V1_SQL.to_owned(),
        2 => format!("{DIAGNOSTICS_SCHEMA_V1_SQL}\n{DIAGNOSTICS_SCHEMA_V2_SQL}"),
        _ => {
            return Err(upgrade_invariant(
                "v6 diagnostics database has an unsupported schema version",
            ))
        }
    };
    validate_exact_schema(&source, &expected_sql, "v6 diagnostics")?;
    drop(source);

    let destination_path = staging_home.join(DIAGNOSTICS_DB_FILE);
    fs::copy(&source_path, &destination_path)?;
    if file_sha256(&source_path)? != file_sha256(&destination_path)? {
        return Err(upgrade_invariant(
            "diagnostics database copy hash does not match source",
        ));
    }
    ensure_current_diagnostics_schema(staging_home)?;
    let destination = open_read_only_database(&destination_path)?;
    validate_database_integrity(&destination, "destination diagnostics")?;
    let destination_version: i64 =
        destination.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if destination_version != 2 {
        return Err(upgrade_invariant(
            "destination diagnostics database did not reach schema version 2",
        ));
    }
    Ok(())
}

fn validate_destination_registry(
    staging_home: &Path,
    destination_home: &Path,
    source_projects: &[SourceProject],
    preserved_counts: &BTreeMap<String, u64>,
) -> StoreResult<()> {
    let registry_path = registry_db_path(staging_home);
    let conn = open_read_only_database(&registry_path)?;
    validate_registry_schema(&conn)?;
    validate_database_integrity(&conn, "destination registry")?;
    let (runtime_home_path, stored_registry_path, storage_profile): (String, String, String) = conn
        .query_row(
            "SELECT runtime_home_path, registry_db_path, storage_profile FROM runtime_home",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
    if Path::new(&runtime_home_path) != destination_home
        || Path::new(&stored_registry_path) != registry_db_path(destination_home)
        || storage_profile != STORAGE_PROFILE
    {
        return Err(upgrade_invariant(
            "destination registry does not contain final v7 Runtime Home coordinates",
        ));
    }

    let expected_ids: BTreeSet<_> = source_projects
        .iter()
        .map(|project| project.project_id.clone())
        .collect();
    let mut statement = conn.prepare(
        "SELECT project_internal_id, project_home, state_db_path
           FROM projects
          ORDER BY project_internal_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut actual_ids = BTreeSet::new();
    for row in rows {
        let (project_id, project_home, state_db_path) = row?;
        actual_ids.insert(project_id.clone());
        let final_project_home = destination_home.join(PROJECTS_DIR).join(&project_id);
        if Path::new(&project_home) != final_project_home
            || Path::new(&state_db_path) != final_project_home.join(PROJECT_STATE_DB_FILE)
        {
            return Err(upgrade_invariant(format!(
                "destination project {project_id} has non-final registry coordinates"
            )));
        }
        let staged_state = project_state_db_path(staging_home, &project_id);
        if !staged_state.is_file() {
            return Err(upgrade_invariant(format!(
                "destination project {project_id} state database is missing"
            )));
        }
        let project_conn = open_read_only_database(&staged_state)?;
        validate_project_state_schema(&project_conn)?;
        validate_database_integrity(&project_conn, "destination project state")?;
    }
    if actual_ids != expected_ids {
        return Err(upgrade_invariant(
            "destination project identifiers do not match the source registry",
        ));
    }
    let expected_project_count = u64::try_from(source_projects.len())
        .map_err(|_| upgrade_invariant("source project count exceeds report value range"))?;
    if preserved_counts.get("registry.projects") != Some(&expected_project_count) {
        return Err(upgrade_invariant(
            "destination registry project count does not match preserved report count",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use volicord_test_support::TempRuntimeHome;

    use super::*;

    #[test]
    fn reconstructed_v6_project_schema_matches_maintained_hash() {
        let sql = v6_project_schema_sql().expect("v6 schema should reconstruct");
        assert_eq!(
            format!("{:x}", Sha256::digest(sql.as_bytes())),
            V6_PROJECT_SCHEMA_SHA256
        );
    }

    #[test]
    fn offline_upgrade_preserves_authority_and_applies_conservative_v7_controls(
    ) -> Result<(), Box<dyn Error>> {
        let source = TempRuntimeHome::new("storage-upgrade-v6-source")?;
        let destination = TempRuntimeHome::new("storage-upgrade-v7-destination")?;
        let repository = TempRuntimeHome::new("storage-upgrade-repository")?;
        create_v6_fixture(source.path(), repository.path())?;
        let source_state =
            open_read_only_database(project_state_db_path(source.path(), "project_legacy"))?;
        let source_resolution_json: String = source_state.query_row(
            "SELECT resolution_json FROM user_action_resolutions
              WHERE user_action_resolution_id = 'resolution_legacy'",
            [],
            |row| row.get(0),
        )?;
        let source_evidence_json: String = source_state.query_row(
            "SELECT metadata_json FROM evidence_observations
              WHERE evidence_observation_id = 'observation_legacy'",
            [],
            |row| row.get(0),
        )?;
        let source_resolution_hash =
            canonical_json_sha256(&serde_json::from_str::<Value>(&source_resolution_json)?)?;
        let source_evidence_hash =
            canonical_json_sha256(&serde_json::from_str::<Value>(&source_evidence_json)?)?;
        drop(source_state);
        let source_before = runtime_home_manifest(source.path())?;

        let report = upgrade_runtime_home_v6_to_v7(source.path(), destination.path())?;

        assert_eq!(report.status, "completed");
        assert!(report.source_unchanged);
        assert!(report.destination_ready);
        assert_eq!(report.source_profile, STORAGE_UPGRADE_SOURCE_PROFILE);
        assert_eq!(report.destination_profile, STORAGE_PROFILE);
        assert_eq!(
            report.activation_action,
            "administrator_rebind_destination_home_and_activate_separately"
        );
        assert!(report.canonical_hash_check_count > 0);
        assert_eq!(runtime_home_manifest(source.path())?, source_before);

        let _registry =
            crate::sqlite::open_registry_database_read_only(registry_db_path(destination.path()))?;
        let state = crate::sqlite::open_project_state_database_read_only(project_state_db_path(
            destination.path(),
            "project_legacy",
        ))?;
        let controls: (String, String, String) = state.query_row(
            "SELECT requested_control_level, effective_control_level, control_level_reason
               FROM tasks WHERE task_id = 'task_legacy'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        assert_eq!(controls.0, "tracked");
        assert_eq!(controls.1, "tracked");
        assert!(controls.2.contains("v6_to_v7"));
        for (task_id, requested, effective) in [
            ("task_advisor_legacy", "observe", "observe"),
            ("task_direct_legacy", "tracked", "tracked"),
        ] {
            let actual: (String, String) = state.query_row(
                "SELECT requested_control_level, effective_control_level
                   FROM tasks WHERE task_id = ?1",
                [task_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            assert_eq!(actual, (requested.to_owned(), effective.to_owned()));
        }
        let ticket: (String, String, Option<String>) = state.query_row(
            "SELECT status, invalidation_reason, revoked_at
               FROM write_tickets WHERE write_ticket_id = 'ticket_legacy'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        assert_eq!(ticket.0, "revoked");
        assert_eq!(ticket.1, "explicit_revoke");
        assert!(ticket.2.is_some());
        assert_eq!(
            state.query_row(
                "SELECT idle_expires_at FROM write_tickets
                  WHERE write_ticket_id = 'ticket_legacy'",
                [],
                |row| row.get::<_, Option<String>>(0),
            )?,
            None
        );
        let consumed: (String, Option<String>, Option<String>) = state.query_row(
            "SELECT status, consumed_by_run_id, consumed_at
               FROM write_tickets WHERE write_ticket_id = 'ticket_consumed_legacy'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        assert_eq!(consumed.0, "consumed");
        assert_eq!(consumed.1.as_deref(), Some("run_consumed_legacy"));
        assert!(consumed.2.is_some());
        assert_eq!(
            state.query_row(
                "SELECT idle_expires_at FROM write_tickets
                  WHERE write_ticket_id = 'ticket_consumed_legacy'",
                [],
                |row| row.get::<_, Option<String>>(0),
            )?,
            None
        );
        for (ticket_id, legacy_expires_at) in [
            ("ticket_expired_legacy", "2026-07-16T00:04:00Z"),
            ("ticket_stale_legacy", "2026-07-16T03:00:00Z"),
        ] {
            let converted: (
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                String,
            ) = state.query_row(
                "SELECT status, invalidation_reason, revoked_at, idle_expires_at,
                            metadata_json
                       FROM write_tickets WHERE write_ticket_id = ?1",
                [ticket_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )?;
            assert_eq!(converted.0, "revoked");
            assert_eq!(converted.1.as_deref(), Some("explicit_revoke"));
            assert!(converted.2.is_some());
            assert_eq!(converted.3, None);
            let metadata: Value = serde_json::from_str(&converted.4)?;
            assert_eq!(metadata["legacy_marker"], "preserved");
            assert_eq!(
                metadata["offline_v6_to_v7_conversion"]["legacy_fixed_expires_at"],
                legacy_expires_at
            );
        }
        assert_eq!(
            state
                .query_row(
                    "SELECT write_ticket_id FROM runs WHERE run_id = 'run_consumed_legacy'",
                    [],
                    |row| row.get::<_, Option<String>>(0),
                )?
                .as_deref(),
            Some("ticket_consumed_legacy")
        );
        let converted_store = crate::core_pipeline::CoreProjectStore::open(
            destination.path(),
            &volicord_types::ProjectId::new("project_legacy"),
        )?;
        let run = converted_store
            .run_record("run_consumed_legacy")?
            .expect("converted Run must remain readable through Store projection");
        assert_eq!(run.status, "recorded");
        assert_eq!(run.baseline_ref.as_deref(), Some("baseline_legacy"));
        let observed_runs = converted_store
            .run_observed_changes_for_task(&volicord_types::TaskId::new("task_legacy"))?;
        let observed_run = observed_runs
            .iter()
            .find(|record| record.run_id == "run_consumed_legacy")
            .expect("converted Run observed changes must remain typed-readable");
        assert_eq!(observed_run.observed_changes.changed_paths, ["src/lib.rs"]);
        assert!(observed_run.observed_changes.product_file_write_observed);
        let projected_ticket = converted_store
            .write_ticket_record("ticket_consumed_legacy")?
            .expect("converted consumed ticket must remain readable through Store projection");
        assert_eq!(projected_ticket.status, "consumed");
        assert_eq!(
            projected_ticket.consumed_by_run_id.as_deref(),
            Some("run_consumed_legacy")
        );
        assert_eq!(
            state.query_row(
                "SELECT confidence FROM unrecorded_changes
                  WHERE unrecorded_change_id = 'unrecorded_legacy'",
                [],
                |row| row.get::<_, String>(0),
            )?,
            "confirmed"
        );
        let guard_result: String = state.query_row(
            "SELECT result_json FROM guard_events WHERE guard_event_id = 'guard_event_legacy'",
            [],
            |row| row.get(0),
        )?;
        let guard_result: Value = serde_json::from_str(&guard_result)?;
        assert_eq!(guard_result["tool"]["effect"], "product_file_write");
        assert_eq!(guard_result["tool"]["confidence"], "confirmed");
        let heuristic_guard_result: String = state.query_row(
            "SELECT result_json FROM guard_events
              WHERE guard_event_id = 'guard_event_heuristic_legacy'",
            [],
            |row| row.get(0),
        )?;
        let heuristic_guard_result: Value = serde_json::from_str(&heuristic_guard_result)?;
        assert_eq!(
            heuristic_guard_result["tool"]["effect"],
            "product_file_write"
        );
        assert_eq!(heuristic_guard_result["tool"]["confidence"], "heuristic");
        let destination_resolution_json: String = state.query_row(
            "SELECT resolution_json FROM user_action_resolutions
              WHERE user_action_resolution_id = 'resolution_legacy'",
            [],
            |row| row.get(0),
        )?;
        let destination_evidence_json: String = state.query_row(
            "SELECT metadata_json FROM evidence_observations
              WHERE evidence_observation_id = 'observation_legacy'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(
            canonical_json_sha256(&serde_json::from_str::<Value>(
                &destination_resolution_json
            )?)?,
            source_resolution_hash
        );
        assert_eq!(
            canonical_json_sha256(&serde_json::from_str::<Value>(&destination_evidence_json)?)?,
            source_evidence_hash
        );
        assert_eq!(
            state.query_row(
                "SELECT policy_version FROM project_workflow_policies",
                [],
                |row| row.get::<_, i64>(0),
            )?,
            1
        );
        let (policy_json, policy_fingerprint): (String, String) = state.query_row(
            "SELECT policy_json, policy_fingerprint FROM project_workflow_policies",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let policy: Value = serde_json::from_str(&policy_json)?;
        assert_eq!(policy["schema"], "volicord-policy-v2");
        assert_eq!(policy["managed_by"], "volicord");
        assert_eq!(policy["connection_id"], "connection_legacy");
        assert_eq!(policy["guard_installation_id"], "guard_legacy");
        assert_eq!(policy["selected_profile"], "detective");
        assert_eq!(policy["host_hook"]["enabled"], true);
        assert_eq!(policy["workflow"], conservative_workflow_policy_json());
        assert_eq!(policy_json, canonical_json_string(&policy)?);
        assert_eq!(policy_fingerprint, canonical_json_sha256(&policy)?.as_str());
        assert_eq!(
            fs::read(
                destination
                    .path()
                    .join(PROJECTS_DIR)
                    .join("project_legacy")
                    .join(ARTIFACTS_DIR)
                    .join("body.txt")
            )?,
            b"legacy evidence"
        );
        let stored_report: StorageUpgradeReport =
            read_json_file(&destination.path().join(STORAGE_UPGRADE_REPORT_FILE))?;
        assert_eq!(stored_report, report);
        let diagnostics = open_read_only_database(destination.path().join(DIAGNOSTICS_DB_FILE))?;
        assert_eq!(
            diagnostics.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))?,
            2
        );
        assert_eq!(
            diagnostics.query_row("SELECT COUNT(*) FROM diagnostic_sessions", [], |row| {
                row.get::<_, i64>(0)
            })?,
            1
        );
        Ok(())
    }

    #[test]
    fn malformed_v6_schema_leaves_empty_destination_unaccepted() -> Result<(), Box<dyn Error>> {
        let source = TempRuntimeHome::new("storage-upgrade-invalid-source")?;
        let destination = TempRuntimeHome::new("storage-upgrade-invalid-destination")?;
        let repository = TempRuntimeHome::new("storage-upgrade-invalid-repository")?;
        create_v6_fixture(source.path(), repository.path())?;
        let state_path = project_state_db_path(source.path(), "project_legacy");
        Connection::open(&state_path)?
            .execute_batch("ALTER TABLE tasks ADD COLUMN unexpected_legacy_column TEXT")?;
        let source_before = runtime_home_manifest(source.path())?;

        assert!(upgrade_runtime_home_v6_to_v7(source.path(), destination.path()).is_err());

        assert_eq!(runtime_home_manifest(source.path())?, source_before);
        assert!(fs::read_dir(destination.path())?.next().is_none());
        let staging = staging_home_path(destination.path())?;
        assert!(staging.join(INCOMPLETE_MARKER_FILE).is_file());
        fs::remove_dir_all(staging)?;
        Ok(())
    }

    #[test]
    fn stale_policy_mcp_bindings_fail_before_destination_staging() -> Result<(), Box<dyn Error>> {
        let source = TempRuntimeHome::new("storage-upgrade-stale-policy-source")?;
        let destination = TempRuntimeHome::new("storage-upgrade-stale-policy-destination")?;
        let repository = TempRuntimeHome::new("storage-upgrade-stale-policy-repository")?;
        create_v6_fixture(source.path(), repository.path())?;
        let policy_path = repository.path().join(LEGACY_PROJECT_POLICY_FILE);
        let original_policy: Value = serde_json::from_slice(&fs::read(&policy_path)?)?;

        for (field, stale_value, expected_detail) in [
            (
                "VOLICORD_HOME",
                "/different/runtime-home",
                "MCP Runtime Home binding is stale",
            ),
            ("VOLICORD_MCP_HOST", "claude_code", "VOLICORD_MCP_HOST"),
            (
                "VOLICORD_MCP_CONNECTION_ID",
                "connection_other",
                "VOLICORD_MCP_CONNECTION_ID",
            ),
            (
                "VOLICORD_MCP_PROJECT_ID",
                "project_other",
                "VOLICORD_MCP_PROJECT_ID",
            ),
            ("VOLICORD_MCP_LAUNCH", "manual", "VOLICORD_MCP_LAUNCH"),
        ] {
            let mut policy = original_policy.clone();
            policy["mcp"]["env"][field] = Value::String(stale_value.to_owned());
            write_json_file(&policy_path, &policy)?;
            let source_before = runtime_home_manifest(source.path())?;
            let policy_before = fs::read(&policy_path)?;

            let error = upgrade_runtime_home_v6_to_v7(source.path(), destination.path())
                .expect_err("a stale managed MCP binding must fail closed");

            assert!(error.to_string().contains(expected_detail));
            assert_eq!(runtime_home_manifest(source.path())?, source_before);
            assert_eq!(fs::read(&policy_path)?, policy_before);
            assert!(fs::read_dir(destination.path())?.next().is_none());
            assert!(!staging_home_path(destination.path())?.exists());
        }
        Ok(())
    }

    #[test]
    fn wrong_shaped_typed_owner_json_leaves_source_and_destination_unaccepted(
    ) -> Result<(), Box<dyn Error>> {
        let source = TempRuntimeHome::new("storage-upgrade-wrong-owner-json-source")?;
        let destination = TempRuntimeHome::new("storage-upgrade-wrong-owner-json-destination")?;
        let repository = TempRuntimeHome::new("storage-upgrade-wrong-owner-json-repository")?;
        create_v6_fixture(source.path(), repository.path())?;
        let state_path = project_state_db_path(source.path(), "project_legacy");
        Connection::open(&state_path)?.execute(
            "INSERT INTO user_action_requests (
                project_id, user_action_request_id, task_id, change_unit_id,
                action_kind, request_json, basis_json, required_for_json,
                requested_by_actor_source, source_method, source_idempotency_key,
                requested_at, metadata_json
             ) VALUES (
                'project_legacy', 'request_wrong_shape', 'task_legacy', 'change_legacy',
                'product_decision', '{}', '{}', '[]',
                'agent_connection:connection_legacy', 'volicord.request_user_action',
                'wrong-shaped-owner-json', '2026-07-16T00:12:00Z', '{}'
             )",
            [],
        )?;
        let source_before = runtime_home_manifest(source.path())?;

        assert!(upgrade_runtime_home_v6_to_v7(source.path(), destination.path()).is_err());

        assert_eq!(runtime_home_manifest(source.path())?, source_before);
        assert!(fs::read_dir(destination.path())?.next().is_none());
        let staging = staging_home_path(destination.path())?;
        assert!(staging.join(INCOMPLETE_MARKER_FILE).is_file());
        fs::remove_dir_all(staging)?;
        Ok(())
    }

    #[test]
    fn retry_replaces_only_its_owned_incomplete_staging() -> Result<(), Box<dyn Error>> {
        let source = TempRuntimeHome::new("storage-upgrade-retry-source")?;
        let destination = TempRuntimeHome::new("storage-upgrade-retry-destination")?;
        let repository = TempRuntimeHome::new("storage-upgrade-retry-repository")?;
        create_v6_fixture(source.path(), repository.path())?;
        let body_path = source
            .path()
            .join(PROJECTS_DIR)
            .join("project_legacy")
            .join(ARTIFACTS_DIR)
            .join("body.txt");
        fs::write(&body_path, b"temporarily corrupt evidence")?;

        assert!(upgrade_runtime_home_v6_to_v7(source.path(), destination.path()).is_err());
        let staging = staging_home_path(destination.path())?;
        assert!(staging.join(INCOMPLETE_MARKER_FILE).is_file());
        assert!(fs::read_dir(destination.path())?.next().is_none());

        fs::write(body_path, b"legacy evidence")?;
        let report = upgrade_runtime_home_v6_to_v7(source.path(), destination.path())?;

        assert!(report.destination_ready);
        assert!(!staging.exists());
        assert!(destination
            .path()
            .join(STORAGE_UPGRADE_REPORT_FILE)
            .is_file());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn contained_artifact_symlink_is_materialized_without_mutating_source(
    ) -> Result<(), Box<dyn Error>> {
        use std::os::unix::fs::symlink;

        let source = TempRuntimeHome::new("storage-upgrade-symlink-source")?;
        let destination = TempRuntimeHome::new("storage-upgrade-symlink-destination")?;
        let repository = TempRuntimeHome::new("storage-upgrade-symlink-repository")?;
        create_v6_fixture(source.path(), repository.path())?;
        let artifacts = source
            .path()
            .join(PROJECTS_DIR)
            .join("project_legacy")
            .join(ARTIFACTS_DIR);
        fs::rename(
            artifacts.join("body.txt"),
            artifacts.join("canonical-body.txt"),
        )?;
        symlink("canonical-body.txt", artifacts.join("body.txt"))?;
        let source_before = runtime_home_manifest(source.path())?;

        upgrade_runtime_home_v6_to_v7(source.path(), destination.path())?;

        assert_eq!(runtime_home_manifest(source.path())?, source_before);
        let destination_body = destination
            .path()
            .join(PROJECTS_DIR)
            .join("project_legacy")
            .join(ARTIFACTS_DIR)
            .join("body.txt");
        assert!(fs::symlink_metadata(&destination_body)?.is_file());
        assert_eq!(fs::read(destination_body)?, b"legacy evidence");
        Ok(())
    }

    fn create_v6_fixture(runtime_home: &Path, repo_root: &Path) -> StoreResult<()> {
        let runtime_home = fs::canonicalize(runtime_home)?;
        let repo_root = fs::canonicalize(repo_root)?;
        let registry_path = registry_db_path(&runtime_home);
        let project_home = runtime_home.join(PROJECTS_DIR).join("project_legacy");
        let state_path = project_home.join(PROJECT_STATE_DB_FILE);
        fs::create_dir_all(project_home.join(ARTIFACTS_DIR))?;

        let registry = Connection::open(&registry_path)?;
        registry.execute_batch(REGISTRY_SCHEMA_SQL)?;
        registry.execute(
            "INSERT INTO runtime_home (
                singleton_id, runtime_home_id, runtime_home_path, registry_db_path,
                storage_profile, metadata_json, created_at, updated_at
             ) VALUES (1, 'runtime_legacy', ?1, ?2, ?3, '{}', ?4, ?4)",
            params![
                path_text(&runtime_home, "fixture_runtime_home")?,
                path_text(&registry_path, "fixture_registry")?,
                STORAGE_UPGRADE_SOURCE_PROFILE,
                "2026-07-16T00:00:00Z"
            ],
        )?;
        registry.execute(
            "INSERT INTO projects (
                project_internal_id, project_name, project_alias, runtime_home_id,
                repo_root, project_home, state_db_path, metadata_json, created_at, updated_at
             ) VALUES (
                'project_legacy', 'Legacy project', 'legacy', 'runtime_legacy',
                ?1, ?2, ?3, '{}', ?4, ?4
             )",
            params![
                path_text(&repo_root, "fixture_repo")?,
                path_text(&project_home, "fixture_project_home")?,
                path_text(&state_path, "fixture_state")?,
                "2026-07-16T00:00:00Z"
            ],
        )?;
        registry.execute(
            "INSERT INTO agent_connections (
                connection_internal_id, host_kind, intent, host_scope,
                project_internal_id, server_name, config_target, mode, enabled,
                managed_fingerprint, created_at, updated_at
             ) VALUES (
                'connection_legacy', 'codex', 'personal', 'project',
                'project_legacy', 'volicord', ?1, 'workflow', 1,
                'legacy-managed-fingerprint', ?2, ?2
             )",
            params![
                path_text(
                    &repo_root.join(".codex").join("config.toml"),
                    "fixture_connection_target"
                )?,
                "2026-07-16T00:00:00Z"
            ],
        )?;
        registry.execute(
            "INSERT INTO connection_projects (
                connection_internal_id, project_internal_id, created_at
             ) VALUES ('connection_legacy', 'project_legacy', ?1)",
            ["2026-07-16T00:00:00Z"],
        )?;
        registry.execute(
            "INSERT INTO guard_installations (
                guard_installation_id, runtime_home_id, connection_internal_id,
                project_internal_id, host_kind, guard_mode, installation_status,
                installed_at, last_checked_at, created_at, updated_at
             ) VALUES (
                'guard_legacy', 'runtime_legacy', 'connection_legacy',
                'project_legacy', 'codex', 'detective', 'configured',
                ?1, ?1, ?1, ?1
             )",
            ["2026-07-16T00:00:00Z"],
        )?;
        drop(registry);

        let policy_directory = repo_root.join(".volicord");
        fs::create_dir_all(&policy_directory)?;
        let legacy_policy = json!({
            "schema": "volicord-policy-v1",
            "managed_by": "volicord",
            "storage_scope": "local_overlay",
            "connection_intent": "personal",
            "host": "codex",
            "repo_root": path_text(&repo_root, "fixture_repo")?,
            "connection_id": "connection_legacy",
            "guard_installation_id": "guard_legacy",
            "selected_profile": "detective",
            "mcp": {
                "command": "/bin/volicord",
                "args": ["mcp", "--stdio"],
                "env": {
                    "VOLICORD_HOME": path_text(&runtime_home, "fixture_runtime_home")?,
                    "VOLICORD_MCP_HOST": "codex",
                    "VOLICORD_MCP_CONNECTION_ID": "connection_legacy",
                    "VOLICORD_MCP_PROJECT_ID": "project_legacy"
                }
            },
            "host_hook": {
                "enabled": true,
                "commands": {
                    "session_start": {
                        "command": "/bin/volicord",
                        "args": ["_hook", "session-start"]
                    },
                    "pre_tool": {
                        "command": "/bin/volicord",
                        "args": ["_hook", "pre-tool"]
                    },
                    "post_tool": {
                        "command": "/bin/volicord",
                        "args": ["_hook", "post-tool"]
                    },
                    "prompt_capture": {
                        "command": "/bin/volicord",
                        "args": ["_hook", "prompt-capture"]
                    },
                    "stop": {
                        "command": "/bin/volicord",
                        "args": ["_hook", "stop"]
                    }
                }
            }
        });
        write_json_file(&policy_directory.join("policy.json"), &legacy_policy)?;

        let state = Connection::open(&state_path)?;
        state.execute_batch(&v6_project_schema_sql()?)?;
        state.execute(
            "INSERT INTO project_state (
                project_id, storage_profile, state_version, created_at, updated_at
             ) VALUES ('project_legacy', ?1, 1, ?2, ?2)",
            params![STORAGE_UPGRADE_SOURCE_PROFILE, "2026-07-16T00:00:00Z"],
        )?;
        state.execute(
            "INSERT INTO tasks (
                project_id, task_id, created_by_actor_source, mode, work_phase,
                acceptance_policy, acceptance_policy_reason, lifecycle_phase,
                title, scope_revision, close_basis_revision, created_at, updated_at
             ) VALUES (
                'project_legacy', 'task_legacy', 'agent', 'work', 'implementation',
                'required', 'legacy acceptance remains required', 'ready',
                'Legacy Task', 2, 0, ?1, ?1
             )",
            ["2026-07-16T00:00:00Z"],
        )?;
        for (task_id, mode, acceptance_policy) in [
            ("task_advisor_legacy", "advisor", "not_required"),
            ("task_direct_legacy", "direct", "required"),
        ] {
            state.execute(
                "INSERT INTO tasks (
                    project_id, task_id, created_by_actor_source, mode, work_phase,
                    acceptance_policy, acceptance_policy_reason, lifecycle_phase,
                    title, scope_revision, close_basis_revision, created_at, updated_at
                 ) VALUES (
                    'project_legacy', ?1, 'agent', ?2, 'implementation',
                    ?3, 'legacy acceptance outcome is preserved', 'ready',
                    ?1, 0, 0, ?4, ?4
                 )",
                params![task_id, mode, acceptance_policy, "2026-07-16T00:00:00Z"],
            )?;
        }
        state.execute(
            "INSERT INTO change_units (
                project_id, change_unit_id, task_id, status, is_current,
                basis_state_version, created_at, updated_at
             ) VALUES (
                'project_legacy', 'change_legacy', 'task_legacy', 'active', 1,
                1, ?1, ?1
             )",
            ["2026-07-16T00:00:00Z"],
        )?;
        let request_json = canonical_json(
            &json!({
                "body": {
                    "action_type": "choice",
                    "judgment_kind": "product_decision",
                    "presentation": "short",
                    "question": "Keep the legacy product direction?",
                    "options": [{
                        "option_id": "accept",
                        "label": "Accept",
                        "description": "Preserve the product direction.",
                        "consequence": "The work may continue.",
                        "machine_action": "accept",
                        "resolution_outcome": "accepted",
                        "is_default": true
                    }],
                    "context": {
                        "summary": "A bounded legacy choice was required.",
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
            }),
            "user_action_requests",
            "request_json",
        )?;
        let basis_json = canonical_json(
            &json!({
                "action_type": "choice",
                "coordinates": {
                    "task_id": "task_legacy",
                    "change_unit_id": null,
                    "scope_revision": 2,
                    "baseline_ref": null,
                    "created_at_state_version": 1,
                    "compatibility_status": "current"
                },
                "close_basis_revision": null,
                "result_refs": [],
                "residual_risk_ids": [],
                "sensitive_action_scope": null
            }),
            "user_action_requests",
            "basis_json",
        )?;
        state.execute(
            "INSERT INTO user_action_requests (
                project_id, user_action_request_id, task_id, action_kind,
                request_json, basis_json, required_for_json,
                requested_by_actor_source, source_method, source_idempotency_key,
                requested_at
             ) VALUES (
                'project_legacy', 'request_legacy', 'task_legacy',
                'product_decision', ?1, ?2, '[\"informational\"]',
                'agent_connection:connection_legacy', 'volicord.request_user_action',
                'legacy-product-decision', ?3
             )",
            params![request_json, basis_json, "2026-07-16T00:01:00Z"],
        )?;
        let resolution_json = canonical_json(
            &json!({
                "resolution_type": "choice",
                "selected_option_id": "accept",
                "machine_action": "accept",
                "resolution_outcome": "accepted",
                "note": null,
                "accepted_risk_ids": []
            }),
            "user_action_resolutions",
            "resolution_json",
        )?;
        state.execute(
            "INSERT INTO user_action_resolutions (
                project_id, user_action_resolution_id, user_action_request_id,
                action_kind, channel_kind, channel_submission_id, resolution_json,
                resolved_by_actor_source, resolved_verification_basis,
                resolved_assurance_level, resolved_at
             ) VALUES (
                'project_legacy', 'resolution_legacy', 'request_legacy',
                'product_decision', 'cli', 'legacy-submission', ?1,
                'local_user', 'cli_direct_user_channel', 'local_user_channel', ?2
             )",
            params![resolution_json, "2026-07-16T00:02:00Z"],
        )?;
        let attempt_scope = canonical_json(
            &json!({
                "task_id": "task_legacy",
                "change_unit_id": "change_legacy",
                "intended_operation": "edit legacy implementation",
                "intended_paths": ["src/"],
                "product_file_write_intended": true,
                "sensitive_categories": [],
                "baseline_ref": "baseline_legacy"
            }),
            "write_tickets",
            "attempt_scope_json",
        )?;
        state.execute(
            "INSERT INTO write_tickets (
                project_id, write_ticket_id, task_id, change_unit_id,
                basis_state_version, status, attempt_scope_json,
                created_by_actor_source, expires_at, created_at
             ) VALUES (
                'project_legacy', 'ticket_legacy', 'task_legacy', 'change_legacy',
                1, 'active', ?1, 'agent', ?2, ?3
             )",
            params![
                attempt_scope,
                "2026-07-16T02:00:00Z",
                "2026-07-16T00:00:00Z"
            ],
        )?;
        state.execute(
            "INSERT INTO write_tickets (
                project_id, write_ticket_id, task_id, change_unit_id,
                basis_state_version, status, attempt_scope_json,
                created_by_actor_source, expires_at, consumed_at, created_at
             ) VALUES (
                'project_legacy', 'ticket_consumed_legacy', 'task_legacy',
                'change_legacy', 2, 'consumed', ?1, 'agent', ?2, ?3, ?4
             )",
            params![
                attempt_scope,
                "2026-07-16T02:00:00Z",
                "2026-07-16T00:06:00Z",
                "2026-07-16T00:05:00Z"
            ],
        )?;
        state.execute(
            "INSERT INTO runs (
                project_id, run_id, task_id, change_unit_id, write_ticket_id,
                kind, status, summary_json, observed_changes_json,
                evidence_updates_json, write_ticket_effect_json, scope_revision,
                created_by_actor_source, started_at, completed_at, created_at
             ) VALUES (
                'project_legacy', 'run_consumed_legacy', 'task_legacy',
                'change_legacy', 'ticket_consumed_legacy', 'implementation',
                'recorded', '{\"summary\":\"legacy run\"}',
                '{\"changed_paths\":[\"src/lib.rs\"],\"product_file_write_observed\":true,\"sensitive_categories\":[],\"baseline_ref\":\"baseline_legacy\"}', '[]',
                '{\"status\":\"consumed\"}', 2,
                'agent_connection:connection_legacy', ?1, ?2, ?1
             )",
            params!["2026-07-16T00:05:00Z", "2026-07-16T00:06:00Z"],
        )?;
        state.execute(
            "UPDATE write_tickets
                SET consumed_by_run_id = 'run_consumed_legacy'
              WHERE write_ticket_id = 'ticket_consumed_legacy'",
            [],
        )?;
        for (ticket_id, basis_state_version, status, expires_at) in [
            (
                "ticket_expired_legacy",
                3_i64,
                "expired",
                "2026-07-16T00:04:00Z",
            ),
            (
                "ticket_stale_legacy",
                4_i64,
                "stale",
                "2026-07-16T03:00:00Z",
            ),
        ] {
            state.execute(
                "INSERT INTO write_tickets (
                    project_id, write_ticket_id, task_id, change_unit_id,
                    basis_state_version, status, attempt_scope_json,
                    created_by_actor_source, expires_at, created_at, metadata_json
                 ) VALUES (
                    'project_legacy', ?1, 'task_legacy', 'change_legacy',
                    ?2, ?3, ?4, 'agent', ?5, ?6,
                    '{\"legacy_marker\":\"preserved\"}'
                 )",
                params![
                    ticket_id,
                    basis_state_version,
                    status,
                    attempt_scope,
                    expires_at,
                    "2026-07-16T00:03:00Z"
                ],
            )?;
        }
        state.execute(
            "INSERT INTO evidence_claims (
                project_id, evidence_claim_id, task_id, statement, created_at
             ) VALUES (
                'project_legacy', 'claim_legacy', 'task_legacy',
                'The legacy implementation was verified.', ?1
             )",
            ["2026-07-16T00:06:00Z"],
        )?;
        let evidence_metadata = canonical_json(
            &json!({
                "recorded_by_run_id": "run_consumed_legacy",
                "invocation_verification_basis": "legacy_registered_tool",
                "producer_anchor": {
                    "producer_kind": "unverified_caller",
                    "producer_ref": null,
                    "output_artifact_refs": [],
                    "verification_basis": null
                },
                "relevance_assessment": {
                    "status": "unassessed",
                    "assessment_ref": null,
                    "assessed_by_actor_source": null
                }
            }),
            "evidence_observations",
            "metadata_json",
        )?;
        state.execute(
            "INSERT INTO evidence_observations (
                project_id, evidence_observation_id, task_id, change_unit_id,
                run_id, evidence_claim_id, source_kind, assurance_level,
                observed_by_actor_source, tool_name, tool_invocation_id,
                tool_metadata_json, input_refs_json, source_refs_json,
                output_artifact_refs_json, limitations_json, observed_at,
                recorded_at, metadata_json
             ) VALUES (
                'project_legacy', 'observation_legacy', 'task_legacy',
                'change_legacy', 'run_consumed_legacy', 'claim_legacy',
                'external_tool', 'external_tool_result',
                'agent_connection:connection_legacy', 'legacy-test-runner',
                'legacy-tool-invocation', '{\"exit_code\":0}', '[]',
                '[{\"source_kind\":\"user_context\",\"source\":{\"context_id\":\"legacy-message\"}}]',
                '[]', '[\"Legacy evidence has bounded provenance.\"]',
                ?1, ?2, ?3
             )",
            params![
                "2026-07-16T00:06:00Z",
                "2026-07-16T00:06:01Z",
                evidence_metadata
            ],
        )?;
        state.execute(
            "INSERT INTO agent_sessions (
                project_id, session_id, connection_internal_id, host_kind,
                guard_mode, started_at
             ) VALUES (
                'project_legacy', 'session_legacy', 'connection_legacy',
                'codex', 'detective', ?1
             )",
            ["2026-07-16T00:00:00Z"],
        )?;
        let guard_result = canonical_json(
            &json!({
                "tool": {
                    "classification": "mutating",
                    "changed_paths": [{
                        "raw": "src/lib.rs",
                        "normalized": "src/lib.rs",
                        "inside_repo": true
                    }]
                }
            }),
            "guard_events",
            "result_json",
        )?;
        state.execute(
            "INSERT INTO guard_events (
                project_id, guard_event_id, session_id, connection_internal_id,
                event_kind, decision, result_json, occurred_at
             ) VALUES (
                'project_legacy', 'guard_event_legacy', 'session_legacy',
                'connection_legacy', 'post_tool', 'warn', ?1, ?2
             )",
            params![guard_result, "2026-07-16T00:10:00Z"],
        )?;
        let heuristic_guard_result = canonical_json(
            &json!({"tool": {"classification": "mutating"}}),
            "guard_events",
            "result_json",
        )?;
        state.execute(
            "INSERT INTO guard_events (
                project_id, guard_event_id, session_id, connection_internal_id,
                guard_installation_id, event_kind, decision, result_json, occurred_at
             ) VALUES (
                'project_legacy', 'guard_event_heuristic_legacy', 'session_legacy',
                'connection_legacy', 'guard_legacy', 'post_tool', 'warn', ?1, ?2
             )",
            params![heuristic_guard_result, "2026-07-16T00:10:30Z"],
        )?;
        state.execute(
            "INSERT INTO unrecorded_changes (
                project_id, unrecorded_change_id, session_id, connection_internal_id,
                task_id, status, summary, detected_at
             ) VALUES (
                'project_legacy', 'unrecorded_legacy', 'session_legacy',
                'connection_legacy', 'task_legacy', 'unresolved',
                'legacy changed path', ?1
             )",
            ["2026-07-16T00:11:00Z"],
        )?;
        state.execute(
            "INSERT INTO session_watch_baselines (
                project_id, watch_baseline_id, session_id, connection_internal_id,
                status, scope_kind, repo_root, snapshot_algorithm, snapshot_digest,
                created_at, updated_at
             ) VALUES (
                'project_legacy', 'watch_baseline_legacy', 'session_legacy',
                'connection_legacy', 'active', 'repository', ?1, 'sha256',
                'digest_before', ?2, ?2
             )",
            params![
                path_text(&repo_root, "fixture_repo")?,
                "2026-07-16T00:00:00Z"
            ],
        )?;
        state.execute(
            "INSERT INTO session_watch_observations (
                project_id, watch_observation_id, watch_baseline_id, session_id,
                connection_internal_id, unrecorded_change_id, observation_status,
                observed_paths_json, snapshot_algorithm, snapshot_digest, observed_at, linked_at
             ) VALUES (
                'project_legacy', 'watch_observation_legacy', 'watch_baseline_legacy',
                'session_legacy', 'connection_legacy', 'unrecorded_legacy', 'linked',
                ?1, 'sha256', 'digest_after', ?2, ?2
             )",
            params!["[\"src/lib.rs\"]", "2026-07-16T00:11:00Z"],
        )?;
        let artifact_body = b"legacy evidence";
        fs::write(
            project_home.join(ARTIFACTS_DIR).join("body.txt"),
            artifact_body,
        )?;
        state.execute(
            "INSERT INTO artifacts (
                project_id, artifact_id, task_id, uri, body_path, sha256,
                size_bytes, content_type, integrity_status, redaction_state,
                status, created_at, updated_at
             ) VALUES (
                'project_legacy', 'artifact_legacy', 'task_legacy',
                'volicord://artifact/artifact_legacy', 'body.txt', ?1, ?2,
                'text/plain', 'verified', 'safe', 'available', ?3, ?3
             )",
            params![
                format!("{:x}", Sha256::digest(artifact_body)),
                i64::try_from(artifact_body.len())
                    .map_err(|_| upgrade_invariant("fixture artifact too large"))?,
                "2026-07-16T00:20:00Z"
            ],
        )?;
        drop(state);

        let diagnostics = Connection::open(runtime_home.join(DIAGNOSTICS_DB_FILE))?;
        diagnostics.execute_batch(DIAGNOSTICS_SCHEMA_V1_SQL)?;
        diagnostics.pragma_update(None, "user_version", 1)?;
        diagnostics.execute(
            "INSERT INTO diagnostic_sessions (
                session_id, transport, package_version, build_id, started_at, updated_at
             ) VALUES ('legacy-diagnostic-session', 'unknown', '0.8.0', 'legacy-build', ?1, ?1)",
            ["2026-07-16T00:00:00Z"],
        )?;
        drop(diagnostics);
        Ok(())
    }
}
