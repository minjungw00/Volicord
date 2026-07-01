# Storage DDL

This document owns the baseline SQLite DDL contract for the storage layout described by [Storage Records](storage-records.md). It makes the baseline `registry.sqlite` and project `state.sqlite` layouts implementable without moving method effects, artifact lifecycle rules, state-version meaning, API schemas, or security guarantees into this document.

## Owner Boundaries

This document owns:

- baseline SQLite table shape for `registry.sqlite` and project `state.sqlite`
- baseline indexes, foreign keys, migration tables, and physical constraints
- SQLite constraints for `project_state.state_version`, replay rows, current Change Unit uniqueness, Write Check basis versions, staged artifact provenance, and guarded-operation records
- the DDL-level split between Runtime Home registration data and project-local Core state

This document does not own:

- record-family purpose, storage locations, storage-owned values, or JSON placement categories; see [Storage Records](storage-records.md)
- method branch storage effects; see [Storage Effects](storage-effects.md)
- artifact staging, promotion, linking, body reads, retention, or integrity lifecycle; see [Artifact Storage](storage-artifacts.md)
- state-version, idempotency, event, lock, or migration semantics; see [Storage Versioning](storage-versioning.md)
- API request or response schemas; see the API schema owners routed from [API Schema Core](api/schema-core.md)
- runtime location boundaries; see [Runtime Boundaries](runtime-boundaries.md)
- security guarantee levels; see [Security](security.md)

## Connection And Transaction Requirements

SQLite foreign keys are part of this DDL contract. Every connection that reads or writes these databases must enable:

```sql
PRAGMA foreign_keys = ON;
```

Mutating transactions must use `BEGIN IMMEDIATE` or an equivalent serialized write boundary before reading freshness, Write Check, staging, or replay rows for a state-changing commit.

Baseline authority rows remain addressable unless an owning storage or migration contract defines a repair or retention path. The registry may cascade-delete non-authority alias rows that are owned by a forgotten project registration; it must not use alias cleanup to imply deletion of project-local Core authority records.

SQLite `TEXT` columns ending in `_json` store JSON as a representation choice. JSON used for authority, lifecycle, scope, evidence, completion, close readiness, or write compatibility is typed owner state. Typed Core code must parse and validate those columns before commit against the applicable API schema owner, storage owner, or artifact owner. Failure to decode typed owner state is corruption and must never be converted to an empty object, empty array, false value, default enum, or "no requirement" interpretation. SQL `NULL` may mean absence only when the owning schema explicitly marks the field optional; malformed JSON in an optional column is corruption, not absence. Open-ended display metadata may remain untyped only when it is not used for authority or close decisions. Safe diagnostics may identify the table, record reference, logical column, and corruption category, but must not expose raw stored JSON, secrets, SQL text, or sensitive absolute paths. SQLite defaults such as `'{}'` and `'[]'` do not make API fields optional.

`project_state.state_version` is the only public baseline state clock. Baseline SQLite DDL must not create `tasks.state_version`.

Write Check rows record Core-state compatibility for a product-file write attempt. They are not OS permissions, filesystem ACLs, sandboxing, network policy, or secret isolation.

## `registry.sqlite`

`registry.sqlite` stores Runtime Home identity, installation profile records, project registration, project aliases, Agent Connection records, Connection Projects membership, guard installation records, and host configuration inventory. It does not store project-local Core state.

Applying the current migrations produces registry schema version `4` for storage profile `baseline_sqlite_v3`. The first DDL block is the initial physical registry schema version `1`; the guard-record additions after it are schema version `2`, the guard-installation lifecycle replacement is schema version `3`, and local web consent tokens are schema version `4`. Storage profile and migration boundary behavior are owned by [Storage Versioning](storage-versioning.md).

```sql
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
```

Registry schema version `2` adds guarded-operation setup records. Registry
schema version `3` replaces the earlier guard installation state column with
the explicit lifecycle and observation fields shown here:

```sql
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
```

The version `3` registry migration updates existing `runtime_home.schema_version` rows from `2` to `3`.

Registry schema version `4` adds local web consent token records for pending
user judgments:

```sql
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
```

The version `4` registry migration updates existing `runtime_home.schema_version` rows from `3` to `4`.

Registry constraints:

- `runtime_home` is a singleton table. It stores Runtime Home identity, the Runtime Home path, the registry database path, storage profile, schema version, metadata, and timestamps. The stored `runtime_home_id` identifies the Runtime Home record; it is not a security guarantee.
- `installation_profile` stores the selected `volicord` command, MCP launch command, bin directory, default connection mode, metadata, and timestamps for the Runtime Home. It may be established by `volicord init` or `volicord setup`. It is not host trust, user authority, or public API state.
- `projects.project_internal_id` is the storage primary key for project records. `projects.project_name` is the display name. `projects.project_alias` is the CLI selection aid. `projects.repo_root` is the repository-root lookup key. `projects.project_alias`, `projects.repo_root`, `projects.project_home`, and `projects.state_db_path` are unique.
- `project_aliases` maps aliases to `project_internal_id` values. Alias rows are registry selection aids, not project-local Core authority records.
- `projects.state_db_path` is retained as a stored column. Store application-level current-registration validation must confirm it equals `project_home/state.sqlite` before operational `ProjectRecord` lookup or listing, project-state migration or writable open, Agent Connection project routing, Core execution, profile reuse, or MCP project availability.
- `projects.status` is storage-owned and baseline-valid only as `active`.
- `agent_connections.connection_internal_id` is the storage primary key for Agent Connection records. The table stores host kind, connection intent in `intent`, host scope, optional `project_internal_id`, server name, config target, mode, enabled state, managed fingerprint, verification summary status, verification report JSON, user actions JSON, metadata, and timestamps.
- `agent_connections.intent` is constrained to `personal`, `shared`, or `global`.
- `agent_connections.host_scope` is constrained with `host_kind`: Codex supports `user` and `project`; Claude Code supports `local`, `project`, and `user`; generic export supports `export`.
- `agent_connections.mode` is constrained to `read_only` or `workflow`.
- `agent_connections.last_verification_report_json` stores the latest verification report JSON object. `agent_connections.last_user_actions_json` stores the latest user-action JSON array.
- `connection_projects` is the explicit project allowlist for one Agent Connection. It stores membership with `connection_internal_id` and `project_internal_id`. Deleting a project or connection that still has membership is restricted.
- `guard_installations` stores local guard setup lifecycle state and host capability for one Runtime Home, Agent Connection, and optional project scope. Its `guard_mode` values are `mcp_only`, `guarded`, and `managed`. Its `installation_status` values are `absent`, `configured`, `reload_required`, `active`, `degraded`, `stale`, and `broken`. A valid observed guard hook for the recorded project, Agent Connection, host kind, guard mode, policy hash, and known hook phase records first-seen and last-seen metadata. It can move a row to `active` only when required hook configuration is complete and the row is not `degraded`, `stale`, or `broken`; otherwise the observation metadata is recorded without making the installation effectively active. These rows are local authority records for guarded operation; they are not OS-level enforcement proof or write-prevention proof.
- `local_web_consent_tokens` stores hashed one-time local web consent tokens for pending user judgments. Rows are scoped to a registered project, Agent Connection, and Connection Projects membership. The raw token is not stored. `status` is `pending`, `consumed`, or `expired`; consumed rows must have completion timestamps, and pending or expired rows must not. These rows are transient User Channel capture metadata and are not Core judgment authority by themselves.
- `schema_migrations` records applied registry schema versions. Migration execution semantics stay with [Storage Versioning](storage-versioning.md).

## Project `state.sqlite`

Each registered project has one project-local `state.sqlite`. It stores Core state for that project and repeats `project_id` in project-scoped rows so foreign keys and indexes can enforce same-project relationships.

Applying the current migrations produces project-state schema version `5` for storage profile `baseline_sqlite_v3`. The main DDL block below shows the current table layout after applied migrations; guarded-operation records were introduced in schema version `2`, expected-write correlation records in schema version `3`, local-recovery replay-category support in schema version `4`, and session-level Product Repository watch records in schema version `5`. Storage profile and migration boundary behavior are owned by [Storage Versioning](storage-versioning.md).

```sql
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
  operation_category TEXT NOT NULL CHECK (operation_category IN ('read', 'agent_workflow', 'user_only', 'admin_local', 'local_recovery')),
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
```

Project-state schema version `2` adds guarded-operation records:

```sql
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
```

The version `2` project-state migration updates existing `project_state.schema_version` rows from `1` to `2`.

Project-state schema version `3` adds expected-write correlation records:

```sql
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
```

The version `3` project-state migration updates existing `project_state.schema_version` rows from `2` to `3`.

Project-state schema version `4` rebuilds `tool_invocations` with `local_recovery` added to the `operation_category` constraint, preserves existing replay rows, and updates existing `project_state.schema_version` rows from `3` to `4`.

Project-state schema version `5` adds session-level Product Repository watch records:

```sql
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
```

The version `5` project-state migration updates existing `project_state.schema_version` rows from `4` to `5`.

Project-state constraints:

- `project_state.state_version` is the only public baseline state clock and must be monotonic according to [Storage Versioning](storage-versioning.md).
- `tasks.created_by_actor_source`, `user_judgments.requested_by_actor_source`, `user_judgments.resolved_by_actor_source`, `write_checks.created_by_actor_source`, `runs.created_by_actor_source`, `artifact_staging.created_by_actor_source`, `evidence_observations.observed_by_actor_source`, and `tool_invocations.actor_source` store actor provenance.
- `tool_invocations.operation_category` is constrained to `read`, `agent_workflow`, `user_only`, `admin_local`, or `local_recovery`.
- User judgment rows store User Channel provenance for authority-bearing resolution. `status='resolved'` records that an answer exists; approval meaning comes from the stored machine action, outcome, basis, provenance, and method owner.
- `write_checks` records single-use Core-state write compatibility. The unique indexes on `write_checks.consumed_by_run_id` and `runs.write_check_id` prevent one Write Check consumption from forking across multiple runs.
- `artifact_staging.created_by_actor_source` records staging provenance. Staged bytes and notices remain artifact-owned and are not evidence authority by themselves.
- `evidence_observations.source_kind` and `assurance_level` distinguish cooperative agent reports, registered connection observations, external tool results, user observations, reused evidence, and unverified claims.
- `tool_invocations` stores replay rows with actor provenance and operation category. Replay rows are not caller authority and do not bypass current connection context or User Channel requirements.
- `agent_sessions`, `guard_events`, `prompt_captures`, `expected_writes`, `unrecorded_changes`, `session_watch_baselines`, and `session_watch_observations` are project-local guarded-operation and session-watch records. They repeat `connection_internal_id` for connection scoping and use project-local keys so records do not leak across projects.
- `guard_events.decision` is constrained to `allow`, `deny`, `warn`, or `inject_context`. These values record local guard decisions; they are not OS-level enforcement proof.
- `expected_writes.status` is constrained to `pending` or `matched`, and `path_policy` is constrained to `exact_paths`. Matched rows must carry the matched post-tool guard event, matched paths JSON, and `matched_at`; pending rows must not carry those matched fields.
- `unrecorded_changes.status` is constrained to `unresolved` or `resolved`. Resolved rows must carry resolution JSON, `resolved_at`, and `resolved_by_actor_source`; unresolved rows must not carry those resolution fields. Resolution JSON must include the compact resolution basis and capture basis required by [Storage Records](storage-records.md), without storing full sensitive command or prompt content.
- `session_watch_baselines.status` is constrained to `disabled`, `active`, `degraded`, or `unavailable`, and `scope_kind` is constrained to `repository` or `path_set`.
- `session_watch_observations.observation_status` is constrained to `unresolved` or `linked`. Linked rows must carry `unrecorded_change_id` and `linked_at`; unresolved rows must not carry those link fields.

## Related Owners

- [Storage Records](storage-records.md) defines persisted record families, placement, relationship layout, storage-owned values, and JSON placement.
- [Storage Effects](storage-effects.md) defines which method branches create, update, observe, or leave records untouched.
- [Storage Versioning](storage-versioning.md) defines state versioning, idempotency, replay, events, locks, and migration contracts.
- [Agent Connection](agent-connection.md) defines Agent Connection, Connection Projects, current connection context, mode gating, and Agent Connection versus User Channel boundaries.
- [Security](security.md) defines security boundaries and guarantee levels.
