use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::{sqlite::begin_immediate_transaction, StoreError, StoreResult};

/// Baseline storage profile recorded by schema migrations.
pub const STORAGE_PROFILE: &str = "baseline_sqlite_v3";

pub(crate) const OLD_STORAGE_PROFILE: &str = "baseline_sqlite";

/// Latest schema version for `registry.sqlite`.
pub const REGISTRY_SCHEMA_VERSION: i64 = 4;

/// Latest schema version for project `state.sqlite`.
pub const PROJECT_STATE_SCHEMA_VERSION: i64 = 5;

/// `schema_migrations.database_kind` for `registry.sqlite`.
pub const REGISTRY_DATABASE_KIND: &str = "registry";

/// `schema_migrations.database_kind` for project `state.sqlite`.
pub const PROJECT_STATE_DATABASE_KIND: &str = "project_state";

const REGISTRY_MIGRATIONS: &[Migration] = &[
    Migration {
        database_kind: REGISTRY_DATABASE_KIND,
        version: 1,
        name: "registry_initial_v1",
        sql: REGISTRY_INITIAL_SQL,
    },
    Migration {
        database_kind: REGISTRY_DATABASE_KIND,
        version: 2,
        name: "registry_guard_records_v2",
        sql: REGISTRY_GUARD_RECORDS_SQL,
    },
    Migration {
        database_kind: REGISTRY_DATABASE_KIND,
        version: 3,
        name: "registry_guard_installation_lifecycle_v3",
        sql: REGISTRY_GUARD_INSTALLATION_LIFECYCLE_SQL,
    },
    Migration {
        database_kind: REGISTRY_DATABASE_KIND,
        version: REGISTRY_SCHEMA_VERSION,
        name: "registry_local_web_consent_tokens_v4",
        sql: REGISTRY_LOCAL_WEB_CONSENT_TOKENS_SQL,
    },
];

const PROJECT_STATE_MIGRATIONS: &[Migration] = &[
    Migration {
        database_kind: PROJECT_STATE_DATABASE_KIND,
        version: 1,
        name: "project_state_initial_v1",
        sql: PROJECT_STATE_INITIAL_SQL,
    },
    Migration {
        database_kind: PROJECT_STATE_DATABASE_KIND,
        version: 2,
        name: "project_state_guard_records_v2",
        sql: PROJECT_STATE_GUARD_RECORDS_SQL,
    },
    Migration {
        database_kind: PROJECT_STATE_DATABASE_KIND,
        version: 3,
        name: "project_state_expected_writes_v3",
        sql: PROJECT_STATE_EXPECTED_WRITES_SQL,
    },
    Migration {
        database_kind: PROJECT_STATE_DATABASE_KIND,
        version: 4,
        name: "project_state_local_recovery_v4",
        sql: PROJECT_STATE_LOCAL_RECOVERY_SQL,
    },
    Migration {
        database_kind: PROJECT_STATE_DATABASE_KIND,
        version: PROJECT_STATE_SCHEMA_VERSION,
        name: "project_state_session_watch_v5",
        sql: PROJECT_STATE_SESSION_WATCH_SQL,
    },
];

struct Migration {
    database_kind: &'static str,
    version: i64,
    name: &'static str,
    sql: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExpectedMigration {
    pub database_kind: &'static str,
    pub version: i64,
    pub name: &'static str,
}

#[derive(Debug)]
struct ExistingMigrationRow {
    database_kind: String,
    version: i64,
    name: String,
    storage_profile: String,
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
    validate_existing_migration_history(conn, migrations)?;

    for migration in migrations {
        if existing_migration(conn, migration)?.is_some() {
            continue;
        }

        apply_sql_migration(conn, migration)?;
    }

    Ok(())
}

fn validate_existing_migration_history(
    conn: &Connection,
    migrations: &[Migration],
) -> StoreResult<()> {
    if !schema_migrations_table_exists(conn)? {
        return Ok(());
    }

    let Some(first_migration) = migrations.first() else {
        return Ok(());
    };
    let rows = existing_migration_rows(conn)?;
    if rows.is_empty() {
        return Err(StoreError::schema_invariant(
            first_migration.database_kind,
            "schema_migrations exists but has no rows",
        ));
    }

    for row in &rows {
        if row.database_kind != first_migration.database_kind {
            return Err(StoreError::schema_invariant(
                first_migration.database_kind,
                format!(
                    "schema_migrations contains row for database_kind {}",
                    row.database_kind
                ),
            ));
        }
        if row.storage_profile != STORAGE_PROFILE {
            return Err(StoreError::unsupported_storage_profile(
                first_migration.database_kind,
                row.storage_profile.clone(),
                STORAGE_PROFILE,
            ));
        }
    }

    if rows.len() > migrations.len() {
        let latest_actual = rows
            .last()
            .map(|row| row.version)
            .unwrap_or(first_migration.version);
        let latest_supported = migrations
            .last()
            .map(|migration| migration.version)
            .unwrap_or(first_migration.version);
        return Err(StoreError::schema_invariant(
            first_migration.database_kind,
            format!(
                "migration version {latest_actual} is newer than supported version {latest_supported}"
            ),
        ));
    }

    for (index, row) in rows.iter().enumerate() {
        let expected = &migrations[index];
        if row.version != expected.version || row.name != expected.name {
            return Err(StoreError::MigrationConflict {
                database_kind: expected.database_kind,
                version: expected.version,
                expected_name: expected.name,
                actual_name: row.name.clone(),
                expected_storage_profile: STORAGE_PROFILE,
                actual_storage_profile: row.storage_profile.clone(),
            });
        }
    }

    Ok(())
}

fn apply_sql_migration(conn: &mut Connection, migration: &Migration) -> StoreResult<()> {
    let tx = begin_immediate_transaction(conn)?;
    tx.execute_batch(migration.sql)?;
    insert_schema_migration(&tx, migration)?;
    tx.commit()?;
    Ok(())
}

fn schema_migrations_table_exists(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT COUNT(*)
           FROM sqlite_master
          WHERE type = 'table' AND name = 'schema_migrations'",
        [],
        |row| Ok(row.get::<_, i64>(0)? > 0),
    )
}

fn existing_migration(conn: &Connection, migration: &Migration) -> StoreResult<Option<()>> {
    if !schema_migrations_table_exists(conn)? {
        return Ok(None);
    }

    conn.query_row(
        "SELECT name, storage_profile
           FROM schema_migrations
          WHERE database_kind = ?1 AND version = ?2",
        params![migration.database_kind, migration.version],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )
    .optional()?
    .map(|(actual_name, actual_storage_profile)| {
        if actual_name == migration.name && actual_storage_profile == STORAGE_PROFILE {
            Ok(())
        } else if actual_storage_profile != STORAGE_PROFILE {
            Err(StoreError::unsupported_storage_profile(
                migration.database_kind,
                actual_storage_profile,
                STORAGE_PROFILE,
            ))
        } else {
            Err(StoreError::MigrationConflict {
                database_kind: migration.database_kind,
                version: migration.version,
                expected_name: migration.name,
                actual_name,
                expected_storage_profile: STORAGE_PROFILE,
                actual_storage_profile,
            })
        }
    })
    .transpose()
}

fn existing_migration_rows(conn: &Connection) -> StoreResult<Vec<ExistingMigrationRow>> {
    let mut stmt = conn.prepare(
        "SELECT database_kind, version, name, storage_profile
           FROM schema_migrations
          ORDER BY version, database_kind",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ExistingMigrationRow {
            database_kind: row.get(0)?,
            version: row.get(1)?,
            name: row.get(2)?,
            storage_profile: row.get(3)?,
        })
    })?;

    let mut actual = Vec::new();
    for row in rows {
        actual.push(row?);
    }
    Ok(actual)
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

const REGISTRY_INITIAL_SQL: &str = r#"
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
  runtime_home_path TEXT NOT NULL UNIQUE,
  registry_db_path TEXT NOT NULL UNIQUE,
  storage_profile TEXT NOT NULL,
  schema_version INTEGER NOT NULL CHECK (schema_version > 0),
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE installation_profile (
  installation_id TEXT PRIMARY KEY,
  runtime_home_id TEXT NOT NULL UNIQUE,
  volicord_command TEXT NOT NULL,
  volicord_mcp_command TEXT NOT NULL,
  bin_dir TEXT NOT NULL,
  default_connection_mode TEXT NOT NULL CHECK (default_connection_mode IN ('read_only', 'workflow')),
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (runtime_home_id) REFERENCES runtime_home (runtime_home_id) ON DELETE RESTRICT
);

CREATE TABLE projects (
  project_internal_id TEXT PRIMARY KEY,
  project_name TEXT NOT NULL,
  project_alias TEXT NOT NULL UNIQUE,
  runtime_home_id TEXT NOT NULL,
  repo_root TEXT NOT NULL UNIQUE,
  project_home TEXT NOT NULL UNIQUE,
  state_db_path TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL DEFAULT 'active' CHECK (status = 'active'),
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (runtime_home_id) REFERENCES runtime_home (runtime_home_id)
);

CREATE TABLE project_aliases (
  alias TEXT PRIMARY KEY,
  project_internal_id TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (project_internal_id)
    REFERENCES projects (project_internal_id)
    ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_projects_repo_root ON projects (repo_root);
CREATE INDEX idx_projects_status ON projects (status);
CREATE INDEX idx_project_aliases_project
  ON project_aliases (project_internal_id);

CREATE TABLE agent_connections (
  connection_internal_id TEXT PRIMARY KEY,
  host_kind TEXT NOT NULL CHECK (host_kind IN ('codex', 'claude_code', 'generic')),
  intent TEXT NOT NULL CHECK (intent IN ('personal', 'shared', 'global')),
  host_scope TEXT NOT NULL CHECK (host_scope IN ('user', 'project', 'local', 'export')),
  project_internal_id TEXT,
  server_name TEXT NOT NULL,
  config_target TEXT NOT NULL,
  mode TEXT NOT NULL CHECK (mode IN ('read_only', 'workflow')),
  enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  managed_fingerprint TEXT NOT NULL,
  last_verification_status TEXT NOT NULL DEFAULT 'not_verified'
    CHECK (last_verification_status IN ('not_verified', 'complete', 'action_required', 'failed')),
  last_verification_report_json TEXT NOT NULL DEFAULT '{}',
  last_user_actions_json TEXT NOT NULL DEFAULT '[]',
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_internal_id) REFERENCES projects (project_internal_id) ON DELETE RESTRICT,
  CHECK (
    (host_kind = 'codex' AND host_scope IN ('user', 'project'))
    OR (host_kind = 'claude_code' AND host_scope IN ('local', 'project', 'user'))
    OR (host_kind = 'generic' AND host_scope = 'export')
  )
);

CREATE TABLE connection_projects (
  connection_internal_id TEXT NOT NULL,
  project_internal_id TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (connection_internal_id, project_internal_id),
  FOREIGN KEY (connection_internal_id)
    REFERENCES agent_connections (connection_internal_id)
    ON DELETE RESTRICT
    DEFERRABLE INITIALLY DEFERRED,
  FOREIGN KEY (project_internal_id) REFERENCES projects (project_internal_id) ON DELETE RESTRICT
);

CREATE INDEX idx_connection_projects_project
  ON connection_projects (project_internal_id);
CREATE INDEX idx_agent_connections_enabled
  ON agent_connections (enabled);
CREATE INDEX idx_agent_connections_project
  ON agent_connections (project_internal_id);
CREATE UNIQUE INDEX idx_agent_connections_target_project
  ON agent_connections (
    host_kind,
    intent,
    host_scope,
    project_internal_id,
    config_target,
    server_name
  )
  WHERE project_internal_id IS NOT NULL;
CREATE UNIQUE INDEX idx_agent_connections_target_global
  ON agent_connections (
    host_kind,
    intent,
    host_scope,
    config_target,
    server_name
  )
  WHERE project_internal_id IS NULL;
"#;

const REGISTRY_GUARD_RECORDS_SQL: &str = r#"
CREATE TABLE guard_installations (
  guard_installation_id TEXT PRIMARY KEY,
  runtime_home_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  project_internal_id TEXT,
  host_kind TEXT NOT NULL CHECK (length(trim(host_kind)) > 0),
  guard_mode TEXT NOT NULL CHECK (guard_mode IN ('mcp_only', 'guarded', 'managed')),
  host_capability_json TEXT NOT NULL DEFAULT '{}',
  installation_health TEXT NOT NULL
    CHECK (installation_health IN ('unknown', 'healthy', 'action_required', 'failed')),
  installed_at TEXT,
  last_checked_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (runtime_home_id) REFERENCES runtime_home (runtime_home_id) ON DELETE RESTRICT,
  FOREIGN KEY (connection_internal_id)
    REFERENCES agent_connections (connection_internal_id)
    ON DELETE RESTRICT,
  FOREIGN KEY (project_internal_id) REFERENCES projects (project_internal_id) ON DELETE RESTRICT
);

CREATE INDEX idx_guard_installations_connection
  ON guard_installations (connection_internal_id);
CREATE INDEX idx_guard_installations_project
  ON guard_installations (project_internal_id);
CREATE INDEX idx_guard_installations_health
  ON guard_installations (installation_health);
CREATE UNIQUE INDEX idx_guard_installations_scope_project
  ON guard_installations (connection_internal_id, project_internal_id, guard_mode)
  WHERE project_internal_id IS NOT NULL;
CREATE UNIQUE INDEX idx_guard_installations_scope_global
  ON guard_installations (connection_internal_id, guard_mode)
  WHERE project_internal_id IS NULL;

UPDATE runtime_home
   SET schema_version = 2,
       updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
 WHERE schema_version = 1;
"#;

const REGISTRY_GUARD_INSTALLATION_LIFECYCLE_SQL: &str = r#"
ALTER TABLE guard_installations RENAME TO guard_installations_v2;

CREATE TABLE guard_installations (
  guard_installation_id TEXT PRIMARY KEY,
  runtime_home_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  project_internal_id TEXT,
  host_kind TEXT NOT NULL CHECK (length(trim(host_kind)) > 0),
  guard_mode TEXT NOT NULL CHECK (guard_mode IN ('mcp_only', 'guarded', 'managed')),
  host_capability_json TEXT NOT NULL DEFAULT '{}',
  installation_status TEXT NOT NULL
    CHECK (installation_status IN (
      'absent',
      'configured',
      'reload_required',
      'active',
      'degraded',
      'stale',
      'broken'
    )),
  installed_at TEXT,
  last_checked_at TEXT NOT NULL,
  first_seen_at TEXT,
  last_seen_at TEXT,
  last_seen_phase TEXT,
  observed_host_kind TEXT,
  observed_policy_hash TEXT,
  observed_binary_version TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (runtime_home_id) REFERENCES runtime_home (runtime_home_id) ON DELETE RESTRICT,
  FOREIGN KEY (connection_internal_id)
    REFERENCES agent_connections (connection_internal_id)
    ON DELETE RESTRICT,
  FOREIGN KEY (project_internal_id) REFERENCES projects (project_internal_id) ON DELETE RESTRICT
);

INSERT INTO guard_installations (
  guard_installation_id,
  runtime_home_id,
  connection_internal_id,
  project_internal_id,
  host_kind,
  guard_mode,
  host_capability_json,
  installation_status,
  installed_at,
  last_checked_at,
  metadata_json,
  created_at,
  updated_at
)
SELECT
  guard_installation_id,
  runtime_home_id,
  connection_internal_id,
  project_internal_id,
  host_kind,
  guard_mode,
  host_capability_json,
  CASE installation_health
    WHEN 'unknown' THEN 'configured'
    WHEN 'healthy' THEN 'active'
    WHEN 'action_required' THEN 'reload_required'
    WHEN 'failed' THEN 'broken'
  END,
  installed_at,
  last_checked_at,
  metadata_json,
  created_at,
  updated_at
FROM guard_installations_v2;

DROP TABLE guard_installations_v2;

CREATE INDEX idx_guard_installations_connection
  ON guard_installations (connection_internal_id);
CREATE INDEX idx_guard_installations_project
  ON guard_installations (project_internal_id);
CREATE INDEX idx_guard_installations_status
  ON guard_installations (installation_status);
CREATE UNIQUE INDEX idx_guard_installations_scope_project
  ON guard_installations (connection_internal_id, project_internal_id, guard_mode)
  WHERE project_internal_id IS NOT NULL;
CREATE UNIQUE INDEX idx_guard_installations_scope_global
  ON guard_installations (connection_internal_id, guard_mode)
  WHERE project_internal_id IS NULL;

UPDATE runtime_home
   SET schema_version = 3,
       updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
 WHERE schema_version = 2;
"#;

const REGISTRY_LOCAL_WEB_CONSENT_TOKENS_SQL: &str = r#"
CREATE TABLE local_web_consent_tokens (
  token_hash TEXT NOT NULL PRIMARY KEY CHECK (length(token_hash) = 64),
  project_internal_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  judgment_id TEXT NOT NULL,
  capture_basis TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending', 'consumed', 'expired')),
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  consumed_at TEXT,
  completed_at TEXT,
  created_metadata_json TEXT NOT NULL DEFAULT '{}',
  completion_metadata_json TEXT NOT NULL DEFAULT '{}',
  FOREIGN KEY (project_internal_id) REFERENCES projects (project_internal_id) ON DELETE RESTRICT,
  FOREIGN KEY (connection_internal_id)
    REFERENCES agent_connections (connection_internal_id)
    ON DELETE RESTRICT,
  FOREIGN KEY (connection_internal_id, project_internal_id)
    REFERENCES connection_projects (connection_internal_id, project_internal_id)
    ON DELETE RESTRICT,
  CHECK (
    (
      status = 'pending'
      AND consumed_at IS NULL
      AND completed_at IS NULL
    )
    OR (
      status = 'consumed'
      AND consumed_at IS NOT NULL
      AND completed_at IS NOT NULL
    )
    OR (
      status = 'expired'
      AND consumed_at IS NULL
      AND completed_at IS NULL
    )
  )
);

CREATE INDEX idx_local_web_consent_tokens_judgment
  ON local_web_consent_tokens (project_internal_id, judgment_id, status);
CREATE INDEX idx_local_web_consent_tokens_connection
  ON local_web_consent_tokens (connection_internal_id, status, expires_at);
CREATE INDEX idx_local_web_consent_tokens_expiry
  ON local_web_consent_tokens (status, expires_at);

UPDATE runtime_home
   SET schema_version = 4,
       updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
 WHERE schema_version = 3;
"#;

const PROJECT_STATE_INITIAL_SQL: &str = r#"
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
  enforcement_profile_json TEXT NOT NULL DEFAULT '{"profile_id":"baseline_cooperative","guarantee_level":"cooperative","enabled_mechanisms":[],"source":"baseline_scope","status":"active"}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  FOREIGN KEY (project_id, active_task_id)
    REFERENCES tasks (project_id, task_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE tasks (
  project_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  created_by_actor_source TEXT NOT NULL,
  mode TEXT NOT NULL,
  lifecycle_phase TEXT NOT NULL,
  result TEXT,
  title TEXT,
  summary TEXT,
  shaping_summary_json TEXT NOT NULL DEFAULT '{}',
  bounded_context_json TEXT NOT NULL DEFAULT '[]',
  autonomy_boundary_json TEXT NOT NULL DEFAULT '{}',
  scope_revision INTEGER NOT NULL DEFAULT 0 CHECK (scope_revision >= 0),
  close_basis_revision INTEGER NOT NULL DEFAULT 0 CHECK (close_basis_revision >= 0),
  close_basis_json TEXT,
  close_summary_json TEXT NOT NULL DEFAULT '{}',
  completion_policy_json TEXT NOT NULL DEFAULT '{}',
  current_change_unit_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  closed_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, task_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
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
  effect_contract_json TEXT NOT NULL DEFAULT 'null',
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
  status TEXT NOT NULL CHECK (status IN ('pending', 'resolved', 'stale', 'superseded', 'expired')),
  request_json TEXT NOT NULL DEFAULT '{}',
  context_json TEXT NOT NULL DEFAULT '{}',
  options_json TEXT NOT NULL DEFAULT '{"schema_version":1,"options":[]}',
  affected_refs_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  sensitive_action_scope_json TEXT NOT NULL DEFAULT '{}',
  basis_json TEXT NOT NULL,
  basis_status TEXT NOT NULL DEFAULT 'current'
    CHECK (basis_status IN ('current', 'stale', 'superseded')),
  resolution_outcome TEXT
    CHECK (resolution_outcome IS NULL OR resolution_outcome IN ('accepted', 'rejected', 'deferred')),
  resolution_machine_action TEXT
    CHECK (resolution_machine_action IS NULL OR resolution_machine_action IN ('accept', 'reject', 'defer')),
  resolution_json TEXT,
  resolution_rationale_json TEXT,
  requested_by_actor_source TEXT NOT NULL,
  resolved_by_actor_source TEXT,
  resolved_verification_basis TEXT,
  resolved_assurance_level TEXT,
  requested_at TEXT NOT NULL,
  resolved_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, judgment_id),
  CHECK (
    (
      status IN ('pending', 'expired')
      AND resolution_outcome IS NULL
      AND resolution_machine_action IS NULL
      AND resolution_json IS NULL
      AND resolution_rationale_json IS NULL
      AND resolved_by_actor_source IS NULL
      AND resolved_verification_basis IS NULL
      AND resolved_assurance_level IS NULL
      AND resolved_at IS NULL
    )
    OR (
      status = 'resolved'
      AND resolution_outcome IS NOT NULL
      AND resolution_machine_action IS NOT NULL
      AND resolution_json IS NOT NULL
      AND resolution_rationale_json IS NOT NULL
      AND resolved_by_actor_source IS NOT NULL
      AND resolved_verification_basis IS NOT NULL
      AND resolved_assurance_level IS NOT NULL
      AND resolved_at IS NOT NULL
    )
    OR (
      status IN ('stale', 'superseded')
      AND (
        (
          resolution_outcome IS NULL
          AND resolution_machine_action IS NULL
          AND resolution_json IS NULL
          AND resolution_rationale_json IS NULL
          AND resolved_by_actor_source IS NULL
          AND resolved_verification_basis IS NULL
          AND resolved_assurance_level IS NULL
          AND resolved_at IS NULL
        )
        OR (
          resolution_outcome IS NOT NULL
          AND resolution_machine_action IS NOT NULL
          AND resolution_json IS NOT NULL
          AND resolution_rationale_json IS NOT NULL
          AND resolved_by_actor_source IS NOT NULL
          AND resolved_verification_basis IS NOT NULL
          AND resolved_assurance_level IS NOT NULL
          AND resolved_at IS NOT NULL
        )
      )
    )
  ),
  CHECK (
    resolution_machine_action IS NULL
    OR (
      (resolution_machine_action = 'accept' AND resolution_outcome = 'accepted')
      OR (resolution_machine_action = 'reject' AND resolution_outcome = 'rejected')
      OR (resolution_machine_action = 'defer' AND resolution_outcome = 'deferred')
    )
  ),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
);

CREATE TABLE project_continuity_records (
  project_id TEXT NOT NULL,
  continuity_record_id TEXT NOT NULL,
  source_task_id TEXT NOT NULL,
  source_change_unit_id TEXT,
  kind TEXT NOT NULL CHECK (kind IN ('decision', 'obligation', 'known_limit', 'accepted_risk', 'constraint')),
  title TEXT NOT NULL CHECK (length(trim(title)) > 0),
  summary TEXT NOT NULL CHECK (length(trim(summary)) > 0),
  rationale TEXT CHECK (rationale IS NULL OR length(trim(rationale)) > 0),
  applies_to_paths_json TEXT NOT NULL DEFAULT '[]',
  applies_to_refs_json TEXT NOT NULL DEFAULT '[]',
  source_refs_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL CHECK (status IN ('active', 'superseded', 'closed')),
  supersedes_refs_json TEXT NOT NULL DEFAULT '[]',
  review_triggers_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, continuity_record_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, source_task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, source_task_id, source_change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
);

CREATE TABLE write_checks (
  project_id TEXT NOT NULL,
  write_check_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version > 0),
  status TEXT NOT NULL CHECK (status IN ('active', 'consumed', 'expired', 'stale', 'revoked')),
  attempt_scope_json TEXT NOT NULL DEFAULT '{}',
  created_by_actor_source TEXT NOT NULL,
  created_by_judgment_id TEXT,
  expires_at TEXT NOT NULL,
  consumed_by_run_id TEXT,
  consumed_at TEXT,
  revoked_at TEXT,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, write_check_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, created_by_judgment_id)
    REFERENCES user_judgments (project_id, judgment_id),
  FOREIGN KEY (project_id, consumed_by_run_id)
    REFERENCES runs (project_id, run_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE UNIQUE INDEX idx_write_checks_consumed_run
  ON write_checks (project_id, consumed_by_run_id)
  WHERE consumed_by_run_id IS NOT NULL;

CREATE TABLE runs (
  project_id TEXT NOT NULL,
  run_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  write_check_id TEXT,
  kind TEXT NOT NULL,
  status TEXT NOT NULL,
  summary_json TEXT NOT NULL DEFAULT '{}',
  observed_changes_json TEXT NOT NULL DEFAULT '{}',
  evidence_updates_json TEXT NOT NULL DEFAULT '[]',
  write_check_effect_json TEXT NOT NULL DEFAULT '{}',
  scope_revision INTEGER NOT NULL CHECK (scope_revision >= 0),
  created_by_actor_source TEXT NOT NULL,
  started_at TEXT,
  completed_at TEXT,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, run_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, write_check_id)
    REFERENCES write_checks (project_id, write_check_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE UNIQUE INDEX idx_runs_write_check
  ON runs (project_id, write_check_id)
  WHERE write_check_id IS NOT NULL;

CREATE TABLE artifact_staging (
  project_id TEXT NOT NULL,
  handle_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  created_by_actor_source TEXT NOT NULL,
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
  integrity_status TEXT NOT NULL DEFAULT 'verified'
    CHECK (integrity_status IN ('verified', 'corrupt')),
  redaction_state TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('available', 'missing', 'integrity_failed', 'unavailable')),
  retention_json TEXT NOT NULL DEFAULT '{}',
  producer_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, artifact_id),
  CHECK (
    integrity_status <> 'verified'
    OR (
      content_type IS NOT NULL
      AND length(trim(content_type)) > 0
      AND sha256 IS NOT NULL
      AND length(sha256) = 64
      AND sha256 NOT GLOB '*[^0-9a-f]*'
      AND size_bytes IS NOT NULL
      AND size_bytes >= 0
    )
  ),
  CHECK (
    body_path IS NULL
    OR (
      length(trim(body_path)) > 0
      AND body_path NOT GLOB '/*'
      AND body_path NOT GLOB '[A-Za-z]:*'
      AND instr(body_path, '\') = 0
      AND body_path <> '..'
      AND body_path NOT GLOB '../*'
      AND body_path NOT GLOB '*/../*'
      AND body_path NOT GLOB '*/..'
      AND body_path <> 'artifacts'
      AND body_path NOT GLOB 'artifacts/*'
    )
  ),
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
    owner_record_kind IN ('task', 'change_unit', 'run', 'user_judgment', 'evidence_summary', 'evidence_observation', 'blocker')
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

CREATE TABLE evidence_observations (
  project_id TEXT NOT NULL,
  evidence_observation_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  run_id TEXT,
  claim TEXT NOT NULL,
  source_kind TEXT NOT NULL CHECK (
    source_kind IN ('agent_report', 'connection_observation', 'external_tool', 'user_observation', 'reused_evidence', 'unverified_claim')
  ),
  assurance_level TEXT NOT NULL CHECK (
    assurance_level IN ('cooperative_report', 'registered_connection_observed', 'external_tool_result', 'user_observed', 'unverified')
  ),
  observed_by_actor_source TEXT,
  tool_name TEXT,
  tool_invocation_id TEXT,
  tool_metadata_json TEXT NOT NULL DEFAULT '{}',
  input_refs_json TEXT NOT NULL DEFAULT '[]',
  output_artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  limitations_json TEXT NOT NULL DEFAULT '[]',
  observed_at TEXT NOT NULL,
  recorded_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, evidence_observation_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, run_id)
    REFERENCES runs (project_id, run_id)
    DEFERRABLE INITIALLY DEFERRED
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
  actor_source TEXT NOT NULL,
  operation_category TEXT NOT NULL CHECK (operation_category IN ('read', 'agent_workflow', 'user_only', 'admin_local')),
  verification_basis TEXT,
  response_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, tool_name, idempotency_key),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id)
);

CREATE INDEX idx_project_state_active_task
  ON project_state (project_id, active_task_id);

CREATE INDEX idx_tasks_lifecycle
  ON tasks (project_id, lifecycle_phase, result);

CREATE INDEX idx_tasks_current_change_unit
  ON tasks (project_id, current_change_unit_id);

CREATE INDEX idx_change_units_task_status
  ON change_units (project_id, task_id, status);

CREATE INDEX idx_user_judgments_task_status
  ON user_judgments (project_id, task_id, status);

CREATE INDEX idx_project_continuity_records_status
  ON project_continuity_records (project_id, status, kind, updated_at);

CREATE INDEX idx_project_continuity_records_source_task
  ON project_continuity_records (project_id, source_task_id);

CREATE INDEX idx_write_checks_task_status
  ON write_checks (project_id, task_id, status);

CREATE INDEX idx_runs_task_created
  ON runs (project_id, task_id, created_at);

CREATE INDEX idx_artifact_staging_task_status
  ON artifact_staging (project_id, task_id, status);

CREATE INDEX idx_artifact_staging_actor_source
  ON artifact_staging (project_id, created_by_actor_source);

CREATE INDEX idx_artifacts_task_status
  ON artifacts (project_id, task_id, status);

CREATE INDEX idx_artifact_links_owner
  ON artifact_links (project_id, owner_record_kind, owner_record_id);

CREATE INDEX idx_evidence_summaries_task_status
  ON evidence_summaries (project_id, task_id, status);

CREATE INDEX idx_evidence_observations_task_claim
  ON evidence_observations (project_id, task_id, claim);

CREATE INDEX idx_evidence_observations_run
  ON evidence_observations (project_id, run_id);

CREATE INDEX idx_blockers_task_status
  ON blockers (project_id, task_id, status);

CREATE INDEX idx_task_events_task_seq
  ON task_events (project_id, task_id, event_seq);
"#;

const PROJECT_STATE_GUARD_RECORDS_SQL: &str = r#"
CREATE TABLE agent_sessions (
  project_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  guard_installation_id TEXT,
  host_kind TEXT NOT NULL CHECK (length(trim(host_kind)) > 0),
  guard_mode TEXT NOT NULL CHECK (guard_mode IN ('mcp_only', 'guarded', 'managed')),
  started_at TEXT NOT NULL,
  ended_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, session_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id)
);

CREATE TABLE guard_events (
  project_id TEXT NOT NULL,
  guard_event_id TEXT NOT NULL,
  session_id TEXT,
  connection_internal_id TEXT NOT NULL,
  guard_installation_id TEXT,
  event_kind TEXT NOT NULL,
  decision TEXT NOT NULL CHECK (decision IN ('allow', 'deny', 'warn', 'inject_context')),
  subject_json TEXT NOT NULL DEFAULT '{}',
  result_json TEXT NOT NULL DEFAULT '{}',
  occurred_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, guard_event_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, session_id) REFERENCES agent_sessions (project_id, session_id)
);

CREATE TABLE prompt_captures (
  project_id TEXT NOT NULL,
  prompt_capture_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  capture_kind TEXT NOT NULL,
  prompt_sha256 TEXT NOT NULL,
  prompt_text TEXT,
  captured_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, prompt_capture_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, session_id) REFERENCES agent_sessions (project_id, session_id)
);

CREATE TABLE unrecorded_changes (
  project_id TEXT NOT NULL,
  unrecorded_change_id TEXT NOT NULL,
  session_id TEXT,
  connection_internal_id TEXT NOT NULL,
  task_id TEXT,
  status TEXT NOT NULL CHECK (status IN ('unresolved', 'resolved')),
  summary TEXT NOT NULL CHECK (length(trim(summary)) > 0),
  observed_paths_json TEXT NOT NULL DEFAULT '[]',
  detection_json TEXT NOT NULL DEFAULT '{}',
  resolution_json TEXT,
  detected_at TEXT NOT NULL,
  resolved_at TEXT,
  resolved_by_actor_source TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, unrecorded_change_id),
  CHECK (
    (
      status = 'unresolved'
      AND resolution_json IS NULL
      AND resolved_at IS NULL
      AND resolved_by_actor_source IS NULL
    )
    OR (
      status = 'resolved'
      AND resolution_json IS NOT NULL
      AND resolved_at IS NOT NULL
      AND resolved_by_actor_source IS NOT NULL
    )
  ),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, session_id) REFERENCES agent_sessions (project_id, session_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id)
);

CREATE INDEX idx_agent_sessions_connection
  ON agent_sessions (project_id, connection_internal_id);
CREATE INDEX idx_agent_sessions_open
  ON agent_sessions (project_id, connection_internal_id)
  WHERE ended_at IS NULL;
CREATE INDEX idx_guard_events_session
  ON guard_events (project_id, session_id, occurred_at);
CREATE INDEX idx_guard_events_connection
  ON guard_events (project_id, connection_internal_id, occurred_at);
CREATE INDEX idx_guard_events_decision
  ON guard_events (project_id, decision, occurred_at);
CREATE INDEX idx_prompt_captures_session
  ON prompt_captures (project_id, session_id, captured_at);
CREATE INDEX idx_prompt_captures_connection
  ON prompt_captures (project_id, connection_internal_id, captured_at);
CREATE INDEX idx_unrecorded_changes_status
  ON unrecorded_changes (project_id, status, detected_at);
CREATE INDEX idx_unrecorded_changes_connection
  ON unrecorded_changes (project_id, connection_internal_id, status);
CREATE INDEX idx_unrecorded_changes_task
  ON unrecorded_changes (project_id, task_id, status);

UPDATE project_state
   SET schema_version = 2,
       updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
 WHERE schema_version = 1;
"#;

const PROJECT_STATE_EXPECTED_WRITES_SQL: &str = r#"
CREATE TABLE expected_writes (
  project_id TEXT NOT NULL,
  expected_write_id TEXT NOT NULL,
  session_id TEXT,
  connection_internal_id TEXT NOT NULL,
  guard_installation_id TEXT,
  pre_tool_guard_event_id TEXT NOT NULL,
  host_invocation_id TEXT,
  tool_name TEXT,
  command_kind TEXT NOT NULL CHECK (length(trim(command_kind)) > 0),
  path_policy TEXT NOT NULL CHECK (path_policy IN ('exact_paths')),
  expected_paths_json TEXT NOT NULL DEFAULT '[]',
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  write_check_ids_json TEXT NOT NULL DEFAULT '[]',
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version >= 0),
  status TEXT NOT NULL CHECK (status IN ('pending', 'matched')),
  matched_post_tool_guard_event_id TEXT,
  matched_paths_json TEXT,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  matched_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, expected_write_id),
  CHECK (
    (
      status = 'pending'
      AND matched_post_tool_guard_event_id IS NULL
      AND matched_paths_json IS NULL
      AND matched_at IS NULL
    )
    OR (
      status = 'matched'
      AND matched_post_tool_guard_event_id IS NOT NULL
      AND matched_paths_json IS NOT NULL
      AND matched_at IS NOT NULL
    )
  ),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, session_id) REFERENCES agent_sessions (project_id, session_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id)
);

CREATE INDEX idx_expected_writes_pending_connection
  ON expected_writes (project_id, connection_internal_id, status, created_at);
CREATE INDEX idx_expected_writes_session
  ON expected_writes (project_id, session_id, status, created_at);
CREATE INDEX idx_expected_writes_host_invocation
  ON expected_writes (project_id, connection_internal_id, host_invocation_id, status)
  WHERE host_invocation_id IS NOT NULL;
CREATE INDEX idx_expected_writes_task
  ON expected_writes (project_id, task_id, status);

UPDATE project_state
   SET schema_version = 3,
       updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
 WHERE schema_version = 2;
"#;

const PROJECT_STATE_LOCAL_RECOVERY_SQL: &str = r#"
ALTER TABLE tool_invocations RENAME TO tool_invocations_old;

CREATE TABLE tool_invocations (
  project_id TEXT NOT NULL,
  tool_name TEXT NOT NULL,
  idempotency_key TEXT NOT NULL,
  request_hash TEXT NOT NULL,
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version >= 0),
  committed_state_version INTEGER NOT NULL CHECK (committed_state_version > basis_state_version),
  status TEXT NOT NULL DEFAULT 'committed' CHECK (status = 'committed'),
  actor_source TEXT NOT NULL,
  operation_category TEXT NOT NULL CHECK (operation_category IN ('read', 'agent_workflow', 'user_only', 'admin_local', 'local_recovery')),
  verification_basis TEXT,
  response_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, tool_name, idempotency_key),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id)
);

INSERT INTO tool_invocations (
  project_id,
  tool_name,
  idempotency_key,
  request_hash,
  basis_state_version,
  committed_state_version,
  status,
  actor_source,
  operation_category,
  verification_basis,
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
  actor_source,
  operation_category,
  verification_basis,
  response_json,
  created_at
FROM tool_invocations_old;

DROP TABLE tool_invocations_old;

UPDATE project_state
   SET schema_version = 4,
       updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
 WHERE schema_version = 3;
"#;

const PROJECT_STATE_SESSION_WATCH_SQL: &str = r#"
CREATE TABLE session_watch_baselines (
  project_id TEXT NOT NULL,
  watch_baseline_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  guard_installation_id TEXT,
  status TEXT NOT NULL CHECK (status IN ('disabled', 'active', 'degraded', 'unavailable')),
  scope_kind TEXT NOT NULL CHECK (scope_kind IN ('repository', 'path_set')),
  repo_root TEXT NOT NULL CHECK (length(trim(repo_root)) > 0),
  watched_paths_json TEXT NOT NULL DEFAULT '[]',
  exclusions_json TEXT NOT NULL DEFAULT '[]',
  snapshot_algorithm TEXT NOT NULL CHECK (length(trim(snapshot_algorithm)) > 0),
  snapshot_digest TEXT NOT NULL CHECK (length(trim(snapshot_digest)) > 0),
  snapshot_entries_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, watch_baseline_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, session_id) REFERENCES agent_sessions (project_id, session_id)
);

CREATE TABLE session_watch_observations (
  project_id TEXT NOT NULL,
  watch_observation_id TEXT NOT NULL,
  watch_baseline_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  expected_write_id TEXT,
  unrecorded_change_id TEXT,
  observation_status TEXT NOT NULL CHECK (observation_status IN ('unresolved', 'linked')),
  observed_paths_json TEXT NOT NULL DEFAULT '[]',
  change_summary_json TEXT NOT NULL DEFAULT '{}',
  snapshot_algorithm TEXT NOT NULL CHECK (length(trim(snapshot_algorithm)) > 0),
  snapshot_digest TEXT NOT NULL CHECK (length(trim(snapshot_digest)) > 0),
  snapshot_entries_json TEXT NOT NULL DEFAULT '[]',
  observed_at TEXT NOT NULL,
  linked_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, watch_observation_id),
  CHECK (
    (
      observation_status = 'unresolved'
      AND unrecorded_change_id IS NULL
      AND linked_at IS NULL
    )
    OR (
      observation_status = 'linked'
      AND unrecorded_change_id IS NOT NULL
      AND linked_at IS NOT NULL
    )
  ),
  FOREIGN KEY (project_id, watch_baseline_id)
    REFERENCES session_watch_baselines (project_id, watch_baseline_id),
  FOREIGN KEY (project_id, session_id) REFERENCES agent_sessions (project_id, session_id),
  FOREIGN KEY (project_id, expected_write_id)
    REFERENCES expected_writes (project_id, expected_write_id),
  FOREIGN KEY (project_id, unrecorded_change_id)
    REFERENCES unrecorded_changes (project_id, unrecorded_change_id)
);

CREATE INDEX idx_session_watch_baselines_session
  ON session_watch_baselines (project_id, session_id, status);
CREATE INDEX idx_session_watch_baselines_status
  ON session_watch_baselines (project_id, status, updated_at);
CREATE INDEX idx_session_watch_observations_unresolved
  ON session_watch_observations (project_id, session_id, observation_status, observed_at);
CREATE INDEX idx_session_watch_observations_baseline
  ON session_watch_observations (project_id, watch_baseline_id, observed_at);
CREATE INDEX idx_session_watch_observations_expected_write
  ON session_watch_observations (project_id, expected_write_id)
  WHERE expected_write_id IS NOT NULL;
CREATE INDEX idx_session_watch_observations_unrecorded_change
  ON session_watch_observations (project_id, unrecorded_change_id)
  WHERE unrecorded_change_id IS NOT NULL;

UPDATE project_state
   SET schema_version = 5,
       updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
 WHERE schema_version = 4;
"#;

#[cfg(test)]
mod tests {
    use std::{error::Error, fs, path::Path};

    use rusqlite::{params, Connection};
    use sha2::{Digest, Sha256};
    use volicord_test_support::TempRuntimeHome;

    use super::*;
    use crate::sqlite::{
        enable_foreign_keys, open_project_state_database, open_registry_database,
        project_state_db_path, registry_db_path, validate_project_state_schema,
        validate_registry_schema,
    };

    #[test]
    fn expected_migration_catalogs_contain_ordered_rows() {
        assert_eq!(STORAGE_PROFILE, "baseline_sqlite_v3");
        assert_eq!(REGISTRY_SCHEMA_VERSION, 4);
        assert_eq!(PROJECT_STATE_SCHEMA_VERSION, 5);
        assert_eq!(
            expected_registry_migrations(),
            vec![
                ExpectedMigration {
                    database_kind: REGISTRY_DATABASE_KIND,
                    version: 1,
                    name: "registry_initial_v1",
                },
                ExpectedMigration {
                    database_kind: REGISTRY_DATABASE_KIND,
                    version: 2,
                    name: "registry_guard_records_v2",
                },
                ExpectedMigration {
                    database_kind: REGISTRY_DATABASE_KIND,
                    version: 3,
                    name: "registry_guard_installation_lifecycle_v3",
                },
                ExpectedMigration {
                    database_kind: REGISTRY_DATABASE_KIND,
                    version: 4,
                    name: "registry_local_web_consent_tokens_v4",
                }
            ]
        );
        assert_eq!(
            expected_project_state_migrations(),
            vec![
                ExpectedMigration {
                    database_kind: PROJECT_STATE_DATABASE_KIND,
                    version: 1,
                    name: "project_state_initial_v1",
                },
                ExpectedMigration {
                    database_kind: PROJECT_STATE_DATABASE_KIND,
                    version: 2,
                    name: "project_state_guard_records_v2",
                },
                ExpectedMigration {
                    database_kind: PROJECT_STATE_DATABASE_KIND,
                    version: 3,
                    name: "project_state_expected_writes_v3",
                },
                ExpectedMigration {
                    database_kind: PROJECT_STATE_DATABASE_KIND,
                    version: 4,
                    name: "project_state_local_recovery_v4",
                },
                ExpectedMigration {
                    database_kind: PROJECT_STATE_DATABASE_KIND,
                    version: 5,
                    name: "project_state_session_watch_v5",
                }
            ]
        );
    }

    #[test]
    fn registry_initial_migration_is_idempotent() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("migration-registry-initial")?;
        let path = registry_db_path(runtime_home.path());

        let conn = open_registry_database(&path)?;
        validate_registry_schema(&conn)?;
        assert_migrations(
            &conn,
            REGISTRY_DATABASE_KIND,
            &[
                "registry_initial_v1",
                "registry_guard_records_v2",
                "registry_guard_installation_lifecycle_v3",
                "registry_local_web_consent_tokens_v4",
            ],
        )?;
        drop(conn);

        let conn = open_registry_database(&path)?;
        validate_registry_schema(&conn)?;
        assert_migrations(
            &conn,
            REGISTRY_DATABASE_KIND,
            &[
                "registry_initial_v1",
                "registry_guard_records_v2",
                "registry_guard_installation_lifecycle_v3",
                "registry_local_web_consent_tokens_v4",
            ],
        )?;
        assert!(table_exists(&conn, "agent_connections")?);
        assert!(table_exists(&conn, "connection_projects")?);
        assert!(table_exists(&conn, "guard_installations")?);
        assert!(table_exists(&conn, "local_web_consent_tokens")?);
        Ok(())
    }

    #[test]
    fn project_state_initial_migration_is_idempotent() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("migration-project-state-initial")?;
        let path = project_state_db_path(runtime_home.path(), "project_initial");

        let conn = open_project_state_database(&path)?;
        validate_project_state_schema(&conn)?;
        assert_migrations(
            &conn,
            PROJECT_STATE_DATABASE_KIND,
            &[
                "project_state_initial_v1",
                "project_state_guard_records_v2",
                "project_state_expected_writes_v3",
                "project_state_local_recovery_v4",
                "project_state_session_watch_v5",
            ],
        )?;
        drop(conn);

        let conn = open_project_state_database(&path)?;
        validate_project_state_schema(&conn)?;
        assert_migrations(
            &conn,
            PROJECT_STATE_DATABASE_KIND,
            &[
                "project_state_initial_v1",
                "project_state_guard_records_v2",
                "project_state_expected_writes_v3",
                "project_state_local_recovery_v4",
                "project_state_session_watch_v5",
            ],
        )?;
        assert!(table_exists(&conn, "tool_invocations")?);
        assert!(table_exists(&conn, "agent_sessions")?);
        assert!(table_exists(&conn, "guard_events")?);
        assert!(table_exists(&conn, "prompt_captures")?);
        assert!(table_exists(&conn, "expected_writes")?);
        assert!(table_exists(&conn, "unrecorded_changes")?);
        assert!(table_exists(&conn, "session_watch_baselines")?);
        assert!(table_exists(&conn, "session_watch_observations")?);
        assert!(column_exists(
            &conn,
            "project_state",
            "enforcement_profile_json"
        )?);
        assert!(table_exists(&conn, "write_checks")?);
        assert!(column_exists(&conn, "tasks", "created_by_actor_source")?);
        assert!(column_exists(
            &conn,
            "user_judgments",
            "resolution_machine_action"
        )?);
        assert!(column_exists(
            &conn,
            "user_judgments",
            "requested_by_actor_source"
        )?);
        assert!(column_exists(
            &conn,
            "tool_invocations",
            "operation_category"
        )?);
        assert!(table_exists(&conn, "project_continuity_records")?);
        assert!(!column_exists(&conn, "tasks", "state_version")?);
        Ok(())
    }

    #[test]
    fn registry_old_profile_is_rejected_without_modification() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("migration-registry-old-profile")?;
        let path = registry_db_path(runtime_home.path());
        {
            let conn = open_registry_database(&path)?;
            conn.execute(
                "UPDATE schema_migrations SET storage_profile = ?1",
                [OLD_STORAGE_PROFILE],
            )?;
            conn.execute(
                "INSERT INTO runtime_home (
                    singleton_id,
                    runtime_home_id,
                    runtime_home_path,
                    registry_db_path,
                    storage_profile,
                    schema_version,
                    metadata_json,
                    created_at,
                    updated_at
                )
                VALUES (1, 'runtime_home_old_profile', ?1, ?2, ?3, 1, '{}', 't0', 't0')",
                rusqlite::params![
                    runtime_home.path().to_string_lossy().as_ref(),
                    path.to_string_lossy().as_ref(),
                    OLD_STORAGE_PROFILE
                ],
            )?;
        }
        let hash_before = file_hash(&path)?;

        let error = open_registry_database(&path)
            .expect_err("old registry storage profile should be rejected");

        assert!(matches!(
            error,
            StoreError::UnsupportedStorageProfile { .. }
        ));
        assert!(error.to_string().contains("explicitly reinitialize"));
        assert_eq!(file_hash(&path)?, hash_before);
        assert_eq!(migration_count(&path, REGISTRY_DATABASE_KIND)?, 4);
        assert_eq!(stored_profile(&path, "runtime_home")?, OLD_STORAGE_PROFILE);
        Ok(())
    }

    #[test]
    fn project_state_old_profile_is_rejected_without_modification() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("migration-state-old-profile")?;
        let path = project_state_db_path(runtime_home.path(), "project_old_profile");
        {
            let conn = open_project_state_database(&path)?;
            conn.execute(
                "UPDATE schema_migrations SET storage_profile = ?1",
                [OLD_STORAGE_PROFILE],
            )?;
            conn.execute(
                "INSERT INTO project_state (
                    project_id,
                    storage_profile,
                    schema_version,
                    created_at,
                    updated_at
                )
                VALUES ('project_old_profile', ?1, 1, 't0', 't0')",
                [OLD_STORAGE_PROFILE],
            )?;
        }
        let hash_before = file_hash(&path)?;

        let error = open_project_state_database(&path)
            .expect_err("old project-state storage profile should be rejected");

        assert!(matches!(
            error,
            StoreError::UnsupportedStorageProfile { .. }
        ));
        assert!(error.to_string().contains("explicitly reinitialize"));
        assert_eq!(file_hash(&path)?, hash_before);
        assert_eq!(migration_count(&path, PROJECT_STATE_DATABASE_KIND)?, 5);
        assert_eq!(stored_profile(&path, "project_state")?, OLD_STORAGE_PROFILE);
        Ok(())
    }

    #[test]
    fn future_migration_row_is_rejected_without_repair() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("migration-future-row")?;
        let path = project_state_db_path(runtime_home.path(), "project_future");
        {
            let conn = open_project_state_database(&path)?;
            conn.execute(
                "INSERT INTO schema_migrations (
                    database_kind,
                    version,
                    name,
                    storage_profile,
                    applied_at
                )
                VALUES (?1, 999, 'project_state_future_v999', ?2, 't_future')",
                params![PROJECT_STATE_DATABASE_KIND, STORAGE_PROFILE],
            )?;
        }
        let hash_before = file_hash(&path)?;

        let error =
            open_project_state_database(&path).expect_err("future migration should be rejected");

        assert!(matches!(error, StoreError::SchemaInvariant { .. }));
        assert!(error.to_string().contains("newer than supported"));
        assert_eq!(file_hash(&path)?, hash_before);
        assert_eq!(migration_count(&path, PROJECT_STATE_DATABASE_KIND)?, 6);
        Ok(())
    }

    #[test]
    fn migration_name_mismatch_is_rejected() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("migration-name-mismatch")?;
        let path = registry_db_path(runtime_home.path());
        {
            let conn = open_registry_database(&path)?;
            conn.execute(
                "UPDATE schema_migrations SET name = 'registry_other_v1'",
                [],
            )?;
        }

        let error = open_registry_database(&path).expect_err("name mismatch should be rejected");

        assert!(matches!(error, StoreError::MigrationConflict { .. }));
        Ok(())
    }

    fn assert_migrations(
        conn: &Connection,
        database_kind: &str,
        expected_names: &[&str],
    ) -> rusqlite::Result<()> {
        let mut stmt = conn.prepare(
            "SELECT version, name, storage_profile
               FROM schema_migrations
              WHERE database_kind = ?1
              ORDER BY version",
        )?;
        let rows = stmt.query_map([database_kind], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut actual = Vec::new();
        for row in rows {
            actual.push(row?);
        }
        let expected = expected_names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                (
                    i64::try_from(index + 1).expect("migration index fits"),
                    (*name).to_owned(),
                    STORAGE_PROFILE.to_owned(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
        assert_eq!(
            migration_count_from_conn(conn, database_kind)?,
            i64::try_from(expected_names.len()).expect("migration count fits")
        );
        Ok(())
    }

    fn migration_count_from_conn(conn: &Connection, database_kind: &str) -> rusqlite::Result<i64> {
        conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE database_kind = ?1",
            [database_kind],
            |row| row.get(0),
        )
    }

    fn migration_count(path: &Path, database_kind: &str) -> rusqlite::Result<i64> {
        let conn = Connection::open(path)?;
        migration_count_from_conn(&conn, database_kind)
    }

    fn table_exists(conn: &Connection, table: &str) -> rusqlite::Result<bool> {
        conn.query_row(
            "SELECT COUNT(*)
               FROM sqlite_master
              WHERE type = 'table' AND name = ?1",
            [table],
            |row| Ok(row.get::<_, i64>(0)? == 1),
        )
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

    fn file_hash(path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(Sha256::digest(fs::read(path)?).to_vec())
    }

    fn stored_profile(path: &Path, table: &str) -> rusqlite::Result<String> {
        let conn = Connection::open(path)?;
        let sql = format!(
            "SELECT storage_profile FROM {} LIMIT 1",
            table.replace('"', "\"\"")
        );
        conn.query_row(&sql, [], |row| row.get(0))
    }

    #[test]
    fn direct_apply_accepts_existing_current_connection() -> Result<(), Box<dyn Error>> {
        let mut conn = Connection::open_in_memory()?;
        enable_foreign_keys(&conn)?;

        apply_project_state_migrations(&mut conn)?;
        apply_project_state_migrations(&mut conn)?;
        validate_project_state_schema(&conn)?;

        assert_migrations(
            &conn,
            PROJECT_STATE_DATABASE_KIND,
            &[
                "project_state_initial_v1",
                "project_state_guard_records_v2",
                "project_state_expected_writes_v3",
                "project_state_local_recovery_v4",
                "project_state_session_watch_v5",
            ],
        )?;
        Ok(())
    }
}
