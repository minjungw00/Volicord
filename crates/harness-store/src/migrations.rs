use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::{
    sqlite::{begin_immediate_transaction, foreign_keys_enabled, set_foreign_keys},
    StoreError, StoreResult,
};

/// Baseline storage profile recorded by schema migrations.
pub const STORAGE_PROFILE: &str = "baseline_sqlite";

/// Historical baseline schema version used by the first SQLite migrations.
pub const BASELINE_SCHEMA_VERSION: i64 = 1;

/// Latest schema version for `registry.sqlite`.
pub const REGISTRY_SCHEMA_VERSION: i64 = 1;

/// Latest schema version for project `state.sqlite`.
pub const PROJECT_STATE_SCHEMA_VERSION: i64 = 5;

const PROJECT_STATE_REPLAY_CONTEXT_SCHEMA_VERSION: i64 = 2;
const PROJECT_STATE_REPLAY_SURFACE_FK_SCHEMA_VERSION: i64 = 3;
const PROJECT_STATE_CLOSE_BASIS_JUDGMENT_BASIS_SCHEMA_VERSION: i64 = 4;
const PROJECT_STATE_JUDGMENT_RESOLUTION_OUTCOME_SCHEMA_VERSION: i64 = 5;

/// `schema_migrations.database_kind` for `registry.sqlite`.
pub const REGISTRY_DATABASE_KIND: &str = "registry";

/// `schema_migrations.database_kind` for project `state.sqlite`.
pub const PROJECT_STATE_DATABASE_KIND: &str = "project_state";

const REGISTRY_MIGRATIONS: &[Migration] = &[Migration {
    database_kind: REGISTRY_DATABASE_KIND,
    version: BASELINE_SCHEMA_VERSION,
    name: "registry_baseline_v1",
    kind: MigrationKind::Sql(REGISTRY_BASELINE_SQL),
}];

const PROJECT_STATE_MIGRATIONS: &[Migration] = &[
    Migration {
        database_kind: PROJECT_STATE_DATABASE_KIND,
        version: BASELINE_SCHEMA_VERSION,
        name: "project_state_baseline_v1",
        kind: MigrationKind::Sql(PROJECT_STATE_BASELINE_SQL),
    },
    Migration {
        database_kind: PROJECT_STATE_DATABASE_KIND,
        version: PROJECT_STATE_REPLAY_CONTEXT_SCHEMA_VERSION,
        name: "project_state_replay_context_v2",
        kind: MigrationKind::Sql(PROJECT_STATE_REPLAY_CONTEXT_V2_SQL),
    },
    Migration {
        database_kind: PROJECT_STATE_DATABASE_KIND,
        version: PROJECT_STATE_REPLAY_SURFACE_FK_SCHEMA_VERSION,
        name: "project_state_replay_surface_fk_v3",
        kind: MigrationKind::Custom(apply_project_state_replay_surface_fk_v3),
    },
    Migration {
        database_kind: PROJECT_STATE_DATABASE_KIND,
        version: PROJECT_STATE_CLOSE_BASIS_JUDGMENT_BASIS_SCHEMA_VERSION,
        name: "project_state_close_basis_judgment_basis_v4",
        kind: MigrationKind::Sql(PROJECT_STATE_CLOSE_BASIS_JUDGMENT_BASIS_V4_SQL),
    },
    Migration {
        database_kind: PROJECT_STATE_DATABASE_KIND,
        version: PROJECT_STATE_JUDGMENT_RESOLUTION_OUTCOME_SCHEMA_VERSION,
        name: "project_state_judgment_resolution_outcome_v5",
        kind: MigrationKind::Sql(PROJECT_STATE_JUDGMENT_RESOLUTION_OUTCOME_V5_SQL),
    },
];

struct Migration {
    database_kind: &'static str,
    version: i64,
    name: &'static str,
    kind: MigrationKind,
}

#[derive(Clone, Copy)]
enum MigrationKind {
    Sql(&'static str),
    Custom(fn(&mut Connection, &Migration) -> StoreResult<()>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExpectedMigration {
    pub database_kind: &'static str,
    pub version: i64,
    pub name: &'static str,
}

/// Applies the executable baseline migration for `registry.sqlite`.
pub fn apply_registry_migrations(conn: &mut Connection) -> StoreResult<()> {
    apply_ordered_migrations(conn, REGISTRY_MIGRATIONS)
}

/// Applies the executable baseline migration for project `state.sqlite`.
pub fn apply_project_state_migrations(conn: &mut Connection) -> StoreResult<()> {
    apply_ordered_migrations(conn, PROJECT_STATE_MIGRATIONS)
}

pub(crate) fn expected_registry_migrations() -> Vec<ExpectedMigration> {
    expected_migrations(REGISTRY_MIGRATIONS)
}

pub(crate) fn expected_project_state_migrations() -> Vec<ExpectedMigration> {
    expected_migrations(PROJECT_STATE_MIGRATIONS)
}

fn expected_migrations(migrations: &[Migration]) -> Vec<ExpectedMigration> {
    migrations
        .iter()
        .map(|migration| ExpectedMigration {
            database_kind: migration.database_kind,
            version: migration.version,
            name: migration.name,
        })
        .collect()
}

fn apply_ordered_migrations(conn: &mut Connection, migrations: &[Migration]) -> StoreResult<()> {
    for migration in migrations {
        if let Some((actual_name, actual_storage_profile)) = existing_migration(conn, migration)? {
            if actual_name != migration.name || actual_storage_profile != STORAGE_PROFILE {
                return Err(StoreError::MigrationConflict {
                    database_kind: migration.database_kind,
                    version: migration.version,
                    expected_name: migration.name,
                    actual_name,
                    expected_storage_profile: STORAGE_PROFILE,
                    actual_storage_profile,
                });
            }
            continue;
        }

        apply_migration(conn, migration)?;
    }

    Ok(())
}

fn apply_migration(conn: &mut Connection, migration: &Migration) -> StoreResult<()> {
    match migration.kind {
        MigrationKind::Sql(sql) => apply_sql_migration(conn, migration, sql),
        MigrationKind::Custom(apply) => apply(conn, migration),
    }
}

fn apply_sql_migration(
    conn: &mut Connection,
    migration: &Migration,
    sql: &'static str,
) -> StoreResult<()> {
    let tx = begin_immediate_transaction(conn)?;
    tx.execute_batch(sql)?;
    insert_schema_migration(&tx, migration)?;
    tx.commit()?;
    Ok(())
}

fn existing_migration(
    conn: &Connection,
    migration: &Migration,
) -> rusqlite::Result<Option<(String, String)>> {
    let schema_table_exists = conn.query_row(
        "SELECT COUNT(*)
           FROM sqlite_master
          WHERE type = 'table' AND name = 'schema_migrations'",
        [],
        |row| row.get::<_, i64>(0),
    )? > 0;

    if !schema_table_exists {
        return Ok(None);
    }

    conn.query_row(
        "SELECT name, storage_profile
           FROM schema_migrations
          WHERE database_kind = ?1 AND version = ?2",
        params![migration.database_kind, migration.version],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
}

fn insert_schema_migration(tx: &Transaction<'_>, migration: &Migration) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO schema_migrations (
            database_kind,
            version,
            name,
            storage_profile,
            applied_at
        )
        VALUES (?1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        params![
            migration.database_kind,
            migration.version,
            migration.name,
            STORAGE_PROFILE
        ],
    )?;
    Ok(())
}

fn apply_project_state_replay_surface_fk_v3(
    conn: &mut Connection,
    migration: &Migration,
) -> StoreResult<()> {
    validate_no_foreign_key_violations(conn, PROJECT_STATE_DATABASE_KIND, None)?;

    let original_foreign_key_mode = foreign_keys_enabled(conn)?;
    if original_foreign_key_mode {
        set_foreign_keys(conn, false)?;
    }

    let migration_result = rebuild_tool_invocations_with_surface_fk(conn, migration);
    let restore_result = restore_foreign_key_mode(conn, original_foreign_key_mode);

    match (migration_result, restore_result) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(StoreError::from(error)),
        (Ok(()), Ok(())) => {
            validate_no_foreign_key_violations(conn, PROJECT_STATE_DATABASE_KIND, None)?;
            Ok(())
        }
    }
}

fn rebuild_tool_invocations_with_surface_fk(
    conn: &mut Connection,
    migration: &Migration,
) -> StoreResult<()> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute_batch(PROJECT_STATE_REPLAY_SURFACE_FK_V3_CREATE_COPY_SQL)?;
    validate_no_foreign_key_violations(
        &tx,
        PROJECT_STATE_DATABASE_KIND,
        Some("tool_invocations_rebuild_v3"),
    )?;
    tx.execute_batch(PROJECT_STATE_REPLAY_SURFACE_FK_V3_SWAP_SQL)?;
    insert_schema_migration(&tx, migration)?;
    tx.commit()?;
    Ok(())
}

fn restore_foreign_key_mode(conn: &Connection, enabled: bool) -> rusqlite::Result<()> {
    if foreign_keys_enabled(conn)? != enabled {
        set_foreign_keys(conn, enabled)?;
    }
    Ok(())
}

fn validate_no_foreign_key_violations(
    conn: &Connection,
    database_kind: &'static str,
    table: Option<&str>,
) -> StoreResult<()> {
    let sql = match table {
        Some(table) => format!(
            "PRAGMA foreign_key_check(\"{}\")",
            table.replace('"', "\"\"")
        ),
        None => "PRAGMA foreign_key_check".to_owned(),
    };
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;

    if rows.next()?.is_some() {
        let detail = match table {
            Some(table) => {
                format!("PRAGMA foreign_key_check reported a violation for {table}")
            }
            None => "PRAGMA foreign_key_check reported a violation".to_owned(),
        };
        return Err(StoreError::schema_invariant(database_kind, detail));
    }

    Ok(())
}

const REGISTRY_BASELINE_SQL: &str = r#"
CREATE TABLE schema_migrations (
  database_kind TEXT NOT NULL CHECK (database_kind = 'registry'),
  version INTEGER NOT NULL CHECK (version > 0),
  name TEXT NOT NULL,
  storage_profile TEXT NOT NULL,
  applied_at TEXT NOT NULL,
  checksum_sha256 TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (database_kind, version)
);

CREATE TABLE runtime_home (
  singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
  runtime_home_id TEXT NOT NULL UNIQUE,
  storage_profile TEXT NOT NULL,
  schema_version INTEGER NOT NULL CHECK (schema_version > 0),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE projects (
  project_id TEXT PRIMARY KEY,
  runtime_home_id TEXT NOT NULL,
  repo_root TEXT NOT NULL,
  project_home TEXT NOT NULL UNIQUE,
  state_db_path TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'active' CHECK (status = 'active'),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  FOREIGN KEY (runtime_home_id) REFERENCES runtime_home (runtime_home_id)
);

CREATE INDEX idx_projects_repo_root ON projects (repo_root);
CREATE INDEX idx_projects_status ON projects (status);
"#;

const PROJECT_STATE_BASELINE_SQL: &str = r#"
CREATE TABLE schema_migrations (
  database_kind TEXT NOT NULL CHECK (database_kind = 'project_state'),
  version INTEGER NOT NULL CHECK (version > 0),
  name TEXT NOT NULL,
  storage_profile TEXT NOT NULL,
  applied_at TEXT NOT NULL,
  checksum_sha256 TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (database_kind, version)
);

CREATE TABLE project_state (
  project_id TEXT PRIMARY KEY,
  storage_profile TEXT NOT NULL,
  schema_version INTEGER NOT NULL CHECK (schema_version > 0),
  state_version INTEGER NOT NULL DEFAULT 0 CHECK (state_version >= 0),
  active_task_id TEXT,
  default_surface_id TEXT,
  default_surface_instance_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  CHECK (
    (default_surface_id IS NULL AND default_surface_instance_id IS NULL)
    OR (default_surface_id IS NOT NULL AND default_surface_instance_id IS NOT NULL)
  ),
  FOREIGN KEY (project_id, active_task_id)
    REFERENCES tasks (project_id, task_id)
    DEFERRABLE INITIALLY DEFERRED,
  FOREIGN KEY (project_id, default_surface_id, default_surface_instance_id)
    REFERENCES surfaces (project_id, surface_id, surface_instance_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE surfaces (
  project_id TEXT NOT NULL,
  surface_id TEXT NOT NULL,
  surface_instance_id TEXT NOT NULL,
  surface_kind TEXT NOT NULL,
  display_name TEXT,
  capability_profile_json TEXT NOT NULL DEFAULT '{}',
  local_access_json TEXT NOT NULL DEFAULT '{}',
  registered_at TEXT NOT NULL,
  last_seen_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, surface_id, surface_instance_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id)
);

CREATE TABLE tasks (
  project_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  created_by_surface_id TEXT NOT NULL,
  created_by_surface_instance_id TEXT NOT NULL,
  mode TEXT NOT NULL,
  lifecycle_phase TEXT NOT NULL,
  result TEXT,
  title TEXT,
  summary TEXT,
  shaping_summary_json TEXT NOT NULL DEFAULT '{}',
  bounded_context_json TEXT NOT NULL DEFAULT '[]',
  autonomy_boundary_json TEXT NOT NULL DEFAULT '{}',
  close_summary_json TEXT NOT NULL DEFAULT '{}',
  completion_policy_json TEXT NOT NULL DEFAULT '{}',
  current_change_unit_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  closed_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, task_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, created_by_surface_id, created_by_surface_instance_id)
    REFERENCES surfaces (project_id, surface_id, surface_instance_id),
  FOREIGN KEY (project_id, task_id, current_change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE change_units (
  project_id TEXT NOT NULL,
  change_unit_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('proposed', 'active', 'replaced', 'closed')),
  is_current INTEGER NOT NULL DEFAULT 0 CHECK (is_current IN (0, 1)),
  basis_state_version INTEGER CHECK (basis_state_version >= 0),
  scope_summary_json TEXT NOT NULL DEFAULT '{}',
  bounded_paths_json TEXT NOT NULL DEFAULT '[]',
  write_basis_json TEXT NOT NULL DEFAULT '{}',
  close_basis_json TEXT NOT NULL DEFAULT '{}',
  lifecycle_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  closed_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, change_unit_id),
  UNIQUE (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id)
);

CREATE UNIQUE INDEX idx_change_units_one_current_active
  ON change_units (project_id, task_id)
  WHERE status = 'active' AND is_current = 1;

CREATE TABLE user_judgments (
  project_id TEXT NOT NULL,
  judgment_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  judgment_kind TEXT NOT NULL,
  status TEXT NOT NULL,
  request_json TEXT NOT NULL DEFAULT '{}',
  context_json TEXT NOT NULL DEFAULT '{}',
  options_json TEXT NOT NULL DEFAULT '[]',
  affected_refs_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  sensitive_action_scope_json TEXT NOT NULL DEFAULT '{}',
  resolution_json TEXT,
  requested_by_surface_id TEXT NOT NULL,
  requested_by_surface_instance_id TEXT NOT NULL,
  requested_at TEXT NOT NULL,
  resolved_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, judgment_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, requested_by_surface_id, requested_by_surface_instance_id)
    REFERENCES surfaces (project_id, surface_id, surface_instance_id)
);

CREATE TABLE write_authorizations (
  project_id TEXT NOT NULL,
  write_authorization_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version > 0),
  status TEXT NOT NULL CHECK (status IN ('active', 'consumed', 'expired', 'stale', 'revoked')),
  attempt_scope_json TEXT NOT NULL DEFAULT '{}',
  created_by_surface_id TEXT NOT NULL,
  created_by_surface_instance_id TEXT NOT NULL,
  created_by_judgment_id TEXT,
  expires_at TEXT NOT NULL,
  consumed_by_run_id TEXT,
  consumed_at TEXT,
  revoked_at TEXT,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, write_authorization_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, created_by_surface_id, created_by_surface_instance_id)
    REFERENCES surfaces (project_id, surface_id, surface_instance_id),
  FOREIGN KEY (project_id, created_by_judgment_id)
    REFERENCES user_judgments (project_id, judgment_id),
  FOREIGN KEY (project_id, consumed_by_run_id)
    REFERENCES runs (project_id, run_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE UNIQUE INDEX idx_write_authorizations_consumed_run
  ON write_authorizations (project_id, consumed_by_run_id)
  WHERE consumed_by_run_id IS NOT NULL;

CREATE TABLE runs (
  project_id TEXT NOT NULL,
  run_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  write_authorization_id TEXT,
  kind TEXT NOT NULL,
  status TEXT NOT NULL,
  summary_json TEXT NOT NULL DEFAULT '{}',
  observed_changes_json TEXT NOT NULL DEFAULT '{}',
  evidence_updates_json TEXT NOT NULL DEFAULT '[]',
  authorization_effect_json TEXT NOT NULL DEFAULT '{}',
  created_by_surface_id TEXT NOT NULL,
  created_by_surface_instance_id TEXT NOT NULL,
  started_at TEXT,
  completed_at TEXT,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, run_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, write_authorization_id)
    REFERENCES write_authorizations (project_id, write_authorization_id)
    DEFERRABLE INITIALLY DEFERRED,
  FOREIGN KEY (project_id, created_by_surface_id, created_by_surface_instance_id)
    REFERENCES surfaces (project_id, surface_id, surface_instance_id)
);

CREATE UNIQUE INDEX idx_runs_write_authorization
  ON runs (project_id, write_authorization_id)
  WHERE write_authorization_id IS NOT NULL;

CREATE TABLE artifact_staging (
  project_id TEXT NOT NULL,
  handle_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  created_by_surface_id TEXT NOT NULL,
  created_by_surface_instance_id TEXT NOT NULL,
  artifact_json TEXT NOT NULL DEFAULT '{}',
  safe_metadata_json TEXT NOT NULL DEFAULT '{}',
  tmp_path TEXT,
  sha256 TEXT,
  size_bytes INTEGER CHECK (size_bytes IS NULL OR size_bytes >= 0),
  content_type TEXT,
  redaction_state TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('staged', 'consumed', 'expired', 'discarded')),
  expires_at TEXT NOT NULL,
  consumed_by_run_id TEXT,
  promoted_artifact_id TEXT,
  consumed_at TEXT,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, handle_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, created_by_surface_id, created_by_surface_instance_id)
    REFERENCES surfaces (project_id, surface_id, surface_instance_id),
  FOREIGN KEY (project_id, consumed_by_run_id)
    REFERENCES runs (project_id, run_id)
    DEFERRABLE INITIALLY DEFERRED,
  FOREIGN KEY (project_id, promoted_artifact_id)
    REFERENCES artifacts (project_id, artifact_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE UNIQUE INDEX idx_artifact_staging_promoted_artifact
  ON artifact_staging (project_id, promoted_artifact_id)
  WHERE promoted_artifact_id IS NOT NULL;

CREATE TABLE artifacts (
  project_id TEXT NOT NULL,
  artifact_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  producer_run_id TEXT,
  source_staging_handle_id TEXT,
  uri TEXT NOT NULL,
  body_path TEXT,
  sha256 TEXT,
  size_bytes INTEGER CHECK (size_bytes IS NULL OR size_bytes >= 0),
  content_type TEXT,
  redaction_state TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('available', 'missing', 'integrity_failed', 'unavailable')),
  retention_json TEXT NOT NULL DEFAULT '{}',
  producer_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, artifact_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, producer_run_id) REFERENCES runs (project_id, run_id),
  FOREIGN KEY (project_id, source_staging_handle_id)
    REFERENCES artifact_staging (project_id, handle_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE UNIQUE INDEX idx_artifacts_source_staging
  ON artifacts (project_id, source_staging_handle_id)
  WHERE source_staging_handle_id IS NOT NULL;

CREATE TABLE artifact_links (
  project_id TEXT NOT NULL,
  artifact_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  owner_record_kind TEXT NOT NULL CHECK (
    owner_record_kind IN ('task', 'change_unit', 'run', 'user_judgment', 'evidence_summary', 'blocker')
  ),
  owner_record_id TEXT NOT NULL,
  created_by_run_id TEXT,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, artifact_id, owner_record_kind, owner_record_id),
  FOREIGN KEY (project_id, artifact_id) REFERENCES artifacts (project_id, artifact_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, created_by_run_id) REFERENCES runs (project_id, run_id)
);

CREATE TABLE evidence_summaries (
  project_id TEXT NOT NULL,
  evidence_summary_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  status TEXT NOT NULL,
  coverage_json TEXT NOT NULL DEFAULT '[]',
  supporting_refs_json TEXT NOT NULL DEFAULT '[]',
  gap_refs_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, evidence_summary_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
);

CREATE TABLE blockers (
  project_id TEXT NOT NULL,
  blocker_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  status TEXT NOT NULL CHECK (status IN ('active', 'resolved', 'superseded')),
  category TEXT NOT NULL,
  code TEXT NOT NULL,
  owner_refs_json TEXT NOT NULL DEFAULT '[]',
  related_refs_json TEXT NOT NULL DEFAULT '[]',
  detail_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  resolved_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, blocker_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
);

CREATE TABLE task_events (
  project_id TEXT NOT NULL,
  event_seq INTEGER NOT NULL CHECK (event_seq > 0),
  event_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  state_version INTEGER NOT NULL CHECK (state_version > 0),
  event_kind TEXT NOT NULL,
  event_payload_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, event_seq),
  UNIQUE (project_id, event_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
);

CREATE TABLE tool_invocations (
  project_id TEXT NOT NULL,
  tool_name TEXT NOT NULL,
  idempotency_key TEXT NOT NULL,
  request_hash TEXT NOT NULL,
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version >= 0),
  committed_state_version INTEGER NOT NULL CHECK (committed_state_version > basis_state_version),
  status TEXT NOT NULL DEFAULT 'committed' CHECK (status = 'committed'),
  response_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, tool_name, idempotency_key),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id)
);

CREATE INDEX idx_project_state_active_task
  ON project_state (project_id, active_task_id);

CREATE INDEX idx_surfaces_last_seen
  ON surfaces (project_id, last_seen_at);

CREATE INDEX idx_tasks_lifecycle
  ON tasks (project_id, lifecycle_phase, result);

CREATE INDEX idx_tasks_current_change_unit
  ON tasks (project_id, current_change_unit_id);

CREATE INDEX idx_change_units_task_status
  ON change_units (project_id, task_id, status);

CREATE INDEX idx_user_judgments_task_status
  ON user_judgments (project_id, task_id, status);

CREATE INDEX idx_write_authorizations_task_status
  ON write_authorizations (project_id, task_id, status);

CREATE INDEX idx_runs_task_created
  ON runs (project_id, task_id, created_at);

CREATE INDEX idx_artifact_staging_task_status
  ON artifact_staging (project_id, task_id, status);

CREATE INDEX idx_artifact_staging_surface
  ON artifact_staging (project_id, created_by_surface_id, created_by_surface_instance_id);

CREATE INDEX idx_artifacts_task_status
  ON artifacts (project_id, task_id, status);

CREATE INDEX idx_artifact_links_owner
  ON artifact_links (project_id, owner_record_kind, owner_record_id);

CREATE INDEX idx_evidence_summaries_task_status
  ON evidence_summaries (project_id, task_id, status);

CREATE INDEX idx_blockers_task_status
  ON blockers (project_id, task_id, status);

CREATE INDEX idx_task_events_task_seq
  ON task_events (project_id, task_id, event_seq);
"#;

const PROJECT_STATE_REPLAY_CONTEXT_V2_SQL: &str = r#"
ALTER TABLE tool_invocations
  ADD COLUMN surface_id TEXT;

ALTER TABLE tool_invocations
  ADD COLUMN surface_instance_id TEXT;

ALTER TABLE tool_invocations
  ADD COLUMN access_class TEXT;

ALTER TABLE tool_invocations
  ADD COLUMN verification_basis TEXT;

ALTER TABLE tool_invocations
  ADD COLUMN replay_context_status TEXT NOT NULL DEFAULT 'legacy_unverified'
    CHECK (replay_context_status IN ('verified', 'legacy_unverified'));

CREATE TRIGGER tool_invocations_verified_context_insert
BEFORE INSERT ON tool_invocations
FOR EACH ROW
WHEN NEW.replay_context_status = 'verified'
  AND (
    NEW.surface_id IS NULL
    OR NEW.surface_instance_id IS NULL
    OR NEW.access_class IS NULL
  )
BEGIN
  SELECT RAISE(ABORT, 'verified replay context requires surface_id, surface_instance_id, and access_class');
END;

CREATE TRIGGER tool_invocations_verified_context_update
BEFORE UPDATE ON tool_invocations
FOR EACH ROW
WHEN NEW.replay_context_status = 'verified'
  AND (
    NEW.surface_id IS NULL
    OR NEW.surface_instance_id IS NULL
    OR NEW.access_class IS NULL
  )
BEGIN
  SELECT RAISE(ABORT, 'verified replay context requires surface_id, surface_instance_id, and access_class');
END;

UPDATE project_state
   SET schema_version = 2,
       updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
 WHERE schema_version < 2;
"#;

const PROJECT_STATE_REPLAY_SURFACE_FK_V3_CREATE_COPY_SQL: &str = r#"
CREATE TABLE tool_invocations_rebuild_v3 (
  project_id TEXT NOT NULL,
  tool_name TEXT NOT NULL,
  idempotency_key TEXT NOT NULL,
  request_hash TEXT NOT NULL,
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version >= 0),
  committed_state_version INTEGER NOT NULL CHECK (committed_state_version > basis_state_version),
  status TEXT NOT NULL DEFAULT 'committed' CHECK (status = 'committed'),
  surface_id TEXT,
  surface_instance_id TEXT,
  access_class TEXT,
  verification_basis TEXT,
  replay_context_status TEXT NOT NULL DEFAULT 'legacy_unverified'
    CHECK (replay_context_status IN ('verified', 'legacy_unverified')),
  response_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, tool_name, idempotency_key),
  CHECK (
    (
      replay_context_status = 'verified'
      AND surface_id IS NOT NULL
      AND surface_instance_id IS NOT NULL
      AND access_class IS NOT NULL
    )
    OR (
      replay_context_status = 'legacy_unverified'
    )
  ),
  FOREIGN KEY (project_id, surface_id, surface_instance_id)
    REFERENCES surfaces (project_id, surface_id, surface_instance_id)
    ON DELETE RESTRICT,
  FOREIGN KEY (project_id) REFERENCES project_state (project_id)
);

INSERT INTO tool_invocations_rebuild_v3 (
  project_id,
  tool_name,
  idempotency_key,
  request_hash,
  basis_state_version,
  committed_state_version,
  status,
  surface_id,
  surface_instance_id,
  access_class,
  verification_basis,
  replay_context_status,
  response_json,
  created_at
)
SELECT
  project_id,
  tool_name,
  idempotency_key,
  request_hash,
  basis_state_version,
  committed_state_version,
  status,
  surface_id,
  surface_instance_id,
  access_class,
  verification_basis,
  replay_context_status,
  response_json,
  created_at
FROM tool_invocations;
"#;

const PROJECT_STATE_REPLAY_SURFACE_FK_V3_SWAP_SQL: &str = r#"
DROP TABLE tool_invocations;

ALTER TABLE tool_invocations_rebuild_v3 RENAME TO tool_invocations;

CREATE TRIGGER tool_invocations_verified_context_insert
BEFORE INSERT ON tool_invocations
FOR EACH ROW
WHEN NEW.replay_context_status = 'verified'
  AND (
    NEW.surface_id IS NULL
    OR NEW.surface_instance_id IS NULL
    OR NEW.access_class IS NULL
  )
BEGIN
  SELECT RAISE(ABORT, 'verified replay context requires surface_id, surface_instance_id, and access_class');
END;

CREATE TRIGGER tool_invocations_verified_context_update
BEFORE UPDATE ON tool_invocations
FOR EACH ROW
WHEN NEW.replay_context_status = 'verified'
  AND (
    NEW.surface_id IS NULL
    OR NEW.surface_instance_id IS NULL
    OR NEW.access_class IS NULL
  )
BEGIN
  SELECT RAISE(ABORT, 'verified replay context requires surface_id, surface_instance_id, and access_class');
END;

UPDATE project_state
   SET schema_version = 3,
       updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
 WHERE schema_version < 3;
"#;

const PROJECT_STATE_CLOSE_BASIS_JUDGMENT_BASIS_V4_SQL: &str = r#"
ALTER TABLE tasks
  ADD COLUMN scope_revision INTEGER NOT NULL DEFAULT 0 CHECK (scope_revision >= 0);

ALTER TABLE tasks
  ADD COLUMN close_basis_revision INTEGER NOT NULL DEFAULT 0 CHECK (close_basis_revision >= 0);

ALTER TABLE tasks
  ADD COLUMN close_basis_json TEXT;

ALTER TABLE user_judgments
  ADD COLUMN basis_json TEXT;

ALTER TABLE user_judgments
  ADD COLUMN basis_status TEXT NOT NULL DEFAULT 'legacy_unbound'
    CHECK (basis_status IN ('current', 'stale', 'superseded', 'legacy_unbound'));

UPDATE project_state
   SET schema_version = 4,
       updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
 WHERE schema_version < 4;
"#;

const PROJECT_STATE_JUDGMENT_RESOLUTION_OUTCOME_V5_SQL: &str = r#"
ALTER TABLE user_judgments
  ADD COLUMN resolution_outcome TEXT
    CHECK (resolution_outcome IS NULL OR resolution_outcome IN ('accepted', 'rejected', 'deferred', 'blocked'));

UPDATE user_judgments
   SET resolution_outcome = status,
       status = 'resolved'
 WHERE status IN ('rejected', 'deferred', 'blocked')
   AND resolution_outcome IS NULL;

UPDATE project_state
   SET schema_version = 5,
       updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
 WHERE schema_version < 5;
"#;

#[cfg(test)]
mod tests {
    use std::{error::Error, fs};

    use harness_test_support::TempRuntimeHome;
    use rusqlite::{params, Connection, Error as SqliteError, ErrorCode};

    use super::*;
    use crate::sqlite::{
        enable_foreign_keys, foreign_keys_enabled, open_project_state_database,
        project_state_db_path, validate_project_state_schema,
    };

    #[test]
    fn version_one_project_state_replay_rows_become_legacy_unverified() -> Result<(), Box<dyn Error>>
    {
        let runtime_home = TempRuntimeHome::new("migration-v1-replay")?;
        let path = project_state_db_path(runtime_home.path(), "project_v1");
        fs::create_dir_all(path.parent().expect("state db path has parent"))?;
        let conn = Connection::open(&path)?;
        conn.execute_batch(PROJECT_STATE_BASELINE_SQL)?;
        conn.execute(
            "INSERT INTO schema_migrations (
                database_kind,
                version,
                name,
                storage_profile,
                applied_at
            )
            VALUES (?1, ?2, 'project_state_baseline_v1', ?3, 't0')",
            params![
                PROJECT_STATE_DATABASE_KIND,
                BASELINE_SCHEMA_VERSION,
                STORAGE_PROFILE
            ],
        )?;
        conn.execute(
            "INSERT INTO project_state (
                project_id,
                storage_profile,
                schema_version,
                created_at,
                updated_at
            )
            VALUES ('project_v1', ?1, 1, 't0', 't0')",
            [STORAGE_PROFILE],
        )?;
        conn.execute(
            "INSERT INTO tool_invocations (
                project_id,
                tool_name,
                idempotency_key,
                request_hash,
                basis_state_version,
                committed_state_version,
                response_json,
                created_at
            )
            VALUES (
                'project_v1',
                'harness.update_scope',
                'idem_legacy',
                'sha256:legacy',
                0,
                1,
                '{\"legacy\":true}',
                't0'
            )",
            [],
        )?;
        drop(conn);

        let conn = open_project_state_database(&path)?;
        let (schema_version, migration_count): (i64, i64) = conn.query_row(
            "SELECT
                (SELECT schema_version FROM project_state WHERE project_id = 'project_v1'),
                (SELECT COUNT(*) FROM schema_migrations WHERE database_kind = ?1)",
            [PROJECT_STATE_DATABASE_KIND],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(schema_version, PROJECT_STATE_SCHEMA_VERSION);
        assert_eq!(migration_count, 5);
        assert_integrity_check_clean(&conn)?;
        assert_tool_invocations_surface_foreign_key(&conn)?;
        assert_foreign_key_check_clean(&conn)?;

        let (status, surface_id, response_json): (String, Option<String>, String) = conn
            .query_row(
                "SELECT replay_context_status, surface_id, response_json
                   FROM tool_invocations
                  WHERE project_id = 'project_v1'
                    AND tool_name = 'harness.update_scope'
                    AND idempotency_key = 'idem_legacy'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
        assert_eq!(status, "legacy_unverified");
        assert!(surface_id.is_none());
        assert_eq!(response_json, "{\"legacy\":true}");
        Ok(())
    }

    #[test]
    fn version_two_project_state_replay_rows_upgrade_to_surface_foreign_key(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("migration-v2-replay-fk")?;
        let path = project_state_db_path(runtime_home.path(), "project_v2");
        fs::create_dir_all(path.parent().expect("state db path has parent"))?;
        let mut conn = Connection::open(&path)?;
        enable_foreign_keys(&conn)?;
        create_project_state_v2(&conn, "project_v2")?;
        conn.execute(
            "INSERT INTO surfaces (
                project_id,
                surface_id,
                surface_instance_id,
                surface_kind,
                registered_at
            )
            VALUES ('project_v2', 'surface_main', 'surface_instance_1', 'cli', 't0')",
            [],
        )?;
        conn.execute(
            "INSERT INTO tool_invocations (
                project_id,
                tool_name,
                idempotency_key,
                request_hash,
                basis_state_version,
                committed_state_version,
                replay_context_status,
                response_json,
                created_at
            )
            VALUES (
                'project_v2',
                'harness.update_scope',
                'idem_legacy',
                'sha256:legacy',
                0,
                1,
                'legacy_unverified',
                '{\"legacy\":true}',
                't0'
            )",
            [],
        )?;
        conn.execute(
            "INSERT INTO tool_invocations (
                project_id,
                tool_name,
                idempotency_key,
                request_hash,
                basis_state_version,
                committed_state_version,
                surface_id,
                surface_instance_id,
                access_class,
                verification_basis,
                replay_context_status,
                response_json,
                created_at
            )
            VALUES (
                'project_v2',
                'harness.update_scope',
                'idem_verified',
                'sha256:verified',
                1,
                2,
                'surface_main',
                'surface_instance_1',
                'core_mutation',
                'migration_test_registration',
                'verified',
                '{\"verified\":true}',
                't1'
            )",
            [],
        )?;

        apply_project_state_migrations(&mut conn)?;
        validate_project_state_schema(&conn)?;

        assert!(foreign_keys_enabled(&conn)?);
        assert_eq!(
            latest_migration_version(&conn, PROJECT_STATE_DATABASE_KIND)?,
            PROJECT_STATE_SCHEMA_VERSION
        );
        assert_eq!(migration_count(&conn, PROJECT_STATE_DATABASE_KIND)?, 5);
        assert_integrity_check_clean(&conn)?;
        assert_tool_invocations_surface_foreign_key(&conn)?;
        assert_foreign_key_check_clean(&conn)?;

        let legacy_status: String = conn.query_row(
            "SELECT replay_context_status
               FROM tool_invocations
              WHERE idempotency_key = 'idem_legacy'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(legacy_status, "legacy_unverified");

        let verified_context: (String, String, String, String) = conn.query_row(
            "SELECT
                replay_context_status,
                surface_id,
                surface_instance_id,
                access_class
               FROM tool_invocations
              WHERE idempotency_key = 'idem_verified'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        assert_eq!(
            verified_context,
            (
                "verified".to_owned(),
                "surface_main".to_owned(),
                "surface_instance_1".to_owned(),
                "core_mutation".to_owned(),
            )
        );
        Ok(())
    }

    #[test]
    fn invalid_verified_replay_surface_reference_fails_migration_atomically(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("migration-v2-invalid-verified")?;
        let path = project_state_db_path(runtime_home.path(), "project_bad_verified");
        fs::create_dir_all(path.parent().expect("state db path has parent"))?;
        let mut conn = Connection::open(&path)?;
        enable_foreign_keys(&conn)?;
        create_project_state_v2(&conn, "project_bad_verified")?;
        conn.execute(
            "INSERT INTO tool_invocations (
                project_id,
                tool_name,
                idempotency_key,
                request_hash,
                basis_state_version,
                committed_state_version,
                surface_id,
                surface_instance_id,
                access_class,
                replay_context_status,
                response_json,
                created_at
            )
            VALUES (
                'project_bad_verified',
                'harness.update_scope',
                'idem_invalid_verified',
                'sha256:invalid',
                0,
                1,
                'missing_surface',
                'missing_instance',
                'core_mutation',
                'verified',
                '{\"invalid\":true}',
                't0'
            )",
            [],
        )?;
        let original_table_sql = tool_invocations_table_sql(&conn)?;

        let error = apply_project_state_migrations(&mut conn)
            .expect_err("missing verified replay surface must fail migration");
        assert!(matches!(
            error,
            StoreError::SchemaInvariant {
                database_kind: PROJECT_STATE_DATABASE_KIND,
                ..
            }
        ));
        assert!(foreign_keys_enabled(&conn)?);
        assert_eq!(
            latest_migration_version(&conn, PROJECT_STATE_DATABASE_KIND)?,
            PROJECT_STATE_REPLAY_CONTEXT_SCHEMA_VERSION
        );
        assert_eq!(migration_count(&conn, PROJECT_STATE_DATABASE_KIND)?, 2);
        assert_eq!(project_schema_version(&conn, "project_bad_verified")?, 2);
        assert_eq!(tool_invocations_table_sql(&conn)?, original_table_sql);
        assert!(!tool_invocations_has_surface_foreign_key(&conn)?);
        assert_integrity_check_clean(&conn)?;

        let response_json: String = conn.query_row(
            "SELECT response_json
               FROM tool_invocations
              WHERE idempotency_key = 'idem_invalid_verified'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(response_json, "{\"invalid\":true}");

        conn.execute(
            "INSERT INTO tool_invocations (
                project_id,
                tool_name,
                idempotency_key,
                request_hash,
                basis_state_version,
                committed_state_version,
                replay_context_status,
                response_json,
                created_at
            )
            VALUES (
                'project_bad_verified',
                'harness.status',
                'idem_after_failure',
                'sha256:after',
                0,
                1,
                'legacy_unverified',
                '{\"after\":true}',
                't1'
            )",
            [],
        )?;
        Ok(())
    }

    #[test]
    fn version_three_project_state_close_basis_and_judgment_basis_upgrade(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("migration-v3-close-basis")?;
        let path = project_state_db_path(runtime_home.path(), "project_v3_close");
        fs::create_dir_all(path.parent().expect("state db path has parent"))?;
        let mut conn = Connection::open(&path)?;
        enable_foreign_keys(&conn)?;
        create_project_state_v3(&conn, "project_v3_close")?;
        insert_surface(&conn, "project_v3_close")?;
        insert_task_v3(
            &conn,
            "project_v3_close",
            "task_open",
            "shaping",
            None,
            "{\"legacy_summary\":\"open\"}",
            None,
        )?;
        insert_task_v3(
            &conn,
            "project_v3_close",
            "task_closed",
            "completed",
            Some("completed"),
            "{\"terminal\":true}",
            Some("t_closed"),
        )?;
        insert_user_judgment_v3(
            &conn,
            "project_v3_close",
            "judgment_legacy",
            "task_closed",
            Some("{\"selected_option_id\":\"accept\"}"),
        )?;

        apply_project_state_migrations(&mut conn)?;
        validate_project_state_schema(&conn)?;

        assert_eq!(
            latest_migration_version(&conn, PROJECT_STATE_DATABASE_KIND)?,
            PROJECT_STATE_SCHEMA_VERSION
        );
        assert_eq!(migration_count(&conn, PROJECT_STATE_DATABASE_KIND)?, 5);
        assert_integrity_check_clean(&conn)?;
        assert_foreign_key_check_clean(&conn)?;

        let open_task: (i64, i64, Option<String>, String) = conn.query_row(
            "SELECT
                scope_revision,
                close_basis_revision,
                close_basis_json,
                close_summary_json
               FROM tasks
              WHERE task_id = 'task_open'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        assert_eq!(
            open_task,
            (0, 0, None, "{\"legacy_summary\":\"open\"}".to_owned(),)
        );

        let closed_task: (i64, i64, Option<String>, String, Option<String>) = conn.query_row(
            "SELECT
                scope_revision,
                close_basis_revision,
                close_basis_json,
                close_summary_json,
                closed_at
               FROM tasks
              WHERE task_id = 'task_closed'",
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
        assert_eq!(
            closed_task,
            (
                0,
                0,
                None,
                "{\"terminal\":true}".to_owned(),
                Some("t_closed".to_owned()),
            )
        );

        let judgment: (Option<String>, String, Option<String>) = conn.query_row(
            "SELECT basis_json, basis_status, resolution_json
               FROM user_judgments
              WHERE judgment_id = 'judgment_legacy'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        assert_eq!(
            judgment,
            (
                None,
                "legacy_unbound".to_owned(),
                Some("{\"selected_option_id\":\"accept\"}".to_owned()),
            )
        );
        Ok(())
    }

    #[test]
    fn close_basis_judgment_basis_migration_rolls_back_atomically() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("migration-v4-rollback")?;
        let path = project_state_db_path(runtime_home.path(), "project_v4_rollback");
        fs::create_dir_all(path.parent().expect("state db path has parent"))?;
        let mut conn = Connection::open(&path)?;
        enable_foreign_keys(&conn)?;
        create_project_state_v3(&conn, "project_v4_rollback")?;
        conn.execute(
            "ALTER TABLE tasks ADD COLUMN close_basis_revision INTEGER",
            [],
        )?;

        let error = apply_project_state_migrations(&mut conn)
            .expect_err("duplicate v4 column should fail migration");
        assert!(matches!(error, StoreError::Sqlite(_)));
        assert_eq!(
            latest_migration_version(&conn, PROJECT_STATE_DATABASE_KIND)?,
            PROJECT_STATE_REPLAY_SURFACE_FK_SCHEMA_VERSION
        );
        assert_eq!(migration_count(&conn, PROJECT_STATE_DATABASE_KIND)?, 3);
        assert_eq!(project_schema_version(&conn, "project_v4_rollback")?, 3);
        assert!(!column_exists(&conn, "tasks", "scope_revision")?);
        assert!(!column_exists(&conn, "tasks", "close_basis_json")?);
        assert!(!column_exists(&conn, "user_judgments", "basis_status")?);
        assert_integrity_check_clean(&conn)?;
        Ok(())
    }

    #[test]
    fn fresh_project_state_database_has_complete_replay_context_schema(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("migration-fresh-replay")?;
        let path = project_state_db_path(runtime_home.path(), "project_fresh");
        let conn = open_project_state_database(&path)?;

        for column in [
            "surface_id",
            "surface_instance_id",
            "access_class",
            "verification_basis",
            "replay_context_status",
        ] {
            assert!(
                column_exists(&conn, "tool_invocations", column)?,
                "tool_invocations.{column} should exist on fresh databases"
            );
        }
        for trigger in [
            "tool_invocations_verified_context_insert",
            "tool_invocations_verified_context_update",
        ] {
            assert!(
                sqlite_object_exists(&conn, "trigger", trigger)?,
                "{trigger} should enforce verified replay context completeness"
            );
        }
        assert_eq!(
            latest_migration_version(&conn, PROJECT_STATE_DATABASE_KIND)?,
            PROJECT_STATE_SCHEMA_VERSION
        );
        assert_eq!(migration_count(&conn, PROJECT_STATE_DATABASE_KIND)?, 5);
        assert_integrity_check_clean(&conn)?;
        assert_tool_invocations_surface_foreign_key(&conn)?;
        assert_foreign_key_check_clean(&conn)?;
        Ok(())
    }

    #[test]
    fn fresh_project_state_database_has_close_basis_judgment_basis_and_outcome_schema(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("migration-fresh-close-basis")?;
        let path = project_state_db_path(runtime_home.path(), "project_fresh_close");
        let conn = open_project_state_database(&path)?;

        for (table, column) in [
            ("tasks", "scope_revision"),
            ("tasks", "close_basis_revision"),
            ("tasks", "close_basis_json"),
            ("user_judgments", "basis_json"),
            ("user_judgments", "basis_status"),
            ("user_judgments", "resolution_outcome"),
        ] {
            assert!(
                column_exists(&conn, table, column)?,
                "{table}.{column} should exist on fresh databases"
            );
        }
        assert!(table_sql(&conn, "user_judgments")?
            .contains("basis_status TEXT NOT NULL DEFAULT 'legacy_unbound'"));
        assert!(table_sql(&conn, "user_judgments")?.contains(
            "CHECK (basis_status IN ('current', 'stale', 'superseded', 'legacy_unbound'))"
        ));
        assert!(table_sql(&conn, "user_judgments")?.contains(
            "resolution_outcome IS NULL OR resolution_outcome IN ('accepted', 'rejected', 'deferred', 'blocked')"
        ));
        assert_eq!(
            latest_migration_version(&conn, PROJECT_STATE_DATABASE_KIND)?,
            PROJECT_STATE_SCHEMA_VERSION
        );
        assert_eq!(migration_count(&conn, PROJECT_STATE_DATABASE_KIND)?, 5);
        assert_integrity_check_clean(&conn)?;
        assert_foreign_key_check_clean(&conn)?;
        Ok(())
    }

    #[test]
    fn version_four_project_state_judgment_outcome_upgrade() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("migration-v4-outcome")?;
        let path = project_state_db_path(runtime_home.path(), "project_v4_outcome");
        fs::create_dir_all(path.parent().expect("state db path has parent"))?;
        let mut conn = Connection::open(&path)?;
        enable_foreign_keys(&conn)?;
        create_project_state_v4(&conn, "project_v4_outcome")?;
        insert_surface(&conn, "project_v4_outcome")?;
        insert_task_current(&conn, "project_v4_outcome", "task_outcome")?;
        for (judgment_id, status) in [
            ("judgment_pending", "pending"),
            ("judgment_resolved_ambiguous", "resolved"),
            ("judgment_rejected", "rejected"),
            ("judgment_deferred", "deferred"),
            ("judgment_blocked", "blocked"),
        ] {
            insert_user_judgment_v4_status(
                &conn,
                "project_v4_outcome",
                judgment_id,
                "task_outcome",
                status,
            )?;
        }

        apply_project_state_migrations(&mut conn)?;
        validate_project_state_schema(&conn)?;

        assert_eq!(
            latest_migration_version(&conn, PROJECT_STATE_DATABASE_KIND)?,
            PROJECT_STATE_SCHEMA_VERSION
        );
        assert_eq!(migration_count(&conn, PROJECT_STATE_DATABASE_KIND)?, 5);
        assert_judgment_status_and_outcome(&conn, "judgment_pending", "pending", None)?;
        assert_judgment_status_and_outcome(&conn, "judgment_resolved_ambiguous", "resolved", None)?;
        assert_judgment_status_and_outcome(
            &conn,
            "judgment_rejected",
            "resolved",
            Some("rejected"),
        )?;
        assert_judgment_status_and_outcome(
            &conn,
            "judgment_deferred",
            "resolved",
            Some("deferred"),
        )?;
        assert_judgment_status_and_outcome(&conn, "judgment_blocked", "resolved", Some("blocked"))?;
        assert_integrity_check_clean(&conn)?;
        assert_foreign_key_check_clean(&conn)?;
        Ok(())
    }

    #[test]
    fn judgment_outcome_migration_rolls_back_atomically() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("migration-v5-rollback")?;
        let path = project_state_db_path(runtime_home.path(), "project_v5_rollback");
        fs::create_dir_all(path.parent().expect("state db path has parent"))?;
        let mut conn = Connection::open(&path)?;
        enable_foreign_keys(&conn)?;
        create_project_state_v4(&conn, "project_v5_rollback")?;
        conn.execute(
            "ALTER TABLE user_judgments ADD COLUMN resolution_outcome TEXT",
            [],
        )?;

        let error = apply_project_state_migrations(&mut conn)
            .expect_err("duplicate v5 column should fail migration");
        assert!(matches!(error, StoreError::Sqlite(_)));
        assert_eq!(
            latest_migration_version(&conn, PROJECT_STATE_DATABASE_KIND)?,
            PROJECT_STATE_CLOSE_BASIS_JUDGMENT_BASIS_SCHEMA_VERSION
        );
        assert_eq!(migration_count(&conn, PROJECT_STATE_DATABASE_KIND)?, 4);
        assert_eq!(project_schema_version(&conn, "project_v5_rollback")?, 4);
        assert_integrity_check_clean(&conn)?;
        Ok(())
    }

    #[test]
    fn invalid_judgment_basis_status_is_rejected() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("migration-invalid-basis-status")?;
        let conn = open_project_state_database(
            runtime_home.project_state_db_path("project_invalid_basis_status"),
        )?;
        insert_project_state(&conn, "project_invalid_basis_status")?;
        insert_surface(&conn, "project_invalid_basis_status")?;
        insert_task_current(&conn, "project_invalid_basis_status", "task_basis_status")?;

        let error = conn
            .execute(
                "INSERT INTO user_judgments (
                    project_id,
                    judgment_id,
                    task_id,
                    judgment_kind,
                    status,
                    requested_by_surface_id,
                    requested_by_surface_instance_id,
                    requested_at,
                    basis_status
                )
                VALUES (
                    'project_invalid_basis_status',
                    'judgment_invalid_status',
                    'task_basis_status',
                    'final_acceptance',
                    'pending',
                    'surface_main',
                    'surface_instance_1',
                    't0',
                    'invalid'
                )",
                [],
            )
            .expect_err("basis_status must be constrained");
        assert_constraint_error(error);
        Ok(())
    }

    #[test]
    fn invalid_judgment_resolution_outcome_is_rejected() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("migration-invalid-resolution-outcome")?;
        let conn = open_project_state_database(
            runtime_home.project_state_db_path("project_invalid_resolution_outcome"),
        )?;
        insert_project_state(&conn, "project_invalid_resolution_outcome")?;
        insert_surface(&conn, "project_invalid_resolution_outcome")?;
        insert_task_current(
            &conn,
            "project_invalid_resolution_outcome",
            "task_resolution_outcome",
        )?;

        let error = conn
            .execute(
                "INSERT INTO user_judgments (
                    project_id,
                    judgment_id,
                    task_id,
                    judgment_kind,
                    status,
                    resolution_outcome,
                    requested_by_surface_id,
                    requested_by_surface_instance_id,
                    requested_at
                )
                VALUES (
                    'project_invalid_resolution_outcome',
                    'judgment_invalid_outcome',
                    'task_resolution_outcome',
                    'final_acceptance',
                    'resolved',
                    'approved',
                    'surface_main',
                    'surface_instance_1',
                    't0'
                )",
                [],
            )
            .expect_err("resolution_outcome must be constrained");
        assert_constraint_error(error);
        Ok(())
    }

    fn create_project_state_v4(conn: &Connection, project_id: &str) -> rusqlite::Result<()> {
        create_project_state_v3(conn, project_id)?;
        conn.execute_batch(PROJECT_STATE_CLOSE_BASIS_JUDGMENT_BASIS_V4_SQL)?;
        insert_migration_row(
            conn,
            PROJECT_STATE_CLOSE_BASIS_JUDGMENT_BASIS_SCHEMA_VERSION,
            "project_state_close_basis_judgment_basis_v4",
        )?;
        Ok(())
    }

    fn create_project_state_v3(conn: &Connection, project_id: &str) -> rusqlite::Result<()> {
        create_project_state_v2(conn, project_id)?;
        conn.execute_batch(PROJECT_STATE_REPLAY_SURFACE_FK_V3_CREATE_COPY_SQL)?;
        conn.execute_batch(PROJECT_STATE_REPLAY_SURFACE_FK_V3_SWAP_SQL)?;
        insert_migration_row(
            conn,
            PROJECT_STATE_REPLAY_SURFACE_FK_SCHEMA_VERSION,
            "project_state_replay_surface_fk_v3",
        )?;
        Ok(())
    }

    fn create_project_state_v2(conn: &Connection, project_id: &str) -> rusqlite::Result<()> {
        conn.execute_batch(PROJECT_STATE_BASELINE_SQL)?;
        insert_migration_row(conn, BASELINE_SCHEMA_VERSION, "project_state_baseline_v1")?;
        conn.execute_batch(PROJECT_STATE_REPLAY_CONTEXT_V2_SQL)?;
        insert_migration_row(
            conn,
            PROJECT_STATE_REPLAY_CONTEXT_SCHEMA_VERSION,
            "project_state_replay_context_v2",
        )?;
        conn.execute(
            "INSERT INTO project_state (
                project_id,
                storage_profile,
                schema_version,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, 2, 't0', 't0')",
            params![project_id, STORAGE_PROFILE],
        )?;
        Ok(())
    }

    fn insert_project_state(conn: &Connection, project_id: &str) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT INTO project_state (
                project_id,
                storage_profile,
                schema_version,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, 't0', 't0')",
            params![project_id, STORAGE_PROFILE, PROJECT_STATE_SCHEMA_VERSION],
        )?;
        Ok(())
    }

    fn insert_surface(conn: &Connection, project_id: &str) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT INTO surfaces (
                project_id,
                surface_id,
                surface_instance_id,
                surface_kind,
                registered_at
            )
            VALUES (?1, 'surface_main', 'surface_instance_1', 'cli', 't0')",
            [project_id],
        )?;
        Ok(())
    }

    fn insert_task_current(
        conn: &Connection,
        project_id: &str,
        task_id: &str,
    ) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT INTO tasks (
                project_id,
                task_id,
                created_by_surface_id,
                created_by_surface_instance_id,
                mode,
                lifecycle_phase,
                created_at,
                updated_at
            )
            VALUES (
                ?1,
                ?2,
                'surface_main',
                'surface_instance_1',
                'work',
                'shaping',
                't0',
                't0'
            )",
            params![project_id, task_id],
        )?;
        Ok(())
    }

    fn insert_task_v3(
        conn: &Connection,
        project_id: &str,
        task_id: &str,
        lifecycle_phase: &str,
        result: Option<&str>,
        close_summary_json: &str,
        closed_at: Option<&str>,
    ) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT INTO tasks (
                project_id,
                task_id,
                created_by_surface_id,
                created_by_surface_instance_id,
                mode,
                lifecycle_phase,
                result,
                close_summary_json,
                created_at,
                updated_at,
                closed_at
            )
            VALUES (
                ?1,
                ?2,
                'surface_main',
                'surface_instance_1',
                'work',
                ?3,
                ?4,
                ?5,
                't0',
                't0',
                ?6
            )",
            params![
                project_id,
                task_id,
                lifecycle_phase,
                result,
                close_summary_json,
                closed_at
            ],
        )?;
        Ok(())
    }

    fn insert_user_judgment_v3(
        conn: &Connection,
        project_id: &str,
        judgment_id: &str,
        task_id: &str,
        resolution_json: Option<&str>,
    ) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT INTO user_judgments (
                project_id,
                judgment_id,
                task_id,
                judgment_kind,
                status,
                request_json,
                requested_by_surface_id,
                requested_by_surface_instance_id,
                requested_at,
                resolution_json,
                resolved_at
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                'final_acceptance',
                'resolved',
                '{\"question\":\"Accept?\"}',
                'surface_main',
                'surface_instance_1',
                't0',
                ?4,
                't1'
            )",
            params![project_id, judgment_id, task_id, resolution_json],
        )?;
        Ok(())
    }

    fn insert_user_judgment_v4_status(
        conn: &Connection,
        project_id: &str,
        judgment_id: &str,
        task_id: &str,
        status: &str,
    ) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT INTO user_judgments (
                project_id,
                judgment_id,
                task_id,
                judgment_kind,
                status,
                request_json,
                requested_by_surface_id,
                requested_by_surface_instance_id,
                requested_at,
                resolved_at
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                'final_acceptance',
                ?4,
                '{\"question\":\"Accept?\"}',
                'surface_main',
                'surface_instance_1',
                't0',
                CASE WHEN ?4 = 'pending' THEN NULL ELSE 't1' END
            )",
            params![project_id, judgment_id, task_id, status],
        )?;
        Ok(())
    }

    fn assert_judgment_status_and_outcome(
        conn: &Connection,
        judgment_id: &str,
        expected_status: &str,
        expected_outcome: Option<&str>,
    ) -> rusqlite::Result<()> {
        let actual: (String, Option<String>) = conn.query_row(
            "SELECT status, resolution_outcome
               FROM user_judgments
              WHERE judgment_id = ?1",
            [judgment_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(
            actual,
            (
                expected_status.to_owned(),
                expected_outcome.map(str::to_owned)
            )
        );
        Ok(())
    }

    fn insert_migration_row(conn: &Connection, version: i64, name: &str) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT INTO schema_migrations (
                database_kind,
                version,
                name,
                storage_profile,
                applied_at
            )
            VALUES (?1, ?2, ?3, ?4, 't0')",
            params![PROJECT_STATE_DATABASE_KIND, version, name, STORAGE_PROFILE],
        )?;
        Ok(())
    }

    fn project_schema_version(conn: &Connection, project_id: &str) -> rusqlite::Result<i64> {
        conn.query_row(
            "SELECT schema_version
               FROM project_state
              WHERE project_id = ?1",
            [project_id],
            |row| row.get(0),
        )
    }

    fn tool_invocations_table_sql(conn: &Connection) -> rusqlite::Result<String> {
        conn.query_row(
            "SELECT sql
               FROM sqlite_master
              WHERE type = 'table'
                AND name = 'tool_invocations'",
            [],
            |row| row.get(0),
        )
    }

    fn table_sql(conn: &Connection, table: &str) -> rusqlite::Result<String> {
        conn.query_row(
            "SELECT sql
               FROM sqlite_master
              WHERE type = 'table'
                AND name = ?1",
            [table],
            |row| row.get(0),
        )
    }

    fn assert_foreign_key_check_clean(conn: &Connection) -> rusqlite::Result<()> {
        let mut stmt = conn.prepare("PRAGMA foreign_key_check")?;
        let mut rows = stmt.query([])?;
        assert!(rows.next()?.is_none());
        Ok(())
    }

    fn assert_integrity_check_clean(conn: &Connection) -> rusqlite::Result<()> {
        let result: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        assert_eq!(result, "ok");
        Ok(())
    }

    fn assert_tool_invocations_surface_foreign_key(conn: &Connection) -> rusqlite::Result<()> {
        assert!(tool_invocations_has_surface_foreign_key(conn)?);
        Ok(())
    }

    fn tool_invocations_has_surface_foreign_key(conn: &Connection) -> rusqlite::Result<bool> {
        let mut stmt = conn.prepare("PRAGMA foreign_key_list(tool_invocations)")?;
        let mapped_rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        let mut rows = Vec::new();
        for row in mapped_rows {
            rows.push(row?);
        }

        let expected = [
            ("project_id", "project_id"),
            ("surface_id", "surface_id"),
            ("surface_instance_id", "surface_instance_id"),
        ];
        for id in rows.iter().map(|(id, _, _, _, _, _)| *id) {
            let mut candidate = rows
                .iter()
                .filter(|(candidate_id, _, _, _, _, _)| *candidate_id == id)
                .cloned()
                .collect::<Vec<_>>();
            candidate.sort_by_key(|(_, seq, _, _, _, _)| *seq);
            if candidate.len() != expected.len() {
                continue;
            }
            if !candidate.iter().all(|(_, _, table, _, _, on_delete)| {
                table == "surfaces" && on_delete == "RESTRICT"
            }) {
                continue;
            }
            let actual = candidate
                .iter()
                .map(|(_, _, _, from, to, _)| (from.as_str(), to.as_str()))
                .collect::<Vec<_>>();
            if actual == expected {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn assert_constraint_error(err: SqliteError) {
        match err {
            SqliteError::SqliteFailure(error, _) => {
                assert_eq!(error.code, ErrorCode::ConstraintViolation);
            }
            other => panic!("expected SQLite constraint error, got {other:?}"),
        }
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
        let escaped_table = table.replace('"', "\"\"");
        let sql = format!("PRAGMA table_info(\"{escaped_table}\")");
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == column {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn sqlite_object_exists(
        conn: &Connection,
        object_type: &str,
        name: &str,
    ) -> rusqlite::Result<bool> {
        conn.query_row(
            "SELECT COUNT(*)
               FROM sqlite_master
              WHERE type = ?1
                AND name = ?2",
            params![object_type, name],
            |row| Ok(row.get::<_, i64>(0)? == 1),
        )
    }

    fn latest_migration_version(conn: &Connection, database_kind: &str) -> rusqlite::Result<i64> {
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0)
               FROM schema_migrations
              WHERE database_kind = ?1",
            [database_kind],
            |row| row.get(0),
        )
    }

    fn migration_count(conn: &Connection, database_kind: &str) -> rusqlite::Result<i64> {
        conn.query_row(
            "SELECT COUNT(*)
               FROM schema_migrations
              WHERE database_kind = ?1",
            [database_kind],
            |row| row.get(0),
        )
    }
}
