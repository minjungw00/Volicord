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

SQLite `TEXT` columns ending in `_json` store JSON as a representation choice. Typed Core code must parse and validate those columns before commit against the applicable API schema owner, storage owner, or artifact owner. SQLite defaults such as `'{}'` and `'[]'` do not make API fields optional.

`project_state.state_version` is the only public baseline state clock. Baseline SQLite DDL must not create `tasks.state_version`.

## `registry.sqlite`

`registry.sqlite` stores Runtime Home identity and project registration only. It does not store project-local Core state.

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
```

Registry constraints:

- `runtime_home` is a singleton table. The stored `runtime_home_id` identifies the Runtime Home record; it is not a security guarantee.
- `projects.project_home` is unique. `repo_root` is indexed for lookup but does not replace project identity.
- `projects.status` is storage-owned and baseline-valid only as `active`.
- `schema_migrations` records applied registry schema versions. Migration execution semantics stay with [Storage Versioning](storage-versioning.md).

## Project `state.sqlite`

Each registered project has one project-local `state.sqlite`. It stores Core state for that project and repeats `project_id` in project-scoped rows so foreign keys and indexes can enforce same-project relationships.

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
  surface_id TEXT,
  surface_instance_id TEXT,
  access_class TEXT,
  verification_basis TEXT,
  replay_context_status TEXT NOT NULL CHECK (replay_context_status IN ('verified', 'legacy_unverified')),
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
    REFERENCES surfaces (project_id, surface_id, surface_instance_id),
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

Surface local access grants:

- `surfaces.local_access_json` is the baseline storage location for registered local access grants.
- The preferred grant field is `authorized_access_classes: string[]`; `access_class: string` is a backward-compatible single-value fallback.
- `verification_basis: string` is trusted registration metadata for explaining how the grant was established.
- `surfaces.capability_profile_json` is a capability declaration and must not be treated as an access-class grant.

Idempotency replay rows:

- The replay uniqueness key is exactly `(project_id, tool_name, idempotency_key)`.
- `request_hash` is stored as the public-request conflict discriminator, but it is not part of a unique key and does not absorb invocation context.
- `tool_invocations.response_json` stores only committed replay responses that [Storage Effects](storage-effects.md) says create replay rows.
- Newly written replay rows use `replay_context_status='verified'` and store complete `surface_id`, `surface_instance_id`, and `access_class` values from the derived `VerifiedSurfaceContext`.
- `verification_basis` may be stored on replay rows for diagnostics, but it is not caller authority.
- Existing replay rows that lack verified context may be represented with `replay_context_status='legacy_unverified'` and null or incomplete context fields; [Storage Versioning](storage-versioning.md) owns replay eligibility.

`Write Authorization` basis versions:

- `write_authorizations.basis_state_version` stores the resulting `project_state.state_version` after the authorization-creation commit.
- `basis_state_version` is not a separate state clock and must not be compared to `tasks` rows.
- `idx_runs_write_authorization` and `idx_write_authorizations_consumed_run` prevent a single authorization consumption from forking across multiple runs.

Staged artifact provenance:

- `artifact_staging.created_by_surface_id` and `artifact_staging.created_by_surface_instance_id` are required and foreign-keyed to `surfaces`.
- Staged-handle consumption must validate stored surface provenance, same project, same `Task`, expiration, lifecycle status, `sha256`, `size_bytes`, and `redaction_state` before commit.
- The `artifact_staging` and `artifacts` unique indexes prevent one staged handle from promoting to multiple artifact rows.

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
