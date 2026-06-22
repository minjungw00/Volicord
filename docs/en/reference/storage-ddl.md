# Storage DDL

This document owns the baseline SQLite DDL contract for the storage layout described by [Storage Records](storage-records.md). It makes the baseline `registry.sqlite` and project `state.sqlite` layouts implementable without moving method effects, artifact lifecycle rules, state-version meaning, API schemas, or security guarantees into this document.

## Owner boundaries

This document owns:

- baseline SQLite table shape for `registry.sqlite` and project `state.sqlite`
- baseline indexes, foreign keys, migration tables, and physical constraints
- SQLite constraints for `project_state.state_version`, replay rows, current Change Unit uniqueness, `Write Authorization` basis versions, and staged artifact provenance
- the DDL-level split between Runtime Home registration data and project-local Core state

This document does not own:

- record-family purpose, storage locations, storage-owned values, or JSON placement categories; see [Storage Records](storage-records.md)
- method branch storage effects; see [Storage Effects](storage-effects.md)
- artifact staging, promotion, linking, body reads, retention, or integrity lifecycle; see [Artifact Storage](storage-artifacts.md)
- state-version, idempotency, event, lock, or migration semantics; see [Storage Versioning](storage-versioning.md)
- API request or response schemas; see the API schema owners routed from [API Schema Core](api/schema-core.md)
- runtime location boundaries; see [Runtime Boundaries](runtime-boundaries.md)
- security guarantee levels; see [Security](security.md)

## Connection and transaction requirements

SQLite foreign keys are part of this DDL contract. Every connection that reads or writes these databases must enable:

```sql
PRAGMA foreign_keys = ON;
```

Mutating transactions must use `BEGIN IMMEDIATE` or an equivalent serialized write boundary before reading freshness, authorization, staging, or replay rows for a state-changing commit.

No baseline table uses `ON DELETE CASCADE`. Authority rows remain addressable unless an owning storage or migration contract defines a repair or retention path.

SQLite `TEXT` columns ending in `_json` store JSON as a representation choice. JSON used for authority, lifecycle, scope, evidence, completion, close readiness, or write compatibility is typed owner state. Typed Core code must parse and validate those columns before commit against the applicable API schema owner, storage owner, or artifact owner. Failure to decode typed owner state is corruption and must never be converted to an empty object, empty array, false value, default enum, or "no requirement" interpretation. SQL `NULL` may mean absence only when the owning schema explicitly marks the field optional; malformed JSON in an optional column is corruption, not absence. Open-ended display metadata may remain untyped only when it is not used for authority or close decisions. Safe diagnostics may identify the table, record reference, logical column, and corruption category, but must not expose raw stored JSON, secrets, SQL text, or sensitive absolute paths. SQLite defaults such as `'{}'` and `'[]'` do not make API fields optional.

`project_state.state_version` is the only public baseline state clock. Baseline SQLite DDL must not create `tasks.state_version`.

## `registry.sqlite`

`registry.sqlite` stores Runtime Home identity, project registration, coding-agent integration registry records, and host setup inventory. It does not store project-local Core state.

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

CREATE TABLE agent_integrations (
  integration_id TEXT PRIMARY KEY,
  interaction_role TEXT NOT NULL CHECK (interaction_role = 'agent'),
  surface_id TEXT NOT NULL,
  surface_instance_id TEXT NOT NULL,
  default_project_id TEXT,
  enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  FOREIGN KEY (integration_id, default_project_id)
    REFERENCES integration_projects (integration_id, project_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE integration_projects (
  integration_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (integration_id, project_id),
  FOREIGN KEY (integration_id)
    REFERENCES agent_integrations (integration_id)
    ON DELETE RESTRICT
    DEFERRABLE INITIALLY DEFERRED,
  FOREIGN KEY (project_id) REFERENCES projects (project_id) ON DELETE RESTRICT
);

CREATE TABLE host_installations (
  installation_id TEXT PRIMARY KEY,
  integration_id TEXT NOT NULL,
  host_kind TEXT NOT NULL CHECK (host_kind IN ('codex', 'claude_code', 'generic')),
  host_scope TEXT NOT NULL CHECK (host_scope IN ('user', 'project', 'local', 'export')),
  server_name TEXT NOT NULL,
  config_target TEXT NOT NULL,
  managed_fingerprint TEXT NOT NULL,
  last_verified_status TEXT NOT NULL DEFAULT 'not_verified'
    CHECK (last_verified_status IN ('not_verified', 'complete', 'action_required', 'partial_failure', 'failed')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  CHECK (
    (host_kind = 'codex' AND host_scope IN ('user', 'project'))
    OR (host_kind = 'claude_code' AND host_scope IN ('local', 'project', 'user'))
    OR (host_kind = 'generic' AND host_scope = 'export')
  ),
  FOREIGN KEY (integration_id) REFERENCES agent_integrations (integration_id) ON DELETE RESTRICT
);

CREATE INDEX idx_integration_projects_project
  ON integration_projects (project_id);
CREATE INDEX idx_agent_integrations_enabled
  ON agent_integrations (enabled);
CREATE INDEX idx_host_installations_integration
  ON host_installations (integration_id);
CREATE UNIQUE INDEX idx_host_installations_target
  ON host_installations (host_kind, host_scope, config_target, server_name);
```

Registry constraints:

- `runtime_home` is a singleton table. The stored `runtime_home_id` identifies the Runtime Home record; it is not a security guarantee.
- `projects.project_home` is unique. `repo_root` is indexed for lookup but does not replace project identity.
- `projects.state_db_path` is retained as a stored column. The SQL column definition does not enforce its equality with `project_home/state.sqlite`; Store application-level execution validation enforces that relationship before project-state inspection, migration, writable open, surface management, Core execution, setup reuse, or MCP project startup. A mismatching registry row remains readable for diagnosis but is execution-ineligible.
- `projects.status` is storage-owned and baseline-valid only as `active`.
- `agent_integrations` stores the integration-bound MCP process identity and bound surface identifiers. The registry database cannot foreign-key these surface identifiers into project-local `state.sqlite`; per-project execution validation must verify compatible surface registration before adapter calls enter Core.
- `agent_integrations.default_project_id`, when present, is physically constrained to an `integration_projects` row for the same `integration_id`. The foreign key is deferrable so creation can insert the profile and membership in one transaction.
- `integration_projects` is the explicit project allowlist for one Agent Integration Profile. Deleting a project or integration that still has membership is restricted.
- `host_installations` records managed host setup inventory and last verification status. It is not the operational source of truth for Codex, Claude Code, or any generic host configuration file.
- `host_installations.host_kind` and `host_installations.host_scope` are constrained to the supported host/scope matrix defined by [Administrative CLI](admin-cli.md).
- `schema_migrations` records applied registry schema versions. Migration execution semantics stay with [Storage Versioning](storage-versioning.md).

## Project `state.sqlite`

Each registered project has one project-local `state.sqlite`. It stores Core state for that project and repeats `project_id` in project-scoped rows so foreign keys and indexes can enforce same-project relationships.

The DDL below is the initial physical project-state schema for storage profile `baseline_sqlite_v2` schema version `1`. Storage profile and migration boundary behavior are owned by [Storage Versioning](storage-versioning.md).

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
  default_surface_id TEXT,
  default_surface_instance_id TEXT,
  enforcement_profile_json TEXT NOT NULL DEFAULT '{"profile_id":"baseline_cooperative","guarantee_level":"cooperative","enabled_mechanisms":[],"source":"baseline_scope","status":"active"}',
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
  interaction_role TEXT NOT NULL DEFAULT 'agent' CHECK (interaction_role IN ('agent', 'user_interaction')),
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
  requested_by_surface_id TEXT NOT NULL,
  requested_by_surface_instance_id TEXT NOT NULL,
  resolved_by_actor_kind TEXT CHECK (resolved_by_actor_kind IS NULL OR resolved_by_actor_kind IN ('agent', 'user')),
  resolved_actor_role TEXT CHECK (resolved_actor_role IS NULL OR resolved_actor_role IN ('agent', 'user_interaction')),
  resolved_by_surface_id TEXT,
  resolved_by_surface_instance_id TEXT,
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
      AND resolved_by_actor_kind IS NULL
      AND resolved_actor_role IS NULL
      AND resolved_by_surface_id IS NULL
      AND resolved_by_surface_instance_id IS NULL
      AND resolved_verification_basis IS NULL
      AND resolved_assurance_level IS NULL
      AND resolved_at IS NULL
    )
    OR (
      status = 'resolved'
      AND resolution_outcome IS NOT NULL
      AND resolution_machine_action IS NOT NULL
      AND resolution_json IS NOT NULL
      AND resolved_by_actor_kind IS NOT NULL
      AND resolved_actor_role IS NOT NULL
      AND resolved_by_surface_id IS NOT NULL
      AND resolved_by_surface_instance_id IS NOT NULL
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
          AND resolved_by_actor_kind IS NULL
          AND resolved_actor_role IS NULL
          AND resolved_by_surface_id IS NULL
          AND resolved_by_surface_instance_id IS NULL
          AND resolved_verification_basis IS NULL
          AND resolved_assurance_level IS NULL
          AND resolved_at IS NULL
        )
        OR (
          resolution_outcome IS NOT NULL
          AND resolution_machine_action IS NOT NULL
          AND resolution_json IS NOT NULL
          AND resolved_by_actor_kind IS NOT NULL
          AND resolved_actor_role IS NOT NULL
          AND resolved_by_surface_id IS NOT NULL
          AND resolved_by_surface_instance_id IS NOT NULL
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
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, requested_by_surface_id, requested_by_surface_instance_id)
    REFERENCES surfaces (project_id, surface_id, surface_instance_id),
  FOREIGN KEY (project_id, resolved_by_surface_id, resolved_by_surface_instance_id)
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
  scope_revision INTEGER NOT NULL CHECK (scope_revision >= 0),
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
  surface_id TEXT NOT NULL,
  surface_instance_id TEXT NOT NULL,
  access_class TEXT NOT NULL,
  verification_basis TEXT,
  response_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, tool_name, idempotency_key),
  FOREIGN KEY (project_id, surface_id, surface_instance_id)
    REFERENCES surfaces (project_id, surface_id, surface_instance_id)
    ON DELETE RESTRICT,
  FOREIGN KEY (project_id) REFERENCES project_state (project_id)
);
```

Baseline indexes for project state:

```sql
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
```

## Constraint notes

`project_state.state_version`:

- `project_state.state_version` is the only public baseline state clock.
- `tasks.state_version` must not be created.
- `task_events.state_version` stores the resulting project-wide version after a committed event.
- `tool_invocations.basis_state_version` stores the project-wide version observed before the original committed mutation.

Current Change Unit:

- `idx_change_units_one_current_active` permits at most one current Change Unit row with `status='active'` per `Task`.
- When `tasks.current_change_unit_id` is set, typed Core code must ensure it points to the same row that is `status='active'` and `is_current=1`.

Task revisions and close basis:

- `tasks.scope_revision` and `tasks.close_basis_revision` are internal current-state coordinates, not public state clocks and not caller-selected authority.
- `runs.scope_revision` stores the current-scope revision observed by the run and is required for every run row.
- Material current-scope or current Change Unit changes increment `tasks.scope_revision`; semantically identical normalized updates do not.
- A committed `harness.record_run` increments `tasks.close_basis_revision` exactly once.
- A material scope change invalidates `tasks.close_basis_json`, increments `tasks.close_basis_revision`, and may make judgment basis rows stale or superseded under their owners.
- Recording a user judgment does not increment either task revision.
- `tasks.close_basis_json` is nullable current `CurrentCloseBasis` storage. SQL `NULL` means no current close basis is available.
- `tasks.close_summary_json` is preserved for successful terminal close results. Existing open Tasks do not automatically convert terminal close summary JSON into a current close basis.

Judgment basis storage:

- `user_judgments.basis_json` stores the required API `JudgmentBasis` snapshot.
- `user_judgments.basis_status` stores the storage-owned compatibility state for the judgment basis: `current`, `stale`, or `superseded`.
- The closed `user_judgments.status` set, required `basis_json`, structured `options_json`, resolution completeness checks, actor provenance columns, resolved-surface provenance columns, and composite resolved-surface foreign key are part of the `baseline_sqlite_v2` project-state schema version `1`.
- `status='resolved'` rows require non-null `resolution_outcome`, `resolution_machine_action`, `resolution_json`, resolved actor provenance, resolved surface provenance, and `resolved_at`.
- `status='pending'` and `status='expired'` rows require all resolution and resolved-provenance columns to be null.
- `status='stale'` and `status='superseded'` rows may carry either a complete resolution group or no resolution group.
- `user_judgments.resolution_outcome` stores the selected option's machine-readable outcome. `user_judgments.resolution_machine_action` stores the selected Core-created authority action. The SQL action/outcome check keeps `accept` with `accepted`, `reject` with `rejected`, and `defer` with `deferred`; `blocked` is not a persisted option action outcome.
- `resolved_by_actor_kind`, `resolved_actor_role`, `resolved_by_surface_id`, `resolved_by_surface_instance_id`, `resolved_verification_basis`, and `resolved_assurance_level` store derived `VerifiedActorContext` provenance for resolution. Authority-bearing rows still require `resolved_by_actor_kind='user'`, `resolved_actor_role='user_interaction'`, a valid resolved surface/instance reference, and non-null provenance fields.

Surface local access grants:

- `surfaces.local_access_json` is the baseline storage location for registered local access grants.
- `surfaces.interaction_role` records whether the registered surface instance supplies `agent` or `user_interaction` actor provenance. Baseline storage does not support mixed-role surface instances.
- `authorized_access_classes: string[]` is required, must contain at least one documented access class, and may contain multiple classes for one surface instance.
- `verification_basis: string` is required and must be non-empty. It is controlled registration or adapter-binding diagnostic metadata for explaining how the grant was established. It is not caller authority and does not add a grant.
- `access_class` is not a valid key in `surfaces.local_access_json`. Stored `access_class` fields in capability profiles, verified replay context, or invocation context remain separate meanings owned by their respective owners.
- `surfaces.capability_profile_json` is a capability declaration and must not be treated as an access-class grant.

Idempotency replay rows:

- The replay uniqueness key is exactly `(project_id, tool_name, idempotency_key)`.
- `request_hash` is stored as the public-request conflict discriminator, but it is not part of a unique key and does not absorb invocation context.
- `tool_invocations.response_json` stores only committed replay responses that [Storage Effects](storage-effects.md) says create replay rows.
- Replay rows store complete non-null `surface_id`, `surface_instance_id`, and `access_class` values from the derived `VerifiedSurfaceContext`.
- Verified replay rows require a valid referenced surface through the physical composite foreign key `(project_id, surface_id, surface_instance_id)` referencing `surfaces(project_id, surface_id, surface_instance_id)`.
- The `tool_invocations` table definition rejects replay rows that lack `surface_id`, `surface_instance_id`, or `access_class`.
- The replay surface foreign key uses restrictive deletion behavior. Schema validation must inspect the actual SQLite foreign-key definition, not only the presence of the columns.
- `verification_basis` may be stored on replay rows for diagnostics, but it is not caller authority.
- [Storage Versioning](storage-versioning.md) owns replay eligibility for stored rows with complete invocation context.

`Write Authorization` basis versions:

- `write_authorizations.basis_state_version` stores the resulting `project_state.state_version` after the authorization-creation commit.
- `basis_state_version` is not a separate state clock and must not be compared to `tasks` rows.
- `idx_runs_write_authorization` and `idx_write_authorizations_consumed_run` prevent a single authorization consumption from forking across multiple runs.

Staged artifact provenance:

- `artifact_staging.created_by_surface_id` and `artifact_staging.created_by_surface_instance_id` are required and foreign-keyed to `surfaces`.
- Staged-handle consumption must validate stored surface provenance, same project, same `Task`, expiration, lifecycle status, `sha256`, `size_bytes`, and `redaction_state` before commit.
- The `artifact_staging` and `artifacts` unique indexes prevent one staged handle from promoting to multiple artifact rows.

Persistent artifact integrity:

- `artifacts.integrity_status='verified'` requires a non-empty `content_type`, a lowercase hexadecimal 64-character `sha256`, and nonnegative `size_bytes`.
- `integrity_status='corrupt'` records a known integrity failure or invalid verified-fact relationship. `corrupt` artifacts cannot satisfy evidence or close authority requirements.
- The DDL check validates metadata shape only. Artifact Storage owns current-byte validation before authority use. Missing, unreadable, unavailable, or unusable backing bytes remain availability conditions and do not add another persisted integrity value.

Persistent artifact body paths:

- `artifacts.body_path`, when present, is a non-empty artifact-store-relative path.
- The DDL check rejects slash-absolute paths, drive-letter prefixes, backslash-separated strings, simple parent-traversal components, and the project-home-relative `artifacts` component or `artifacts/` prefix.
- Artifact Storage owns full current-byte, artifact-store boundary, and symlink validation before authority use.

Migration records:

- Each database has its own `schema_migrations` table.
- `schema_migrations` records applied schema versions, names, storage profiles, application time, optional checksum, and storage-owned metadata.
- Migration semantics, repair behavior, and supported migration paths remain owned by [Storage Versioning](storage-versioning.md).

Foreign key limits:

- Direct same-project and same-`Task` relationships use composite foreign keys.
- `artifact_links.owner_record_kind` is a closed storage-owned value set, but `owner_record_id` is polymorphic. Typed Core code must validate that the owner row exists in the table named by `owner_record_kind` and belongs to the same `project_id` and `task_id`.
- JSON reference arrays cannot be represented as SQLite foreign keys. Typed Core code must validate those references before commit.

## Related owners

- [Storage Records](storage-records.md) defines record families, placement, storage-owned values, and JSON placement categories.
- [Storage Effects](storage-effects.md) defines which method branches create, update, observe, or leave storage untouched.
- [Artifact Storage](storage-artifacts.md) defines staged-handle lifecycle, promotion, linking, body reads, retention, and integrity boundaries.
- [Storage Versioning](storage-versioning.md) defines state clock meaning, idempotency and replay semantics, event meaning, locking, and migration semantics.
- [API Schema Core](api/schema-core.md) and sibling schema owners define public API shapes and API-owned value meanings.
