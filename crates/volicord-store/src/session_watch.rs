use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{self, Read},
    path::{Component, Path, PathBuf},
};

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    agent_connections::is_agent_connection_project_allowed,
    bootstrap::{project_record_for_execution, ProjectRecord},
    runtime_home::validate_runtime_home_product_repository,
    sqlite::{begin_immediate_transaction, open_project_state_database},
    StoreError, StoreResult,
};

/// Stable snapshot digest algorithm for session-level Product Repository watching.
pub const WATCH_SNAPSHOT_ALGORITHM: &str = "volicord_session_watch_snapshot_v1_sha256";

/// Default maximum regular-file bytes read for one snapshot entry.
pub const DEFAULT_MAX_FILE_HASH_BYTES: u64 = 10 * 1024 * 1024;

/// Session-level Product Repository watch availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionWatchStatus {
    Disabled,
    Active,
    Degraded,
    Unavailable,
}

impl SessionWatchStatus {
    /// Returns the stable storage value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Active => "active",
            Self::Degraded => "degraded",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Watch root scope represented by a baseline snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchScopeKind {
    Repository,
    PathSet,
}

impl WatchScopeKind {
    /// Returns the stable storage value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Repository => "repository",
            Self::PathSet => "path_set",
        }
    }
}

/// Stored watch-observation lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchObservationStatus {
    Unresolved,
    Linked,
}

impl WatchObservationStatus {
    /// Returns the stable storage value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unresolved => "unresolved",
            Self::Linked => "linked",
        }
    }
}

/// File-level change kind derived from two snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WatchPathChangeKind {
    Added,
    Modified,
    Deleted,
}

impl WatchPathChangeKind {
    /// Returns the stable JSON value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
        }
    }
}

/// Deterministic scanner options for one Product Repository snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchSnapshotOptions {
    pub watch_paths: Vec<PathBuf>,
    pub excluded_paths: Vec<PathBuf>,
    pub max_file_size_bytes: u64,
}

impl Default for WatchSnapshotOptions {
    fn default() -> Self {
        Self {
            watch_paths: Vec::new(),
            excluded_paths: Vec::new(),
            max_file_size_bytes: DEFAULT_MAX_FILE_HASH_BYTES,
        }
    }
}

/// One file or skipped path entry in a snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchSnapshotEntry {
    pub path: String,
    pub kind: String,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
    pub skip_reason: Option<String>,
}

impl WatchSnapshotEntry {
    fn file(path: String, size_bytes: u64, sha256: String) -> Self {
        Self {
            path,
            kind: "file".to_owned(),
            sha256: Some(sha256),
            size_bytes: Some(size_bytes),
            skip_reason: None,
        }
    }

    fn skipped(path: String, size_bytes: Option<u64>, reason: &'static str) -> Self {
        Self {
            path,
            kind: "skipped".to_owned(),
            sha256: None,
            size_bytes,
            skip_reason: Some(reason.to_owned()),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "path": self.path,
            "kind": self.kind,
            "sha256": self.sha256,
            "size_bytes": self.size_bytes,
            "skip_reason": self.skip_reason,
        })
    }
}

/// Deterministic Product Repository state snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchSnapshot {
    pub repo_root: PathBuf,
    pub scope_kind: WatchScopeKind,
    pub watched_paths: Vec<String>,
    pub excluded_paths: Vec<String>,
    pub algorithm: String,
    pub digest: String,
    pub entries: Vec<WatchSnapshotEntry>,
}

impl WatchSnapshot {
    /// Returns snapshot entries as stable JSON array text.
    pub fn entries_json(&self) -> String {
        json_array_text(self.entries.iter().map(WatchSnapshotEntry::to_json))
    }

    /// Returns watched paths as stable JSON array text.
    pub fn watched_paths_json(&self) -> String {
        json_array_text(self.watched_paths.iter().map(|path| json!(path)))
    }

    /// Returns effective excluded paths as stable JSON array text.
    pub fn exclusions_json(&self) -> String {
        json_array_text(self.excluded_paths.iter().map(|path| json!(path)))
    }
}

/// One changed path between two snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchPathChange {
    pub path: String,
    pub change_kind: WatchPathChangeKind,
    pub before: Option<WatchSnapshotEntry>,
    pub after: Option<WatchSnapshotEntry>,
}

/// Deterministic diff between two snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchSnapshotDiff {
    pub changes: Vec<WatchPathChange>,
}

impl WatchSnapshotDiff {
    /// Returns changed paths as stable JSON array text.
    pub fn observed_paths_json(&self) -> String {
        json_array_text(self.changes.iter().map(|change| json!(change.path)))
    }

    /// Returns compact per-path change details as stable JSON object text.
    pub fn change_summary_json(&self) -> String {
        json!({
            "changes": self.changes.iter().map(|change| {
                json!({
                    "path": change.path,
                    "change_kind": change.change_kind.as_str(),
                    "before": change.before.as_ref().map(WatchSnapshotEntry::to_json),
                    "after": change.after.as_ref().map(WatchSnapshotEntry::to_json),
                })
            }).collect::<Vec<_>>()
        })
        .to_string()
    }
}

/// Storage input for creating one session watch baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchBaselineCreate {
    pub watch_baseline_id: String,
    pub session_id: String,
    pub connection_internal_id: String,
    pub guard_installation_id: Option<String>,
    pub status: SessionWatchStatus,
    pub snapshot: WatchSnapshot,
    pub created_at: String,
    pub metadata_json: String,
}

/// Session watch baseline row stored in project `state.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchBaselineRecord {
    pub project_id: String,
    pub watch_baseline_id: String,
    pub session_id: String,
    pub connection_internal_id: String,
    pub guard_installation_id: Option<String>,
    pub status: String,
    pub scope_kind: String,
    pub repo_root: String,
    pub watched_paths_json: String,
    pub exclusions_json: String,
    pub snapshot_algorithm: String,
    pub snapshot_digest: String,
    pub snapshot_entries_json: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Storage input for updating a watch baseline status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchStatusUpdate {
    pub status: SessionWatchStatus,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Storage input for recording one watch observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchObservationInsert {
    pub watch_observation_id: String,
    pub watch_baseline_id: String,
    pub expected_write_id: Option<String>,
    pub snapshot: WatchSnapshot,
    pub diff: WatchSnapshotDiff,
    pub observed_at: String,
    pub metadata_json: String,
}

/// Session watch observation row stored in project `state.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchObservationRecord {
    pub project_id: String,
    pub watch_observation_id: String,
    pub watch_baseline_id: String,
    pub session_id: String,
    pub connection_internal_id: String,
    pub expected_write_id: Option<String>,
    pub unrecorded_change_id: Option<String>,
    pub observation_status: String,
    pub observed_paths_json: String,
    pub change_summary_json: String,
    pub snapshot_algorithm: String,
    pub snapshot_digest: String,
    pub snapshot_entries_json: String,
    pub observed_at: String,
    pub linked_at: Option<String>,
    pub metadata_json: String,
}

/// Creates a deterministic Product Repository snapshot without executing repository code.
pub fn snapshot_product_repository(
    runtime_home: impl AsRef<Path>,
    repo_root: impl AsRef<Path>,
    options: WatchSnapshotOptions,
) -> StoreResult<WatchSnapshot> {
    let path_validation =
        validate_runtime_home_product_repository(runtime_home.as_ref(), repo_root.as_ref())
            .map_err(|error| StoreError::InvalidInput {
                detail: error.to_string(),
            })?;
    let repo_root = path_validation.repo_root;
    let watch_paths = normalize_relative_path_set("watch_paths", &options.watch_paths)?;
    let effective_exclusions = effective_excluded_paths(&options.excluded_paths)?;
    let mut entries = Vec::new();

    if watch_paths.is_empty() {
        scan_path(
            &repo_root,
            Path::new(""),
            &effective_exclusions,
            options.max_file_size_bytes,
            &mut entries,
        )?;
    } else {
        for relative in &watch_paths {
            scan_path(
                &repo_root,
                relative,
                &effective_exclusions,
                options.max_file_size_bytes,
                &mut entries,
            )?;
        }
    }

    entries.sort_by(|left, right| left.path.cmp(&right.path));
    let watched_paths = path_texts(&watch_paths)?;
    let excluded_paths = path_texts(&effective_exclusions)?;
    let scope_kind = if watch_paths.is_empty() {
        WatchScopeKind::Repository
    } else {
        WatchScopeKind::PathSet
    };
    let digest = snapshot_digest(scope_kind, &watched_paths, &excluded_paths, &entries);

    Ok(WatchSnapshot {
        repo_root,
        scope_kind,
        watched_paths,
        excluded_paths,
        algorithm: WATCH_SNAPSHOT_ALGORITHM.to_owned(),
        digest,
        entries,
    })
}

/// Compares two deterministic snapshots and returns added, modified, and deleted paths.
pub fn compare_watch_snapshots(
    baseline: &WatchSnapshot,
    current: &WatchSnapshot,
) -> WatchSnapshotDiff {
    let baseline_entries = baseline
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let current_entries = current
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let paths = baseline_entries
        .keys()
        .chain(current_entries.keys())
        .copied()
        .collect::<BTreeSet<_>>();

    let changes = paths
        .into_iter()
        .filter_map(
            |path| match (baseline_entries.get(path), current_entries.get(path)) {
                (None, Some(after)) => Some(WatchPathChange {
                    path: path.to_owned(),
                    change_kind: WatchPathChangeKind::Added,
                    before: None,
                    after: Some((*after).clone()),
                }),
                (Some(before), None) => Some(WatchPathChange {
                    path: path.to_owned(),
                    change_kind: WatchPathChangeKind::Deleted,
                    before: Some((*before).clone()),
                    after: None,
                }),
                (Some(before), Some(after)) if before != after => Some(WatchPathChange {
                    path: path.to_owned(),
                    change_kind: WatchPathChangeKind::Modified,
                    before: Some((*before).clone()),
                    after: Some((*after).clone()),
                }),
                _ => None,
            },
        )
        .collect();

    WatchSnapshotDiff { changes }
}

/// Inserts one watch baseline for an existing project session.
pub fn create_watch_baseline(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    input: WatchBaselineCreate,
) -> StoreResult<WatchBaselineRecord> {
    validate_watch_baseline_create(&input)?;
    let mut project = open_watch_project(runtime_home, project_id, &input.connection_internal_id)?;
    validate_session_scope(
        &project.conn,
        &project.project.project_id,
        &input.session_id,
        &input.connection_internal_id,
    )?;
    validate_snapshot_repo_root(&input.snapshot, &project.project)?;

    let repo_root = path_to_text(
        "session_watch_baselines.repo_root",
        &input.snapshot.repo_root,
    )?;
    let watched_paths_json = input.snapshot.watched_paths_json();
    let exclusions_json = input.snapshot.exclusions_json();
    let snapshot_entries_json = input.snapshot.entries_json();
    let tx = begin_immediate_transaction(&mut project.conn)?;
    tx.execute(
        "INSERT INTO session_watch_baselines (
            project_id,
            watch_baseline_id,
            session_id,
            connection_internal_id,
            guard_installation_id,
            status,
            scope_kind,
            repo_root,
            watched_paths_json,
            exclusions_json,
            snapshot_algorithm,
            snapshot_digest,
            snapshot_entries_json,
            created_at,
            updated_at,
            metadata_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14, ?15)",
        params![
            project.project.project_id,
            input.watch_baseline_id,
            input.session_id,
            input.connection_internal_id,
            input.guard_installation_id,
            input.status.as_str(),
            input.snapshot.scope_kind.as_str(),
            repo_root,
            watched_paths_json,
            exclusions_json,
            input.snapshot.algorithm,
            input.snapshot.digest,
            snapshot_entries_json,
            input.created_at,
            input.metadata_json,
        ],
    )?;
    tx.commit()?;

    watch_baseline_by_conn(
        &project.conn,
        &project.project.project_id,
        &input.watch_baseline_id,
    )
}

/// Reads one watch baseline by id.
pub fn watch_baseline(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    watch_baseline_id: &str,
) -> StoreResult<Option<WatchBaselineRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("watch_baseline_id", watch_baseline_id)?;
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(None);
    };
    watch_baseline_from_conn(
        &project.conn,
        &project.project.project_id,
        watch_baseline_id,
    )
}

/// Reads the most recent watch baseline for one project session.
pub fn latest_watch_baseline_for_session(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    session_id: &str,
) -> StoreResult<Option<WatchBaselineRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("session_id", session_id)?;
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(None);
    };
    project
        .conn
        .query_row(
            "SELECT
                project_id,
                watch_baseline_id,
                session_id,
                connection_internal_id,
                guard_installation_id,
                status,
                scope_kind,
                repo_root,
                watched_paths_json,
                exclusions_json,
                snapshot_algorithm,
                snapshot_digest,
                snapshot_entries_json,
                created_at,
                updated_at,
                metadata_json
             FROM session_watch_baselines
            WHERE project_id = ?1
              AND session_id = ?2
            ORDER BY updated_at DESC, watch_baseline_id DESC
            LIMIT 1",
            params![project.project.project_id, session_id],
            watch_baseline_from_row,
        )
        .optional()
        .map_err(StoreError::from)
}

/// Reads the most recent watch baseline for one project Agent Connection.
pub fn latest_watch_baseline_for_connection(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    connection_internal_id: &str,
) -> StoreResult<Option<WatchBaselineRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("connection_internal_id", connection_internal_id)?;
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(None);
    };
    project
        .conn
        .query_row(
            "SELECT
                project_id,
                watch_baseline_id,
                session_id,
                connection_internal_id,
                guard_installation_id,
                status,
                scope_kind,
                repo_root,
                watched_paths_json,
                exclusions_json,
                snapshot_algorithm,
                snapshot_digest,
                snapshot_entries_json,
                created_at,
                updated_at,
                metadata_json
             FROM session_watch_baselines
            WHERE project_id = ?1
              AND connection_internal_id = ?2
            ORDER BY updated_at DESC, watch_baseline_id DESC
            LIMIT 1",
            params![project.project.project_id, connection_internal_id],
            watch_baseline_from_row,
        )
        .optional()
        .map_err(StoreError::from)
}

/// Updates the availability status for one watch baseline.
pub fn update_watch_status(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    watch_baseline_id: &str,
    input: WatchStatusUpdate,
) -> StoreResult<WatchBaselineRecord> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("watch_baseline_id", watch_baseline_id)?;
    validate_timestamp_text("updated_at", &input.updated_at)?;
    validate_json_object(
        "session_watch_baselines.metadata_json",
        &input.metadata_json,
    )?;
    let mut project = open_project_for_required_read(runtime_home, project_id)?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    let changed = tx.execute(
        "UPDATE session_watch_baselines
            SET status = ?3,
                updated_at = ?4,
                metadata_json = ?5
          WHERE project_id = ?1
            AND watch_baseline_id = ?2",
        params![
            project.project.project_id,
            watch_baseline_id,
            input.status.as_str(),
            input.updated_at,
            input.metadata_json,
        ],
    )?;
    tx.commit()?;
    if changed == 0 {
        return Err(StoreError::NotFound {
            entity: "session_watch_baseline",
            id: watch_baseline_id.to_owned(),
        });
    }

    watch_baseline_by_conn(
        &project.conn,
        &project.project.project_id,
        watch_baseline_id,
    )
}

/// Inserts one unresolved watch observation for an existing baseline.
pub fn record_watch_observation(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    input: WatchObservationInsert,
) -> StoreResult<WatchObservationRecord> {
    validate_watch_observation_insert(&input)?;
    let mut project = open_project_for_required_read(runtime_home, project_id)?;
    let baseline = watch_baseline_from_conn(
        &project.conn,
        &project.project.project_id,
        &input.watch_baseline_id,
    )?
    .ok_or_else(|| StoreError::NotFound {
        entity: "session_watch_baseline",
        id: input.watch_baseline_id.clone(),
    })?;
    let snapshot_repo_root = path_to_text(
        "session_watch_observations.repo_root",
        &input.snapshot.repo_root,
    )?;
    if snapshot_repo_root != baseline.repo_root {
        return Err(StoreError::InvalidInput {
            detail: "watch observation snapshot repo_root must match the baseline repo_root"
                .to_owned(),
        });
    }
    if let Some(expected_write_id) = input.expected_write_id.as_deref() {
        validate_expected_write_scope(
            &project.conn,
            &project.project.project_id,
            expected_write_id,
            &baseline.session_id,
            &baseline.connection_internal_id,
        )?;
    }

    let observed_paths_json = input.diff.observed_paths_json();
    let change_summary_json = input.diff.change_summary_json();
    let snapshot_entries_json = input.snapshot.entries_json();
    let tx = begin_immediate_transaction(&mut project.conn)?;
    tx.execute(
        "INSERT INTO session_watch_observations (
            project_id,
            watch_observation_id,
            watch_baseline_id,
            session_id,
            connection_internal_id,
            expected_write_id,
            observation_status,
            observed_paths_json,
            change_summary_json,
            snapshot_algorithm,
            snapshot_digest,
            snapshot_entries_json,
            observed_at,
            metadata_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'unresolved', ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            project.project.project_id,
            input.watch_observation_id,
            input.watch_baseline_id,
            baseline.session_id,
            baseline.connection_internal_id,
            input.expected_write_id,
            observed_paths_json,
            change_summary_json,
            input.snapshot.algorithm,
            input.snapshot.digest,
            snapshot_entries_json,
            input.observed_at,
            input.metadata_json,
        ],
    )?;
    tx.commit()?;

    watch_observation_by_conn(
        &project.conn,
        &project.project.project_id,
        &input.watch_observation_id,
    )
}

/// Reads one watch observation by id.
pub fn watch_observation(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    watch_observation_id: &str,
) -> StoreResult<Option<WatchObservationRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("watch_observation_id", watch_observation_id)?;
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(None);
    };
    watch_observation_from_conn(
        &project.conn,
        &project.project.project_id,
        watch_observation_id,
    )
}

/// Reads one watch observation for a baseline and resulting snapshot digest.
pub fn watch_observation_for_baseline_digest(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    watch_baseline_id: &str,
    snapshot_digest: &str,
) -> StoreResult<Option<WatchObservationRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("watch_baseline_id", watch_baseline_id)?;
    validate_lowercase_sha256(
        "session_watch_observations.snapshot_digest",
        snapshot_digest,
    )?;
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(None);
    };
    project
        .conn
        .query_row(
            "SELECT
                project_id,
                watch_observation_id,
                watch_baseline_id,
                session_id,
                connection_internal_id,
                expected_write_id,
                unrecorded_change_id,
                observation_status,
                observed_paths_json,
                change_summary_json,
                snapshot_algorithm,
                snapshot_digest,
                snapshot_entries_json,
                observed_at,
                linked_at,
                metadata_json
             FROM session_watch_observations
            WHERE project_id = ?1
              AND watch_baseline_id = ?2
              AND snapshot_digest = ?3
            ORDER BY observed_at DESC, watch_observation_id DESC
            LIMIT 1",
            params![
                project.project.project_id,
                watch_baseline_id,
                snapshot_digest
            ],
            watch_observation_from_row,
        )
        .optional()
        .map_err(StoreError::from)
}

/// Lists watch observations linked to one unrecorded-change row.
pub fn watch_observations_for_unrecorded_change(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    unrecorded_change_id: &str,
) -> StoreResult<Vec<WatchObservationRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("unrecorded_change_id", unrecorded_change_id)?;
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(Vec::new());
    };
    let mut stmt = project.conn.prepare(
        "SELECT
            project_id,
            watch_observation_id,
            watch_baseline_id,
            session_id,
            connection_internal_id,
            expected_write_id,
            unrecorded_change_id,
            observation_status,
            observed_paths_json,
            change_summary_json,
            snapshot_algorithm,
            snapshot_digest,
            snapshot_entries_json,
            observed_at,
            linked_at,
            metadata_json
         FROM session_watch_observations
        WHERE project_id = ?1
          AND unrecorded_change_id = ?2
        ORDER BY observed_at DESC, watch_observation_id DESC",
    )?;
    let rows = stmt.query_map(
        params![project.project.project_id, unrecorded_change_id],
        watch_observation_from_row,
    )?;
    collect_rows(rows)
}

/// Lists unresolved watch observations for one project session.
pub fn list_unresolved_watch_observations(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    session_id: &str,
) -> StoreResult<Vec<WatchObservationRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("session_id", session_id)?;
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(Vec::new());
    };
    let mut stmt = project.conn.prepare(
        "SELECT
            project_id,
            watch_observation_id,
            watch_baseline_id,
            session_id,
            connection_internal_id,
            expected_write_id,
            unrecorded_change_id,
            observation_status,
            observed_paths_json,
            change_summary_json,
            snapshot_algorithm,
            snapshot_digest,
            snapshot_entries_json,
            observed_at,
            linked_at,
            metadata_json
         FROM session_watch_observations
        WHERE project_id = ?1
          AND session_id = ?2
          AND observation_status = 'unresolved'
        ORDER BY observed_at, watch_observation_id",
    )?;
    let rows = stmt.query_map(
        params![project.project.project_id, session_id],
        watch_observation_from_row,
    )?;
    collect_rows(rows)
}

/// Links one unresolved watch observation to an existing unresolved unrecorded-change row.
pub fn link_watch_observation_to_unrecorded_change(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    watch_observation_id: &str,
    unrecorded_change_id: &str,
    linked_at: &str,
) -> StoreResult<WatchObservationRecord> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("watch_observation_id", watch_observation_id)?;
    validate_identifier("unrecorded_change_id", unrecorded_change_id)?;
    validate_timestamp_text("linked_at", linked_at)?;
    let mut project = open_project_for_required_read(runtime_home, project_id)?;
    let observation = watch_observation_from_conn(
        &project.conn,
        &project.project.project_id,
        watch_observation_id,
    )?
    .ok_or_else(|| StoreError::NotFound {
        entity: "session_watch_observation",
        id: watch_observation_id.to_owned(),
    })?;
    if observation.observation_status != WatchObservationStatus::Unresolved.as_str() {
        return Err(StoreError::Conflict {
            entity: "session_watch_observation",
            id: watch_observation_id.to_owned(),
            detail: "watch observation is already linked".to_owned(),
        });
    }
    validate_unrecorded_change_scope(
        &project.conn,
        &project.project.project_id,
        unrecorded_change_id,
        &observation.session_id,
        &observation.connection_internal_id,
    )?;

    let tx = begin_immediate_transaction(&mut project.conn)?;
    let changed = tx.execute(
        "UPDATE session_watch_observations
            SET observation_status = 'linked',
                unrecorded_change_id = ?3,
                linked_at = ?4
          WHERE project_id = ?1
            AND watch_observation_id = ?2
            AND observation_status = 'unresolved'",
        params![
            project.project.project_id,
            watch_observation_id,
            unrecorded_change_id,
            linked_at,
        ],
    )?;
    tx.commit()?;
    if changed == 0 {
        return Err(StoreError::Conflict {
            entity: "session_watch_observation",
            id: watch_observation_id.to_owned(),
            detail: "watch observation could not be linked from unresolved state".to_owned(),
        });
    }

    watch_observation_by_conn(
        &project.conn,
        &project.project.project_id,
        watch_observation_id,
    )
}

struct OpenWatchProject {
    project: ProjectRecord,
    conn: Connection,
}

fn open_watch_project(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    connection_internal_id: &str,
) -> StoreResult<OpenWatchProject> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("connection_internal_id", connection_internal_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    if !is_agent_connection_project_allowed(&runtime_home, connection_internal_id, project_id)? {
        return Err(StoreError::NotFound {
            entity: "connection_project",
            id: format!("{connection_internal_id}/{project_id}"),
        });
    }
    open_project_for_required_read(runtime_home, project_id)
}

fn open_project_for_read(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<Option<OpenWatchProject>> {
    let Some(project) = project_record_for_execution(runtime_home, project_id)? else {
        return Ok(None);
    };
    let conn = open_project_state_database(&project.state_db_path)?;
    Ok(Some(OpenWatchProject { project, conn }))
}

fn open_project_for_required_read(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<OpenWatchProject> {
    open_project_for_read(runtime_home, project_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "project",
        id: project_id.to_owned(),
    })
}

fn scan_path(
    repo_root: &Path,
    relative: &Path,
    excluded_paths: &[PathBuf],
    max_file_size_bytes: u64,
    entries: &mut Vec<WatchSnapshotEntry>,
) -> StoreResult<()> {
    if !relative.as_os_str().is_empty() && is_excluded(relative, excluded_paths) {
        return Ok(());
    }

    let absolute = repo_root.join(relative);
    let metadata = match fs::symlink_metadata(&absolute) {
        Ok(metadata) => metadata,
        Err(error) if missing_path_error(&error) => return Ok(()),
        Err(error) if relative.as_os_str().is_empty() => return Err(StoreError::Io(error)),
        Err(_) => {
            entries.push(WatchSnapshotEntry::skipped(
                relative_path_text(relative)?,
                None,
                "metadata_unavailable",
            ));
            return Ok(());
        }
    };
    let file_type = metadata.file_type();

    if file_type.is_symlink() {
        entries.push(WatchSnapshotEntry::skipped(
            relative_path_text(relative)?,
            Some(metadata.len()),
            "symlink",
        ));
    } else if metadata.is_dir() {
        scan_directory(
            repo_root,
            relative,
            excluded_paths,
            max_file_size_bytes,
            entries,
        )?;
    } else if metadata.is_file() {
        let path = relative_path_text(relative)?;
        if metadata.len() > max_file_size_bytes {
            entries.push(WatchSnapshotEntry::skipped(
                path,
                Some(metadata.len()),
                "size_limit",
            ));
        } else {
            match hash_file_limited(&absolute, max_file_size_bytes) {
                Ok(FileHashResult::Hashed { sha256, size_bytes }) => {
                    entries.push(WatchSnapshotEntry::file(path, size_bytes, sha256));
                }
                Ok(FileHashResult::SizeLimit { size_bytes }) => {
                    entries.push(WatchSnapshotEntry::skipped(
                        path,
                        Some(size_bytes),
                        "size_limit",
                    ));
                }
                Err(_) => {
                    entries.push(WatchSnapshotEntry::skipped(
                        path,
                        Some(metadata.len()),
                        "read_unavailable",
                    ));
                }
            }
        }
    } else {
        entries.push(WatchSnapshotEntry::skipped(
            relative_path_text(relative)?,
            Some(metadata.len()),
            "unsupported_file_type",
        ));
    }

    Ok(())
}

fn scan_directory(
    repo_root: &Path,
    relative: &Path,
    excluded_paths: &[PathBuf],
    max_file_size_bytes: u64,
    entries: &mut Vec<WatchSnapshotEntry>,
) -> StoreResult<()> {
    let absolute = repo_root.join(relative);
    let read_dir = match fs::read_dir(&absolute) {
        Ok(read_dir) => read_dir,
        Err(error) if relative.as_os_str().is_empty() => return Err(StoreError::Io(error)),
        Err(_) => {
            entries.push(WatchSnapshotEntry::skipped(
                relative_path_text(relative)?,
                None,
                "directory_unavailable",
            ));
            return Ok(());
        }
    };
    let mut children = Vec::new();
    for entry in read_dir {
        let entry = entry?;
        let child_relative = if relative.as_os_str().is_empty() {
            PathBuf::from(entry.file_name())
        } else {
            relative.join(entry.file_name())
        };
        if is_excluded(&child_relative, excluded_paths) {
            continue;
        }
        let child_key = relative_path_text(&child_relative)?;
        children.push((child_key, child_relative));
    }
    children.sort_by(|left, right| left.0.cmp(&right.0));
    for (_, child_relative) in children {
        scan_path(
            repo_root,
            &child_relative,
            excluded_paths,
            max_file_size_bytes,
            entries,
        )?;
    }
    Ok(())
}

enum FileHashResult {
    Hashed { sha256: String, size_bytes: u64 },
    SizeLimit { size_bytes: u64 },
}

fn hash_file_limited(path: &Path, max_file_size_bytes: u64) -> io::Result<FileHashResult> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total += u64::try_from(read).expect("buffer read size fits u64");
        if total > max_file_size_bytes {
            return Ok(FileHashResult::SizeLimit { size_bytes: total });
        }
        hasher.update(&buffer[..read]);
    }
    Ok(FileHashResult::Hashed {
        sha256: lowercase_sha256_digest(&hasher.finalize()),
        size_bytes: total,
    })
}

fn compare_relative_paths(left: &Path, right: &Path) -> std::cmp::Ordering {
    relative_path_text(left)
        .unwrap_or_else(|_| left.to_string_lossy().into_owned())
        .cmp(&relative_path_text(right).unwrap_or_else(|_| right.to_string_lossy().into_owned()))
}

fn normalize_relative_path_set(
    field: &'static str,
    paths: &[PathBuf],
) -> StoreResult<Vec<PathBuf>> {
    let mut values = paths
        .iter()
        .map(|path| normalize_relative_path(field, path))
        .collect::<StoreResult<Vec<_>>>()?;
    values.sort_by(|left, right| compare_relative_paths(left, right));
    values.dedup();
    Ok(values)
}

fn effective_excluded_paths(configured_paths: &[PathBuf]) -> StoreResult<Vec<PathBuf>> {
    let mut values = default_excluded_paths();
    values.extend(normalize_relative_path_set(
        "excluded_paths",
        configured_paths,
    )?);
    values.sort_by(|left, right| compare_relative_paths(left, right));
    values.dedup();
    Ok(values)
}

fn default_excluded_paths() -> Vec<PathBuf> {
    [".git", ".hg", ".svn", ".jj", ".volicord"]
        .iter()
        .map(PathBuf::from)
        .collect()
}

fn normalize_relative_path(field: &'static str, path: &Path) -> StoreResult<PathBuf> {
    if path.as_os_str().is_empty() {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} path must not be empty"),
        });
    }
    if path.is_absolute() {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} path must be relative to the Product Repository"),
        });
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => {
                let value = value.to_str().ok_or_else(|| StoreError::InvalidInput {
                    detail: format!("{field} path must be valid UTF-8"),
                })?;
                if value.contains('\0') {
                    return Err(StoreError::InvalidInput {
                        detail: format!("{field} path must not contain NUL bytes"),
                    });
                }
                normalized.push(value);
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(StoreError::InvalidInput {
                    detail: format!(
                        "{field} path must not be absolute or contain parent traversal"
                    ),
                });
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        Err(StoreError::InvalidInput {
            detail: format!("{field} path must not be empty"),
        })
    } else {
        Ok(normalized)
    }
}

fn is_excluded(relative: &Path, excluded_paths: &[PathBuf]) -> bool {
    excluded_paths
        .iter()
        .any(|excluded| relative == excluded || relative.starts_with(excluded))
}

fn relative_path_text(path: &Path) -> StoreResult<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => {
                let value = value.to_str().ok_or_else(|| StoreError::InvalidInput {
                    detail: "snapshot path must be valid UTF-8".to_owned(),
                })?;
                if value.contains('\0') {
                    return Err(StoreError::InvalidInput {
                        detail: "snapshot path must not contain NUL bytes".to_owned(),
                    });
                }
                parts.push(value.to_owned());
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(StoreError::InvalidInput {
                    detail: "snapshot path must stay relative to the Product Repository".to_owned(),
                });
            }
        }
    }
    if parts.is_empty() {
        Err(StoreError::InvalidInput {
            detail: "snapshot path must not be empty".to_owned(),
        })
    } else {
        Ok(parts.join("/"))
    }
}

fn path_texts(paths: &[PathBuf]) -> StoreResult<Vec<String>> {
    paths
        .iter()
        .map(|path| relative_path_text(path))
        .collect::<StoreResult<Vec<_>>>()
}

fn snapshot_digest(
    scope_kind: WatchScopeKind,
    watched_paths: &[String],
    excluded_paths: &[String],
    entries: &[WatchSnapshotEntry],
) -> String {
    let mut hasher = Sha256::new();
    update_framed(&mut hasher, WATCH_SNAPSHOT_ALGORITHM.as_bytes());
    update_framed(&mut hasher, scope_kind.as_str().as_bytes());
    for path in watched_paths {
        update_framed(&mut hasher, b"watch_path");
        update_framed(&mut hasher, path.as_bytes());
    }
    for path in excluded_paths {
        update_framed(&mut hasher, b"excluded_path");
        update_framed(&mut hasher, path.as_bytes());
    }
    for entry in entries {
        update_framed(&mut hasher, b"entry");
        update_framed(&mut hasher, entry.path.as_bytes());
        update_framed(&mut hasher, entry.kind.as_bytes());
        update_framed(
            &mut hasher,
            entry.sha256.as_deref().unwrap_or_default().as_bytes(),
        );
        update_framed(
            &mut hasher,
            entry
                .size_bytes
                .map(|value| value.to_string())
                .unwrap_or_default()
                .as_bytes(),
        );
        update_framed(
            &mut hasher,
            entry.skip_reason.as_deref().unwrap_or_default().as_bytes(),
        );
    }
    lowercase_sha256_digest(&hasher.finalize())
}

fn update_framed(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn lowercase_sha256_digest(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        value.push_str(&format!("{byte:02x}"));
    }
    value
}

fn missing_path_error(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
    )
}

fn json_array_text(values: impl Iterator<Item = Value>) -> String {
    Value::Array(values.collect()).to_string()
}

fn validate_watch_baseline_create(input: &WatchBaselineCreate) -> StoreResult<()> {
    validate_identifier("watch_baseline_id", &input.watch_baseline_id)?;
    validate_identifier("session_id", &input.session_id)?;
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    if let Some(guard_installation_id) = &input.guard_installation_id {
        validate_identifier("guard_installation_id", guard_installation_id)?;
    }
    validate_snapshot("session_watch_baselines.snapshot", &input.snapshot)?;
    validate_timestamp_text("created_at", &input.created_at)?;
    validate_json_object(
        "session_watch_baselines.metadata_json",
        &input.metadata_json,
    )
}

fn validate_watch_observation_insert(input: &WatchObservationInsert) -> StoreResult<()> {
    validate_identifier("watch_observation_id", &input.watch_observation_id)?;
    validate_identifier("watch_baseline_id", &input.watch_baseline_id)?;
    if let Some(expected_write_id) = &input.expected_write_id {
        validate_identifier("expected_write_id", expected_write_id)?;
    }
    validate_snapshot("session_watch_observations.snapshot", &input.snapshot)?;
    validate_timestamp_text("observed_at", &input.observed_at)?;
    validate_json_object(
        "session_watch_observations.metadata_json",
        &input.metadata_json,
    )
}

fn validate_snapshot(field: &'static str, snapshot: &WatchSnapshot) -> StoreResult<()> {
    if snapshot.algorithm != WATCH_SNAPSHOT_ALGORITHM {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} algorithm must be {WATCH_SNAPSHOT_ALGORITHM}"),
        });
    }
    validate_lowercase_sha256(&format!("{field}.digest"), &snapshot.digest)?;
    validate_json_array(
        "snapshot.watched_paths_json",
        &snapshot.watched_paths_json(),
    )?;
    validate_json_array("snapshot.exclusions_json", &snapshot.exclusions_json())?;
    validate_json_array("snapshot.snapshot_entries_json", &snapshot.entries_json())
}

fn validate_snapshot_repo_root(
    snapshot: &WatchSnapshot,
    project: &ProjectRecord,
) -> StoreResult<()> {
    if snapshot.repo_root == project.repo_root {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail:
                "watch baseline snapshot repo_root must match the registered Product Repository"
                    .to_owned(),
        })
    }
}

fn validate_session_scope(
    conn: &Connection,
    project_id: &str,
    session_id: &str,
    connection_internal_id: &str,
) -> StoreResult<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
           FROM agent_sessions
          WHERE project_id = ?1
            AND session_id = ?2
            AND connection_internal_id = ?3",
        params![project_id, session_id, connection_internal_id],
        |row| row.get(0),
    )?;
    if count == 1 {
        Ok(())
    } else {
        Err(StoreError::NotFound {
            entity: "agent_session",
            id: session_id.to_owned(),
        })
    }
}

fn validate_expected_write_scope(
    conn: &Connection,
    project_id: &str,
    expected_write_id: &str,
    session_id: &str,
    connection_internal_id: &str,
) -> StoreResult<()> {
    let row = conn
        .query_row(
            "SELECT session_id, connection_internal_id
               FROM expected_writes
              WHERE project_id = ?1
                AND expected_write_id = ?2",
            params![project_id, expected_write_id],
            |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "expected_write",
            id: expected_write_id.to_owned(),
        })?;
    let (expected_session_id, expected_connection_id) = row;
    if expected_connection_id != connection_internal_id {
        return Err(StoreError::Conflict {
            entity: "expected_write",
            id: expected_write_id.to_owned(),
            detail: "expected write belongs to a different Agent Connection".to_owned(),
        });
    }
    if expected_session_id
        .as_deref()
        .is_some_and(|expected| expected != session_id)
    {
        return Err(StoreError::Conflict {
            entity: "expected_write",
            id: expected_write_id.to_owned(),
            detail: "expected write belongs to a different session".to_owned(),
        });
    }
    Ok(())
}

fn validate_unrecorded_change_scope(
    conn: &Connection,
    project_id: &str,
    unrecorded_change_id: &str,
    session_id: &str,
    connection_internal_id: &str,
) -> StoreResult<()> {
    let row = conn
        .query_row(
            "SELECT session_id, connection_internal_id, status
               FROM unrecorded_changes
              WHERE project_id = ?1
                AND unrecorded_change_id = ?2",
            params![project_id, unrecorded_change_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "unrecorded_change",
            id: unrecorded_change_id.to_owned(),
        })?;
    let (change_session_id, change_connection_id, status) = row;
    if status != "unresolved" {
        return Err(StoreError::Conflict {
            entity: "unrecorded_change",
            id: unrecorded_change_id.to_owned(),
            detail: "unrecorded change is not unresolved".to_owned(),
        });
    }
    if change_connection_id != connection_internal_id {
        return Err(StoreError::Conflict {
            entity: "unrecorded_change",
            id: unrecorded_change_id.to_owned(),
            detail: "unrecorded change belongs to a different Agent Connection".to_owned(),
        });
    }
    if change_session_id
        .as_deref()
        .is_some_and(|change_session| change_session != session_id)
    {
        return Err(StoreError::Conflict {
            entity: "unrecorded_change",
            id: unrecorded_change_id.to_owned(),
            detail: "unrecorded change belongs to a different session".to_owned(),
        });
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> StoreResult<()> {
    validate_text(field, value)?;
    if value.trim().is_empty() {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not be empty"),
        })
    } else {
        Ok(())
    }
}

fn validate_text(field: &'static str, value: &str) -> StoreResult<()> {
    if value.contains('\0') {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not contain NUL bytes"),
        })
    } else {
        Ok(())
    }
}

fn validate_timestamp_text(field: &'static str, value: &str) -> StoreResult<()> {
    validate_identifier(field, value)
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

fn validate_json_array(field: &'static str, text: &str) -> StoreResult<()> {
    let value = serde_json::from_str::<Value>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be JSON array text: {error}"),
    })?;
    if value.is_array() {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be a JSON array"),
        })
    }
}

fn validate_lowercase_sha256(field: &str, value: &str) -> StoreResult<()> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be a lowercase 64-character SHA-256 hex string"),
        })
    }
}

fn path_to_text(field: &'static str, path: &Path) -> StoreResult<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| StoreError::InvalidInput {
            detail: format!("{field} must be valid UTF-8"),
        })
}

fn watch_baseline_from_conn(
    conn: &Connection,
    project_id: &str,
    watch_baseline_id: &str,
) -> StoreResult<Option<WatchBaselineRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            watch_baseline_id,
            session_id,
            connection_internal_id,
            guard_installation_id,
            status,
            scope_kind,
            repo_root,
            watched_paths_json,
            exclusions_json,
            snapshot_algorithm,
            snapshot_digest,
            snapshot_entries_json,
            created_at,
            updated_at,
            metadata_json
         FROM session_watch_baselines
        WHERE project_id = ?1
          AND watch_baseline_id = ?2",
        params![project_id, watch_baseline_id],
        watch_baseline_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn watch_baseline_by_conn(
    conn: &Connection,
    project_id: &str,
    watch_baseline_id: &str,
) -> StoreResult<WatchBaselineRecord> {
    watch_baseline_from_conn(conn, project_id, watch_baseline_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "session_watch_baseline",
            id: watch_baseline_id.to_owned(),
        }
    })
}

fn watch_baseline_from_row(row: &Row<'_>) -> rusqlite::Result<WatchBaselineRecord> {
    Ok(WatchBaselineRecord {
        project_id: row.get(0)?,
        watch_baseline_id: row.get(1)?,
        session_id: row.get(2)?,
        connection_internal_id: row.get(3)?,
        guard_installation_id: row.get(4)?,
        status: row.get(5)?,
        scope_kind: row.get(6)?,
        repo_root: row.get(7)?,
        watched_paths_json: row.get(8)?,
        exclusions_json: row.get(9)?,
        snapshot_algorithm: row.get(10)?,
        snapshot_digest: row.get(11)?,
        snapshot_entries_json: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        metadata_json: row.get(15)?,
    })
}

fn watch_observation_from_conn(
    conn: &Connection,
    project_id: &str,
    watch_observation_id: &str,
) -> StoreResult<Option<WatchObservationRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            watch_observation_id,
            watch_baseline_id,
            session_id,
            connection_internal_id,
            expected_write_id,
            unrecorded_change_id,
            observation_status,
            observed_paths_json,
            change_summary_json,
            snapshot_algorithm,
            snapshot_digest,
            snapshot_entries_json,
            observed_at,
            linked_at,
            metadata_json
         FROM session_watch_observations
        WHERE project_id = ?1
          AND watch_observation_id = ?2",
        params![project_id, watch_observation_id],
        watch_observation_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn watch_observation_by_conn(
    conn: &Connection,
    project_id: &str,
    watch_observation_id: &str,
) -> StoreResult<WatchObservationRecord> {
    watch_observation_from_conn(conn, project_id, watch_observation_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "session_watch_observation",
            id: watch_observation_id.to_owned(),
        }
    })
}

fn watch_observation_from_row(row: &Row<'_>) -> rusqlite::Result<WatchObservationRecord> {
    Ok(WatchObservationRecord {
        project_id: row.get(0)?,
        watch_observation_id: row.get(1)?,
        watch_baseline_id: row.get(2)?,
        session_id: row.get(3)?,
        connection_internal_id: row.get(4)?,
        expected_write_id: row.get(5)?,
        unrecorded_change_id: row.get(6)?,
        observation_status: row.get(7)?,
        observed_paths_json: row.get(8)?,
        change_summary_json: row.get(9)?,
        snapshot_algorithm: row.get(10)?,
        snapshot_digest: row.get(11)?,
        snapshot_entries_json: row.get(12)?,
        observed_at: row.get(13)?,
        linked_at: row.get(14)?,
        metadata_json: row.get(15)?,
    })
}

fn collect_rows<T, F>(rows: rusqlite::MappedRows<'_, F>) -> StoreResult<Vec<T>>
where
    F: FnMut(&Row<'_>) -> rusqlite::Result<T>,
{
    let mut values = Vec::new();
    for row in rows {
        values.push(row?);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, error::Error, fs, path::Path};

    use rusqlite::params;
    use volicord_test_support::TempRuntimeHome;

    use super::*;
    use crate::{
        agent_connections::{
            add_connection_project, ensure_agent_connection, AgentConnectionRegistration,
            ConnectionProjectRegistration, CONNECTION_INTENT_SHARED, CONNECTION_MODE_WORKFLOW,
            HOST_KIND_CODEX, HOST_SCOPE_PROJECT, VERIFIED_STATUS_COMPLETE,
        },
        bootstrap::{
            initialize_runtime_home, register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS,
        },
        guards::{
            insert_agent_session, insert_expected_write, insert_unrecorded_change,
            AgentSessionInsert, ExpectedWriteInsert, UnrecordedChangeInsert,
        },
        sqlite::open_project_state_database,
    };

    #[test]
    fn snapshot_diff_detects_added_modified_and_deleted_files() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("watch-snapshot-diff")?;
        let repo = fixture.create_product_repo("repo")?;
        write_file(&repo.join("src/keep.txt"), "before")?;
        write_file(&repo.join("src/delete.txt"), "remove me")?;
        let baseline =
            snapshot_product_repository(fixture.path(), &repo, WatchSnapshotOptions::default())?;

        write_file(&repo.join("src/keep.txt"), "after")?;
        fs::remove_file(repo.join("src/delete.txt"))?;
        write_file(&repo.join("src/add.txt"), "new")?;
        let current =
            snapshot_product_repository(fixture.path(), &repo, WatchSnapshotOptions::default())?;
        let diff = compare_watch_snapshots(&baseline, &current);

        let actual = diff
            .changes
            .iter()
            .map(|change| (change.path.as_str(), change.change_kind.as_str()))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            actual,
            BTreeSet::from([
                ("src/add.txt", "added"),
                ("src/delete.txt", "deleted"),
                ("src/keep.txt", "modified"),
            ])
        );
        Ok(())
    }

    #[test]
    fn snapshot_excludes_vcs_integration_and_configured_paths() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("watch-snapshot-exclusions")?;
        let repo = fixture.create_product_repo("repo")?;
        write_file(&repo.join("src/included.txt"), "before")?;
        write_file(&repo.join(".git/index"), "before")?;
        write_file(&repo.join(".volicord/policy.json"), "{}")?;
        write_file(&repo.join("target/generated.txt"), "before")?;
        let options = WatchSnapshotOptions {
            excluded_paths: vec![PathBuf::from("target")],
            ..WatchSnapshotOptions::default()
        };
        let baseline = snapshot_product_repository(fixture.path(), &repo, options.clone())?;

        assert_eq!(
            baseline
                .entries
                .iter()
                .map(|entry| entry.path.as_str())
                .collect::<Vec<_>>(),
            vec!["src/included.txt"]
        );

        write_file(&repo.join("src/included.txt"), "after")?;
        write_file(&repo.join(".git/index"), "after")?;
        write_file(&repo.join(".volicord/policy.json"), r#"{"changed":true}"#)?;
        write_file(&repo.join("target/generated.txt"), "after")?;
        let current = snapshot_product_repository(fixture.path(), &repo, options)?;
        let diff = compare_watch_snapshots(&baseline, &current);

        assert_eq!(diff.changes.len(), 1);
        assert_eq!(diff.changes[0].path, "src/included.txt");
        assert_eq!(diff.changes[0].change_kind, WatchPathChangeKind::Modified);
        Ok(())
    }

    #[test]
    fn snapshot_hash_is_stable_for_same_repository_state() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("watch-snapshot-stable")?;
        let repo = fixture.create_product_repo("repo")?;
        write_file(&repo.join("b.txt"), "bravo")?;
        write_file(&repo.join("a.txt"), "alpha")?;

        let first =
            snapshot_product_repository(fixture.path(), &repo, WatchSnapshotOptions::default())?;
        let second =
            snapshot_product_repository(fixture.path(), &repo, WatchSnapshotOptions::default())?;

        assert_eq!(first.digest, second.digest);
        assert_eq!(first.entries, second.entries);
        assert_eq!(
            first
                .entries
                .iter()
                .map(|entry| entry.path.as_str())
                .collect::<Vec<_>>(),
            vec!["a.txt", "b.txt"]
        );
        Ok(())
    }

    #[test]
    fn snapshot_marks_oversized_files_as_skipped_without_hashing() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("watch-snapshot-large")?;
        let repo = fixture.create_product_repo("repo")?;
        write_file(&repo.join("large.bin"), "abcdef")?;

        let snapshot = snapshot_product_repository(
            fixture.path(),
            &repo,
            WatchSnapshotOptions {
                max_file_size_bytes: 3,
                ..WatchSnapshotOptions::default()
            },
        )?;

        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].path, "large.bin");
        assert_eq!(snapshot.entries[0].kind, "skipped");
        assert_eq!(
            snapshot.entries[0].skip_reason.as_deref(),
            Some("size_limit")
        );
        assert!(snapshot.entries[0].sha256.is_none());
        Ok(())
    }

    #[test]
    fn watch_store_round_trips_baseline_observation_and_link() -> Result<(), Box<dyn Error>> {
        let fixture = WatchFixture::new("watch-store-round-trip")?;
        let repo = fixture.add_project_connection("project_watch_a", "conn_watch_a", "repo-a")?;
        fixture.insert_session("project_watch_a", "conn_watch_a", "session_watch_a")?;
        fixture.insert_task("project_watch_a", "task_watch_a")?;
        insert_expected_write(
            fixture.runtime_home.path(),
            "project_watch_a",
            ExpectedWriteInsert {
                expected_write_id: "expected_write_watch_a".to_owned(),
                session_id: Some("session_watch_a".to_owned()),
                connection_internal_id: "conn_watch_a".to_owned(),
                guard_installation_id: None,
                pre_tool_guard_event_id: "guard_event_watch_a".to_owned(),
                host_invocation_id: Some("tool_call_watch_a".to_owned()),
                tool_name: Some("shell".to_owned()),
                command_kind: "mutating".to_owned(),
                path_policy: "exact_paths".to_owned(),
                expected_paths_json: r#"["src/lib.rs"]"#.to_owned(),
                task_id: "task_watch_a".to_owned(),
                change_unit_id: None,
                write_ticket_ids_json: "[]".to_owned(),
                basis_state_version: 1,
                created_at: "2026-07-01T00:00:00Z".to_owned(),
                expires_at: "2026-07-01T00:15:00Z".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        write_file(&repo.join("src/lib.rs"), "before")?;
        let baseline_snapshot =
            snapshot_product_repository(fixture.runtime_home.path(), &repo, Default::default())?;
        let baseline = create_watch_baseline(
            fixture.runtime_home.path(),
            "project_watch_a",
            WatchBaselineCreate {
                watch_baseline_id: "watch_baseline_a".to_owned(),
                session_id: "session_watch_a".to_owned(),
                connection_internal_id: "conn_watch_a".to_owned(),
                guard_installation_id: None,
                status: SessionWatchStatus::Active,
                snapshot: baseline_snapshot.clone(),
                created_at: "2026-07-01T00:00:01Z".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        assert_eq!(baseline.status, "active");
        assert_eq!(baseline.scope_kind, "repository");
        assert_eq!(baseline.snapshot_digest, baseline_snapshot.digest);

        let degraded = update_watch_status(
            fixture.runtime_home.path(),
            "project_watch_a",
            "watch_baseline_a",
            WatchStatusUpdate {
                status: SessionWatchStatus::Degraded,
                updated_at: "2026-07-01T00:00:02Z".to_owned(),
                metadata_json: r#"{"reason":"test"}"#.to_owned(),
            },
        )?;
        assert_eq!(degraded.status, "degraded");

        write_file(&repo.join("src/lib.rs"), "after")?;
        let current_snapshot =
            snapshot_product_repository(fixture.runtime_home.path(), &repo, Default::default())?;
        let diff = compare_watch_snapshots(&baseline_snapshot, &current_snapshot);
        let observation = record_watch_observation(
            fixture.runtime_home.path(),
            "project_watch_a",
            WatchObservationInsert {
                watch_observation_id: "watch_observation_a".to_owned(),
                watch_baseline_id: "watch_baseline_a".to_owned(),
                expected_write_id: Some("expected_write_watch_a".to_owned()),
                snapshot: current_snapshot,
                diff: diff.clone(),
                observed_at: "2026-07-01T00:00:03Z".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        assert_eq!(observation.observation_status, "unresolved");
        assert_eq!(
            observation.expected_write_id.as_deref(),
            Some("expected_write_watch_a")
        );
        assert_eq!(observation.observed_paths_json, r#"["src/lib.rs"]"#);
        assert_eq!(
            list_unresolved_watch_observations(
                fixture.runtime_home.path(),
                "project_watch_a",
                "session_watch_a",
            )?
            .len(),
            1
        );

        insert_unrecorded_change(
            fixture.runtime_home.path(),
            "project_watch_a",
            UnrecordedChangeInsert {
                unrecorded_change_id: "unrecorded_change_watch_a".to_owned(),
                session_id: Some("session_watch_a".to_owned()),
                connection_internal_id: "conn_watch_a".to_owned(),
                task_id: None,
                summary: "Watch observation found a Product Repository change".to_owned(),
                observed_paths_json: diff.observed_paths_json(),
                detection_json: r#"{"source":"session_watch"}"#.to_owned(),
                detected_at: "2026-07-01T00:00:04Z".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        let linked = link_watch_observation_to_unrecorded_change(
            fixture.runtime_home.path(),
            "project_watch_a",
            "watch_observation_a",
            "unrecorded_change_watch_a",
            "2026-07-01T00:00:05Z",
        )?;
        assert_eq!(linked.observation_status, "linked");
        assert_eq!(
            linked.unrecorded_change_id.as_deref(),
            Some("unrecorded_change_watch_a")
        );
        assert!(list_unresolved_watch_observations(
            fixture.runtime_home.path(),
            "project_watch_a",
            "session_watch_a",
        )?
        .is_empty());
        Ok(())
    }

    #[test]
    fn watch_observations_are_project_and_session_scoped() -> Result<(), Box<dyn Error>> {
        let fixture = WatchFixture::new("watch-store-scope")?;
        let repo_a = fixture.add_project_connection("project_watch_a", "conn_watch_a", "repo-a")?;
        let _repo_b =
            fixture.add_project_connection("project_watch_b", "conn_watch_b", "repo-b")?;
        fixture.insert_session("project_watch_a", "conn_watch_a", "session_watch_a")?;
        fixture.insert_session("project_watch_b", "conn_watch_b", "session_watch_b")?;

        write_file(&repo_a.join("a.txt"), "before")?;
        let baseline_snapshot =
            snapshot_product_repository(fixture.runtime_home.path(), &repo_a, Default::default())?;
        create_watch_baseline(
            fixture.runtime_home.path(),
            "project_watch_a",
            WatchBaselineCreate {
                watch_baseline_id: "watch_baseline_scope_a".to_owned(),
                session_id: "session_watch_a".to_owned(),
                connection_internal_id: "conn_watch_a".to_owned(),
                guard_installation_id: None,
                status: SessionWatchStatus::Active,
                snapshot: baseline_snapshot.clone(),
                created_at: "2026-07-01T01:00:00Z".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        write_file(&repo_a.join("a.txt"), "after")?;
        let current_snapshot =
            snapshot_product_repository(fixture.runtime_home.path(), &repo_a, Default::default())?;
        record_watch_observation(
            fixture.runtime_home.path(),
            "project_watch_a",
            WatchObservationInsert {
                watch_observation_id: "watch_observation_scope_a".to_owned(),
                watch_baseline_id: "watch_baseline_scope_a".to_owned(),
                expected_write_id: None,
                snapshot: current_snapshot.clone(),
                diff: compare_watch_snapshots(&baseline_snapshot, &current_snapshot),
                observed_at: "2026-07-01T01:01:00Z".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;

        assert_eq!(
            list_unresolved_watch_observations(
                fixture.runtime_home.path(),
                "project_watch_a",
                "session_watch_a",
            )?
            .len(),
            1
        );
        assert!(list_unresolved_watch_observations(
            fixture.runtime_home.path(),
            "project_watch_b",
            "session_watch_b",
        )?
        .is_empty());
        assert!(watch_observation(
            fixture.runtime_home.path(),
            "project_watch_b",
            "watch_observation_scope_a",
        )?
        .is_none());
        Ok(())
    }

    struct WatchFixture {
        runtime_home: TempRuntimeHome,
    }

    impl WatchFixture {
        fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
            let runtime_home = TempRuntimeHome::new(prefix)?;
            initialize_runtime_home(runtime_home.path(), &format!("runtime_home_{prefix}"), "{}")?;
            Ok(Self { runtime_home })
        }

        fn add_project_connection(
            &self,
            project_id: &str,
            connection_id: &str,
            repo_name: &str,
        ) -> Result<PathBuf, Box<dyn Error>> {
            let repo_root = self.runtime_home.create_product_repo(repo_name)?;
            register_project(
                self.runtime_home.path(),
                ProjectRegistration {
                    project_id: project_id.to_owned(),
                    repo_root: repo_root.clone(),
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            ensure_agent_connection(
                self.runtime_home.path(),
                AgentConnectionRegistration {
                    connection_internal_id: connection_id.to_owned(),
                    host_kind: HOST_KIND_CODEX.to_owned(),
                    intent: CONNECTION_INTENT_SHARED.to_owned(),
                    host_scope: HOST_SCOPE_PROJECT.to_owned(),
                    server_name: format!("volicord-{connection_id}"),
                    config_target: self
                        .runtime_home
                        .path()
                        .join("agent-connections")
                        .join(connection_id)
                        .to_string_lossy()
                        .into_owned(),
                    mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                    enabled: true,
                    managed_fingerprint: format!("fingerprint:{connection_id}"),
                    last_verification_status: VERIFIED_STATUS_COMPLETE.to_owned(),
                    last_verification_report_json: "{}".to_owned(),
                    last_user_actions_json: "[]".to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            add_connection_project(
                self.runtime_home.path(),
                ConnectionProjectRegistration {
                    connection_internal_id: connection_id.to_owned(),
                    project_id: project_id.to_owned(),
                },
            )?;
            Ok(repo_root)
        }

        fn insert_session(
            &self,
            project_id: &str,
            connection_id: &str,
            session_id: &str,
        ) -> Result<(), Box<dyn Error>> {
            insert_agent_session(
                self.runtime_home.path(),
                project_id,
                AgentSessionInsert {
                    session_id: session_id.to_owned(),
                    connection_internal_id: connection_id.to_owned(),
                    guard_installation_id: None,
                    host_kind: "codex".to_owned(),
                    guard_mode: "record".to_owned(),
                    started_at: "2026-07-01T00:00:00Z".to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            Ok(())
        }

        fn insert_task(&self, project_id: &str, task_id: &str) -> Result<(), Box<dyn Error>> {
            let project = project_record_for_execution(self.runtime_home.path(), project_id)?
                .expect("project should be registered");
            let conn = open_project_state_database(&project.state_db_path)?;
            conn.execute(
                "INSERT INTO tasks (
                    project_id,
                    task_id,
                    created_by_actor_source,
                    mode,
                    lifecycle_phase,
                    created_at,
                    updated_at
                )
                VALUES (?1, ?2, 'agent_connection:conn_watch_a', 'work', 'shaping', 't0', 't0')",
                params![project_id, task_id],
            )?;
            Ok(())
        }
    }

    fn write_file(path: &Path, contents: &str) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)
    }
}
