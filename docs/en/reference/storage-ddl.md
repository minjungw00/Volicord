# Storage DDL

This document owns the physical SQLite DDL contract for the one canonical
storage layout described by [Storage Records](storage-records.md). It makes
`registry.sqlite`, project `state.sqlite`, and the physical `StorageManifest`
placement implementable without moving manifest identity, database-open
classification, method effects, artifact lifecycle rules, state-version
meaning, API schemas, or security guarantees into this document.

## Owner Boundaries

This document owns:

- canonical SQLite table shape for `registry.sqlite` and project `state.sqlite`
- canonical indexes, foreign keys, views, and physical constraints
- the physical `StorageManifest` carrier columns and their strict persisted representation
- SQLite constraints for `project_state.state_version`, replay rows, current Change Unit uniqueness, write-ticket basis versions, staged artifact provenance, and host-observation records
- the DDL-level split between Runtime Home registration data and project-local Core state
- the canonical SQL inputs from which `GeneratedSchemaMetadata` and the documentation projection are derived

This document does not own:

- record-family purpose, storage locations, storage-owned values, or JSON placement categories; see [Storage Records](storage-records.md)
- method branch storage effects; see [Storage Effects](storage-effects.md)
- artifact staging, promotion, linking, body reads, retention, or integrity lifecycle; see [Artifact Storage](storage-artifacts.md)
- `StorageManifest` semantic identity, digest construction, capability meaning,
  exact-open comparison, failure classification, state-version, idempotency,
  event, or lock behavior; see [Storage Versioning](storage-versioning.md)
- API request or response schemas; see the API schema owners routed from [API Schema Core](api/schema-core.md)
- runtime location boundaries; see [Runtime Boundaries](runtime-boundaries.md)
- security guarantee levels; see [Security](security.md)

<a id="surface-stability"></a>
## Surface Stability

For canonical vocabulary, see [Documentation Policy](../maintain/documentation-policy.md#surface-stability-labels). In this section, `stable` means a documented compatibility surface; `beta` means supported, but details may change; `internal` means an implementation or generated-integration detail, not a normal user input surface; and `diagnostic` means a troubleshooting or status-reporting surface whose prose or diagnostic wording is not a stable API contract.

| Surface | Stability | Notes |
|---|---|---|
| Canonical SQLite DDL, manifest carrier columns, canonical SQL blocks, table constraints, indexes, views, foreign keys, `project_state.state_version` as the public state clock, and `project_state.updated_at` as the physical canonical-UTC floor | `stable` | This is the implementable storage DDL contract for the one accepted manifest. The UTC floor is not a second public state-version field. |
| Physical table names, column names, internal IDs, generated host-observation rows, and `_json` representation columns | `internal` | These make the storage layout implementable; they are not ordinary user-facing selectors or public API arguments unless another focused owner exposes them. |
| Safe storage or corruption diagnostics that identify table, record reference, logical column, or corruption category | `diagnostic` | Diagnostics must not expose raw stored JSON, secrets, SQL text, or sensitive absolute paths. |

## Connection And Transaction Requirements

SQLite foreign keys are part of this DDL contract. Every connection that reads or writes these databases must enable:

```sql
PRAGMA foreign_keys = ON;
```

Mutating transactions must use `BEGIN IMMEDIATE` or an equivalent serialized
write boundary before reading freshness, write-ticket compatibility rows,
staging, replay rows, or the persisted canonical-UTC floor for a commit.

Authority rows remain addressable unless an owning storage contract defines a repair or retention path. The registry may cascade-delete non-authority alias rows that are owned by a forgotten project registration; it must not use alias cleanup to imply deletion of project-local Core authority records.

SQLite `TEXT` columns ending in `_json` store JSON as a representation choice. JSON used for authority, lifecycle, scope, evidence, completion, close readiness, or write compatibility is typed owner state. Typed Core code must parse and validate those columns before commit against the applicable API schema owner, storage owner, or artifact owner. Failure to decode typed owner state is corruption and must never be converted to an empty object, empty array, false value, default enum, or "no requirement" interpretation. SQL `NULL` may mean absence only when the owning schema explicitly marks the field optional; malformed JSON in an optional column is corruption, not absence. Open-ended display metadata may remain untyped only when it is not used for authority or close decisions. Safe diagnostics may identify the table, record reference, logical column, and corruption category, but must not expose raw stored JSON, secrets, SQL text, or sensitive absolute paths. SQLite defaults such as `'{}'` and `'[]'` do not make API fields optional.

`project_state.state_version` is the only public state clock.
`project_state.updated_at` is a distinct physical floor for the canonical Core
UTC clock, not a public conflict version or schema version. Canonical SQLite DDL
must not create `tasks.state_version`, storage `schema_version` columns, or a
migration ledger table.

The physical `write_tickets` table stores authority records for product-file
write attempts and exact approval-bound non-product actions under effective
`sensitive` control. These rows record Volicord-authorized bounded intent and
compatibility state; they are not OS permissions, filesystem ACLs, sandboxing,
network policy, secret isolation, global filesystem interception, or proof that
an effect occurred.

<a id="physical-storage-manifest-placement"></a>
## Physical `StorageManifest` Placement

The canonical SQL has no separate manifest table and no numeric schema-version
column. The complete manifest occupies the two existing carrier columns below:

| Database | Owning row | Carrier column | Exact DDL shape |
|---|---|---|---|
| `registry.sqlite` | the `runtime_home` row selected by `singleton_id=1` | `runtime_home.storage_profile` | `TEXT NOT NULL` with no SQL default |
| project `state.sqlite` | the `project_state` row for that database's `project_id` | `project_state.storage_profile` | `TEXT NOT NULL` with no SQL default |

Despite its physical name, `storage_profile` is not a profile selector,
numeric revision, migration key, or compatibility alias. It stores the one
deterministic canonical UTF-8 JSON encoding of the complete current
`StorageManifest`. The object
has exactly `contract_id`, `canonical_ddl_digest`,
`integrity_constraints_digest`, and `enabled_capabilities`; missing, unknown,
or duplicate members are invalid. The capability array must preserve the
complete sorted, duplicate-free set owned by
[Storage Versioning](storage-versioning.md).

Fresh initialization writes the same current manifest value into the registry
carrier and every newly created project carrier. Store strict-decodes each
carrier independently before reading authority or policy records. It requires
the persisted value to equal the current built-in manifest and requires a
selected project's manifest to equal the registry manifest. It does not parse
an integer, compare versions, inspect field presence to select a decoder, or
try another profile. The exact open result and failure category remain with
[Storage Versioning](storage-versioning.md).

Adding the immutable Agent Connection `integration_instance_id` changes both
schema digests. A Runtime Home carrying the immediately prior schema manifest
is rejected as an unsupported storage profile with explicit reinitialization
guidance. Store does not add the column or synthesize values in place.

## Canonical SQL Sources

The only executable DDL sources are
[`registry.sql`](../../../crates/volicord-store/src/schema/registry.sql) and
[`project.sql`](../../../crates/volicord-store/src/schema/project.sql), in that
fixed source order. Fresh initialization applies them only to empty SQLite
databases. There is no migration, conversion, upgrade, importer, historical SQL
bundle, numeric profile dispatch, or alternate database opener.

These two source files are exact-byte textual contracts. Their canonical
repository form uses LF bytes only and ends with exactly one LF. The root
`.gitattributes` rule forces those same bytes in Linux, macOS, native Windows,
and WSL2 checkouts regardless of client line-ending configuration. Because
`include_str!` embeds the exact source bytes used to derive
`GeneratedSchemaMetadata`, changing schema line endings changes the
`canonical_ddl_digest`, `integrity_constraints_digest`, and resulting
`StorageManifest` identity. A CRLF canonical schema source is invalid and must
be rejected; consumers do not normalize line endings, replace CRLF, or trim
arbitrary whitespace before deriving schema identity. Any canonical SQL change
must pass the strict byte, isolated checkout, generated metadata, and fixed
digest contract tests.

A deterministic extractor derives the tables, columns, indexes, constraints,
and both schema digests from those files for the single shared
`GeneratedSchemaMetadata`. Runtime validation, manifest construction, Store
query projections, fixtures, the DDL contract test, and the documentation
inventory consume that generated artifact. None keeps a second authoritative
inventory.

The canonical SQL blocks below are a checked documentation projection, not a
second DDL source. `docs-check` requires them to match the source files exactly,
and the focused `storage_ddl_contract` test validates the executable schema
semantics. Any table, column, index, view, foreign key, `CHECK`, `UNIQUE`,
default, or other physical SQLite object absent from the canonical SQL is not
part of the accepted layout.

## `registry.sqlite`

`registry.sqlite` stores Runtime Home identity, installation profile records, project registration, project aliases, Agent Connection records, Connection Projects membership, authoritative MCP runtime sessions and project reservations, host-hook installation records, and host configuration inventory. It does not store project-local Core state.

<!-- canonical-storage-sql: registry start -->
```sql
CREATE TABLE runtime_home (
  singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
  runtime_home_id TEXT NOT NULL UNIQUE,
  runtime_home_path TEXT NOT NULL UNIQUE,
  registry_db_path TEXT NOT NULL UNIQUE,
  storage_profile TEXT NOT NULL,
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
  integration_instance_id TEXT NOT NULL CHECK (
    length(integration_instance_id) = 56
    AND substr(integration_instance_id, 1, 20) = 'connection_instance_'
    AND substr(integration_instance_id, 29, 1) = '-'
    AND substr(integration_instance_id, 34, 1) = '-'
    AND substr(integration_instance_id, 39, 1) = '-'
    AND substr(integration_instance_id, 44, 1) = '-'
    AND substr(integration_instance_id, 21, 8) NOT GLOB '*[^0-9a-f]*'
    AND substr(integration_instance_id, 30, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(integration_instance_id, 35, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(integration_instance_id, 40, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(integration_instance_id, 45, 12) NOT GLOB '*[^0-9a-f]*'
    AND substr(integration_instance_id, 35, 1) = '4'
    AND substr(integration_instance_id, 40, 1) GLOB '[89ab]'
  ),
  host_kind TEXT NOT NULL CHECK (host_kind = 'codex'),
  intent TEXT NOT NULL CHECK (intent IN ('personal', 'shared')),
  host_scope TEXT NOT NULL CHECK (host_scope IN ('user', 'project')),
  project_internal_id TEXT,
  server_name TEXT NOT NULL,
  config_target TEXT NOT NULL,
  mode TEXT NOT NULL CHECK (mode IN ('read_only', 'workflow')),
  enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  managed_fingerprint TEXT NOT NULL,
  integration_generation INTEGER NOT NULL DEFAULT 0 CHECK (integration_generation >= 0),
  verification_report_json TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_internal_id) REFERENCES projects (project_internal_id) ON DELETE RESTRICT,
  CHECK (host_kind = 'codex' AND host_scope IN ('user', 'project'))
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
CREATE UNIQUE INDEX idx_agent_connections_integration_instance
  ON agent_connections (integration_instance_id);
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
CREATE UNIQUE INDEX idx_agent_connections_target_unscoped
  ON agent_connections (
    host_kind,
    intent,
    host_scope,
    config_target,
    server_name
  )
  WHERE project_internal_id IS NULL;

CREATE TRIGGER agent_connections_integration_instance_immutable
BEFORE UPDATE OF integration_instance_id ON agent_connections
BEGIN
  SELECT RAISE(ABORT, 'agent_connections.integration_instance_id is immutable');
END;

CREATE TABLE mcp_runtime_sessions (
  runtime_session_id TEXT PRIMARY KEY,
  connection_internal_id TEXT NOT NULL,
  session_source TEXT NOT NULL CHECK (session_source IN ('managed_host', 'cli_preflight')),
  connection_integration_revision TEXT NOT NULL CHECK (
    length(connection_integration_revision) = 71
    AND substr(connection_integration_revision, 1, 7) = 'sha256:'
    AND substr(connection_integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  observed_host_executable_version TEXT,
  client_name TEXT,
  client_version TEXT,
  negotiated_protocol_version TEXT,
  process_id INTEGER NOT NULL CHECK (process_id > 0),
  process_started_at TEXT NOT NULL,
  initialize_completed_at TEXT,
  initialized_notification_at TEXT,
  tools_list_observed_at TEXT,
  required_tools_present INTEGER CHECK (required_tools_present IN (0, 1)),
  last_safe_read_only_tool_call_at TEXT,
  last_observed_at TEXT NOT NULL,
  terminal_protocol_failure_code TEXT,
  terminal_protocol_failure_details TEXT,
  graceful_close_at TEXT,
  UNIQUE (runtime_session_id, connection_internal_id),
  FOREIGN KEY (connection_internal_id)
    REFERENCES agent_connections (connection_internal_id)
    ON DELETE RESTRICT,
  CHECK (
    (client_name IS NULL AND client_version IS NULL)
    OR (client_name IS NOT NULL AND client_version IS NOT NULL)
  ),
  CHECK (
    (initialize_completed_at IS NULL AND negotiated_protocol_version IS NULL AND client_name IS NULL)
    OR (initialize_completed_at IS NOT NULL AND negotiated_protocol_version IS NOT NULL AND client_name IS NOT NULL)
  ),
  CHECK (
    (tools_list_observed_at IS NULL AND required_tools_present IS NULL)
    OR (tools_list_observed_at IS NOT NULL AND required_tools_present IS NOT NULL)
  ),
  CHECK (initialized_notification_at IS NULL OR initialize_completed_at IS NOT NULL),
  CHECK (last_safe_read_only_tool_call_at IS NULL OR initialized_notification_at IS NOT NULL),
  CHECK (
    (terminal_protocol_failure_code IS NULL AND terminal_protocol_failure_details IS NULL)
    OR terminal_protocol_failure_code IS NOT NULL
  ),
  CHECK (terminal_protocol_failure_code IS NULL OR graceful_close_at IS NULL),
  CHECK (last_observed_at >= process_started_at),
  CHECK (initialize_completed_at IS NULL OR initialize_completed_at >= process_started_at),
  CHECK (initialized_notification_at IS NULL OR initialized_notification_at >= initialize_completed_at),
  CHECK (tools_list_observed_at IS NULL OR tools_list_observed_at >= initialize_completed_at),
  CHECK (last_safe_read_only_tool_call_at IS NULL OR last_safe_read_only_tool_call_at >= initialized_notification_at),
  CHECK (terminal_protocol_failure_code IS NULL OR last_observed_at >= process_started_at),
  CHECK (graceful_close_at IS NULL OR graceful_close_at >= process_started_at)
);

CREATE INDEX idx_mcp_runtime_sessions_current_revision
  ON mcp_runtime_sessions (
    connection_internal_id,
    session_source,
    connection_integration_revision,
    last_observed_at
  );
CREATE INDEX idx_mcp_runtime_sessions_successful_managed
  ON mcp_runtime_sessions (
    connection_internal_id,
    connection_integration_revision,
    last_safe_read_only_tool_call_at
  )
  WHERE session_source = 'managed_host'
    AND initialized_notification_at IS NOT NULL
    AND required_tools_present = 1
    AND last_safe_read_only_tool_call_at IS NOT NULL;

CREATE TABLE mcp_runtime_project_session_bindings (
  runtime_session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  project_internal_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  host_session_id TEXT NOT NULL,
  bound_at TEXT NOT NULL,
  PRIMARY KEY (runtime_session_id, host_session_id),
  UNIQUE (project_internal_id, session_id),
  FOREIGN KEY (runtime_session_id, connection_internal_id)
    REFERENCES mcp_runtime_sessions (runtime_session_id, connection_internal_id)
    ON DELETE RESTRICT,
  FOREIGN KEY (project_internal_id)
    REFERENCES projects (project_internal_id)
    ON DELETE RESTRICT,
  FOREIGN KEY (connection_internal_id, project_internal_id)
    REFERENCES connection_projects (connection_internal_id, project_internal_id)
    ON DELETE RESTRICT
);

CREATE INDEX idx_mcp_runtime_project_bindings_project
  ON mcp_runtime_project_session_bindings (project_internal_id, connection_internal_id, bound_at);

CREATE TABLE guard_installations (
  guard_installation_id TEXT PRIMARY KEY,
  runtime_home_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  project_internal_id TEXT NOT NULL,
  manifest_json TEXT NOT NULL CHECK (json_valid(manifest_json) AND json_type(manifest_json) = 'object'),
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
CREATE UNIQUE INDEX idx_guard_installations_scope_project
  ON guard_installations (connection_internal_id, project_internal_id);
```
<!-- canonical-storage-sql: registry end -->

Registry constraints:

- `runtime_home` is a singleton table. Its `storage_profile` column is the required manifest carrier and stores the complete current `StorageManifest`; the row also stores Runtime Home identity, the Runtime Home path, the registry database path, metadata, and timestamps. The stored `runtime_home_id` identifies the Runtime Home record; it is not a security guarantee.
- `installation_profile` stores the selected `volicord` command, MCP launch command, bin directory, default connection mode, metadata, and timestamps for the Runtime Home. It may be established by `volicord init`. It is not host trust, user authority, or public API state.
- `projects.project_internal_id` is the storage primary key for project records. `projects.project_name` is the display name. `projects.project_alias` is the CLI selection aid. `projects.repo_root` is the repository-root lookup key. `projects.project_alias`, `projects.repo_root`, `projects.project_home`, and `projects.state_db_path` are unique.
- `project_aliases` maps aliases to `project_internal_id` values. Alias rows are registry selection aids, not project-local Core authority records.
- `projects.state_db_path` is retained as a stored column. Store application-level current-registration validation must confirm it equals `project_home/state.sqlite` before operational `ProjectRecord` lookup or listing, writable project-state open, Agent Connection project routing, Core execution, project-store reuse, or MCP project availability.
- `projects.status` is storage-owned and valid only as `active`.
- `agent_connections.connection_internal_id` is the storage primary key for Agent Connection records. The table stores the unique immutable Store-generated `integration_instance_id`, host kind, connection intent in `intent`, host scope, optional `project_internal_id`, server name, config target, mode, enabled state, managed fingerprint, a Store-owned integration generation, an optional canonical verification report JSON value, metadata, and timestamps.
- `agent_connections.integration_instance_id` is a strict `connection_instance_`-prefixed UUIDv4 lifecycle coordinate created only for a new physical row. Its unique index prevents current-row collisions, and `agent_connections_integration_instance_immutable` rejects attempted updates. Compatible replay and every in-place lifecycle update preserve it. Physical row deletion removes it, and recreating the same deterministic Connection identity gets a new value.
- `agent_connections.intent` is constrained to `personal` or `shared` for the current `host_kind=codex` contract.
- The current Codex connection contract uses `host_kind=codex` and a `host_scope` of `user` or `project` according to its connection intent.
- `agent_connections.mode` is constrained to `read_only` or `workflow`.
- `agent_connections.integration_generation` is a nonnegative Store-owned input to the Connection integration revision. A successful real mode transition increments it exactly once in the same Registry transaction that updates the mode and all owned Guard manifests. A same-mode no-op does not increment it.
- The integration generation distinguishes revisions within one physical Connection instance, while `integration_instance_id` distinguishes physical deletion and recreation. Neither value identifies a host or actor, certifies a release, or acts as a security credential, and callers cannot select either value.
- `agent_connections.verification_report_json` is SQL null when no completed report exists. A non-null value stores one strict canonical `ConnectionVerificationReport`, including its derived status and actions; absent optional members are omitted rather than encoded as explicit null. Store does not persist those components independently.
- `connection_projects` is the explicit project allowlist for one Agent Connection. It stores membership with `connection_internal_id` and `project_internal_id`. Deleting a project or connection that still has membership is restricted.
- `guard_installations` stores one stable project-scoped Guard installation identity and its canonical typed Guard manifest. The manifest is bound to the row, Agent Connection, project, current integration revision, policy hash, runtime commands, complete managed-file inventory, and required hook phases. It describes Volicord ownership only; it is not a host-capability certificate, lifecycle status, OS-level enforcement proof, or write-prevention proof. File state is audited from the manifest and current files, while observation state is derived from current-owned `guard_events`.
- Explicit Connection Project removal satisfies the restrictive Registry foreign keys by owner-ordered deletion in one immediate transaction. It deletes selected project-session bindings before the selected Guard Installation and membership. When no membership remains, it deletes every remaining connection-owned binding and Guard Installation before `mcp_runtime_sessions` and then `agent_connections`. It does not cascade into `projects`, `runtime_home`, `installation_profile`, or a project `state.sqlite` database.

## Project `state.sqlite`

Each registered project has one project-local `state.sqlite`. It stores Core state for that project and repeats `project_id` in project-scoped rows so foreign keys and indexes can enforce same-project relationships.

<!-- canonical-storage-sql: project start -->
```sql
CREATE TABLE project_state (
  project_id TEXT PRIMARY KEY,
  storage_profile TEXT NOT NULL,
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
  requested_control_level TEXT NOT NULL CHECK (requested_control_level IN ('auto', 'observe', 'light', 'tracked', 'sensitive')),
  effective_control_level TEXT NOT NULL CHECK (effective_control_level IN ('observe', 'light', 'tracked', 'sensitive')),
  control_level_reason TEXT NOT NULL CHECK (length(trim(control_level_reason)) > 0),
  work_phase TEXT NOT NULL CHECK (work_phase IN ('shaping', 'implementation')),
  acceptance_policy TEXT NOT NULL CHECK (
    acceptance_policy IN ('required', 'not_required', 'policy_dependent')
  ),
  acceptance_policy_reason TEXT NOT NULL CHECK (length(trim(acceptance_policy_reason)) > 0),
  predecessor_task_id TEXT,
  lineage_relation TEXT CHECK (
    lineage_relation IS NULL OR lineage_relation IN (
      'continues', 'derived_from', 'split_from', 'replaces', 'implements_advice_from'
    )
  ),
  lineage_reason TEXT,
  carry_forward_json TEXT NOT NULL DEFAULT '[]',
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
  close_summary_json TEXT NOT NULL DEFAULT '{"close_reason":"none"}',
  current_change_unit_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  closed_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, task_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, predecessor_task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, current_change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
    DEFERRABLE INITIALLY DEFERRED,
  CHECK (
    (predecessor_task_id IS NULL AND lineage_relation IS NULL AND lineage_reason IS NULL)
    OR (
      predecessor_task_id IS NOT NULL
      AND lineage_relation IS NOT NULL
      AND lineage_reason IS NOT NULL
      AND length(trim(lineage_reason)) > 0
      AND predecessor_task_id <> task_id
    )
  )
);

CREATE TABLE acceptance_criteria (
  project_id TEXT NOT NULL,
  acceptance_criterion_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  statement TEXT NOT NULL CHECK (length(trim(statement)) > 0),
  evidence_requirement TEXT NOT NULL CHECK (
    evidence_requirement IN ('required', 'optional', 'not_required')
  ),
  position INTEGER NOT NULL CHECK (position >= 0),
  status TEXT NOT NULL CHECK (status IN ('active', 'retired')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  retired_at TEXT,
  PRIMARY KEY (project_id, acceptance_criterion_id),
  UNIQUE (project_id, task_id, acceptance_criterion_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  CHECK (
    (status = 'active' AND retired_at IS NULL)
    OR (status = 'retired' AND retired_at IS NOT NULL)
  )
);

CREATE TABLE evidence_claims (
  project_id TEXT NOT NULL,
  evidence_claim_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  statement TEXT NOT NULL CHECK (length(trim(statement)) > 0),
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, task_id, evidence_claim_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id)
);

CREATE TABLE change_units (
  project_id TEXT NOT NULL,
  change_unit_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('proposed', 'active', 'replaced', 'closed')),
  is_current INTEGER NOT NULL DEFAULT 0 CHECK (is_current IN (0, 1)),
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version >= 0),
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

CREATE TABLE evidence_capture_intents (
  project_id TEXT NOT NULL,
  evidence_capture_intent_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT NOT NULL,
  scope_revision INTEGER NOT NULL CHECK (scope_revision >= 0),
  baseline_ref TEXT NOT NULL CHECK (length(trim(baseline_ref)) > 0),
  target_json TEXT NOT NULL,
  capture_kind TEXT NOT NULL CHECK (
    capture_kind IN (
      'verified_command_execution',
      'verified_tool_invocation'
    )
  ),
  capture_spec_json TEXT NOT NULL,
  input_sha256 TEXT NOT NULL CHECK (
    length(input_sha256) = 64 AND input_sha256 NOT GLOB '*[^0-9a-f]*'
  ),
  expected_outcome_json TEXT NOT NULL,
  requested_by_actor_source TEXT NOT NULL CHECK (
    length(trim(requested_by_actor_source)) > 0
  ),
  requesting_connection_internal_id TEXT NOT NULL CHECK (
    length(trim(requesting_connection_internal_id)) > 0
  ),
  session_context_json TEXT NOT NULL DEFAULT '{}',
  workspace_context_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, evidence_capture_intent_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
);

CREATE TABLE user_action_requests (
  project_id TEXT NOT NULL,
  user_action_request_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  action_kind TEXT NOT NULL CHECK (
    action_kind IN (
      'product_decision',
      'technical_decision',
      'scope_decision',
      'sensitive_approval',
      'final_acceptance',
      'residual_risk_acceptance',
      'cancellation',
      'evidence_observation'
    )
  ),
  request_json TEXT NOT NULL,
  basis_json TEXT NOT NULL,
  basis_status TEXT NOT NULL DEFAULT 'current'
    CHECK (basis_status IN ('current', 'stale', 'superseded')),
  required_for_json TEXT NOT NULL,
  requested_by_actor_source TEXT NOT NULL,
  source_method TEXT NOT NULL CHECK (
    source_method IN ('volicord.request_user_action', 'volicord.reconcile_changes')
  ),
  source_idempotency_key TEXT NOT NULL CHECK (length(trim(source_idempotency_key)) > 0),
  requested_at TEXT NOT NULL,
  expires_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, user_action_request_id),
  UNIQUE (project_id, user_action_request_id, action_kind),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
);

CREATE TABLE user_action_resolutions (
  project_id TEXT NOT NULL,
  user_action_resolution_id TEXT NOT NULL,
  user_action_request_id TEXT NOT NULL,
  action_kind TEXT NOT NULL CHECK (
    action_kind IN (
      'product_decision',
      'technical_decision',
      'scope_decision',
      'sensitive_approval',
      'final_acceptance',
      'residual_risk_acceptance',
      'cancellation',
      'evidence_observation'
    )
  ),
  channel_kind TEXT NOT NULL CHECK (channel_kind = 'cli'),
  channel_submission_id TEXT NOT NULL CHECK (
    length(CAST(channel_submission_id AS BLOB)) BETWEEN 1 AND 256
    AND length(channel_submission_id) = length(CAST(channel_submission_id AS BLOB))
    AND channel_submission_id NOT GLOB '*[^!-~]*'
  ),
  resolution_json TEXT NOT NULL,
  resolved_by_actor_source TEXT NOT NULL CHECK (resolved_by_actor_source = 'local_user'),
  resolved_verification_basis TEXT NOT NULL CHECK (length(trim(resolved_verification_basis)) > 0),
  resolved_assurance_level TEXT NOT NULL CHECK (length(trim(resolved_assurance_level)) > 0),
  resolved_at TEXT NOT NULL,
  PRIMARY KEY (project_id, user_action_resolution_id),
  UNIQUE (project_id, user_action_request_id),
  UNIQUE (project_id, channel_kind, channel_submission_id),
  FOREIGN KEY (project_id, user_action_request_id, action_kind)
    REFERENCES user_action_requests (
      project_id,
      user_action_request_id,
      action_kind
    )
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

CREATE TABLE write_tickets (
  project_id TEXT NOT NULL,
  write_ticket_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT NOT NULL,
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

CREATE UNIQUE INDEX idx_write_tickets_consumed_run
  ON write_tickets (project_id, consumed_by_run_id)
  WHERE consumed_by_run_id IS NOT NULL;

CREATE TABLE runs (
  project_id TEXT NOT NULL,
  run_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  write_ticket_id TEXT,
  kind TEXT NOT NULL,
  status TEXT NOT NULL,
  summary_json TEXT NOT NULL DEFAULT '{}',
  observed_changes_json TEXT NOT NULL DEFAULT '{}',
  evidence_updates_json TEXT NOT NULL DEFAULT '[]',
  write_ticket_effect_json TEXT NOT NULL DEFAULT '{}',
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
  FOREIGN KEY (project_id, write_ticket_id)
    REFERENCES write_tickets (project_id, write_ticket_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE UNIQUE INDEX idx_runs_write_ticket
  ON runs (project_id, write_ticket_id)
  WHERE write_ticket_id IS NOT NULL;

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

CREATE TABLE evidence_capture_receipts (
  project_id TEXT NOT NULL,
  evidence_capture_receipt_id TEXT NOT NULL,
  evidence_capture_intent_id TEXT NOT NULL,
  staging_handle_id TEXT NOT NULL,
  capture_kind TEXT NOT NULL CHECK (
    capture_kind IN (
      'verified_command_execution',
      'verified_tool_invocation'
    )
  ),
  input_sha256 TEXT NOT NULL CHECK (
    length(input_sha256) = 64 AND input_sha256 NOT GLOB '*[^0-9a-f]*'
  ),
  result_sha256 TEXT NOT NULL CHECK (
    length(result_sha256) = 64 AND result_sha256 NOT GLOB '*[^0-9a-f]*'
  ),
  expected_outcome_json TEXT NOT NULL,
  observed_outcome_json TEXT NOT NULL,
  source_refs_json TEXT NOT NULL DEFAULT '[]',
  observed_by_actor_source TEXT NOT NULL CHECK (
    length(trim(observed_by_actor_source)) > 0
  ),
  observed_at TEXT NOT NULL,
  completeness TEXT NOT NULL CHECK (completeness = 'complete'),
  limitations_json TEXT NOT NULL DEFAULT '[]',
  safe_receipt_json TEXT NOT NULL,
  safe_receipt_sha256 TEXT NOT NULL CHECK (
    length(safe_receipt_sha256) = 64 AND safe_receipt_sha256 NOT GLOB '*[^0-9a-f]*'
  ),
  safe_receipt_size_bytes INTEGER NOT NULL CHECK (safe_receipt_size_bytes >= 0),
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, evidence_capture_receipt_id),
  UNIQUE (project_id, evidence_capture_intent_id),
  UNIQUE (
    project_id,
    evidence_capture_intent_id,
    evidence_capture_receipt_id
  ),
  UNIQUE (project_id, staging_handle_id),
  FOREIGN KEY (project_id, evidence_capture_intent_id)
    REFERENCES evidence_capture_intents (project_id, evidence_capture_intent_id),
  FOREIGN KEY (project_id, staging_handle_id)
    REFERENCES artifact_staging (project_id, handle_id)
);

CREATE TABLE evidence_capture_source_claims (
  project_id TEXT NOT NULL,
  source_claim_kind TEXT NOT NULL CHECK (
    source_claim_kind = 'host_invocation'
  ),
  source_claim_id TEXT NOT NULL CHECK (length(trim(source_claim_id)) > 0),
  evidence_capture_intent_id TEXT NOT NULL,
  evidence_capture_receipt_id TEXT NOT NULL,
  capture_kind TEXT NOT NULL CHECK (
    capture_kind IN (
      'verified_command_execution',
      'verified_tool_invocation'
    )
  ),
  claimed_at TEXT NOT NULL,
  CHECK (
    source_claim_kind != 'host_invocation'
    OR (
      length(source_claim_id) = 64
      AND source_claim_id NOT GLOB '*[^0-9a-f]*'
    )
  ),
  PRIMARY KEY (project_id, source_claim_kind, source_claim_id),
  FOREIGN KEY (
    project_id,
    evidence_capture_intent_id,
    evidence_capture_receipt_id
  ) REFERENCES evidence_capture_receipts (
    project_id,
    evidence_capture_intent_id,
    evidence_capture_receipt_id
  )
);

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
    owner_record_kind IN ('task', 'change_unit', 'run', 'user_action_request', 'user_action_resolution', 'evidence_summary', 'evidence_observation', 'evidence_producer', 'blocker')
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
  produced_at_state_version INTEGER NOT NULL CHECK (produced_at_state_version >= 0),
  status TEXT NOT NULL,
  coverage_json TEXT NOT NULL DEFAULT '[]',
  supporting_refs_json TEXT NOT NULL DEFAULT '[]',
  gap_refs_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, evidence_summary_id),
  UNIQUE (project_id, task_id, produced_at_state_version),
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
  acceptance_criterion_id TEXT,
  evidence_claim_id TEXT,
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
  source_refs_json TEXT NOT NULL DEFAULT '[]',
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
    DEFERRABLE INITIALLY DEFERRED,
  FOREIGN KEY (project_id, task_id, acceptance_criterion_id)
    REFERENCES acceptance_criteria (project_id, task_id, acceptance_criterion_id),
  FOREIGN KEY (project_id, task_id, evidence_claim_id)
    REFERENCES evidence_claims (project_id, task_id, evidence_claim_id),
  CHECK (
    (acceptance_criterion_id IS NOT NULL AND evidence_claim_id IS NULL)
    OR (acceptance_criterion_id IS NULL AND evidence_claim_id IS NOT NULL)
  )
);

CREATE TABLE evidence_producers (
  project_id TEXT NOT NULL,
  evidence_producer_id TEXT NOT NULL,
  evidence_capture_intent_id TEXT NOT NULL,
  evidence_capture_receipt_id TEXT NOT NULL,
  evidence_observation_id TEXT NOT NULL,
  artifact_id TEXT NOT NULL,
  run_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT NOT NULL,
  scope_revision INTEGER NOT NULL CHECK (scope_revision >= 0),
  baseline_ref TEXT NOT NULL CHECK (length(trim(baseline_ref)) > 0),
  producer_kind TEXT NOT NULL CHECK (
    producer_kind IN (
      'verified_command_execution',
      'verified_tool_invocation'
    )
  ),
  canonical_producer_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, evidence_producer_id),
  UNIQUE (project_id, evidence_capture_intent_id),
  UNIQUE (project_id, evidence_capture_receipt_id),
  UNIQUE (project_id, evidence_observation_id),
  UNIQUE (project_id, artifact_id),
  FOREIGN KEY (project_id, evidence_capture_intent_id)
    REFERENCES evidence_capture_intents (project_id, evidence_capture_intent_id),
  FOREIGN KEY (project_id, evidence_capture_receipt_id)
    REFERENCES evidence_capture_receipts (project_id, evidence_capture_receipt_id),
  FOREIGN KEY (
    project_id,
    evidence_capture_intent_id,
    evidence_capture_receipt_id
  ) REFERENCES evidence_capture_receipts (
    project_id,
    evidence_capture_intent_id,
    evidence_capture_receipt_id
  ),
  FOREIGN KEY (project_id, evidence_observation_id)
    REFERENCES evidence_observations (project_id, evidence_observation_id),
  FOREIGN KEY (project_id, artifact_id) REFERENCES artifacts (project_id, artifact_id),
  FOREIGN KEY (project_id, run_id) REFERENCES runs (project_id, run_id),
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

CREATE TABLE authority_events (
  project_id TEXT NOT NULL,
  event_seq INTEGER NOT NULL CHECK (event_seq > 0),
  event_id TEXT NOT NULL,
  state_version INTEGER NOT NULL CHECK (state_version > 0),
  event_type TEXT NOT NULL,
  actor_source TEXT NOT NULL,
  operation_category TEXT NOT NULL CHECK (operation_category IN ('read', 'agent_workflow', 'user_only', 'admin_local', 'local_recovery')),
  task_id TEXT,
  change_unit_id TEXT,
  payload_json TEXT NOT NULL DEFAULT '{}',
  request_hash TEXT NOT NULL,
  previous_event_hash TEXT,
  event_hash TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, event_seq),
  UNIQUE (project_id, event_id),
  UNIQUE (project_id, event_hash),
  CHECK (length(trim(event_hash)) > 0),
  CHECK (previous_event_hash IS NULL OR length(trim(previous_event_hash)) > 0),
  CHECK (
    (event_type = 'project_workflow_policy_applied'
      AND task_id IS NULL AND change_unit_id IS NULL)
    OR (event_type <> 'project_workflow_policy_applied' AND task_id IS NOT NULL)
  ),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, previous_event_hash)
    REFERENCES authority_events (project_id, event_hash)
    DEFERRABLE INITIALLY DEFERRED
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
  git_workspace_context_json TEXT,
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

CREATE INDEX idx_acceptance_criteria_task_status
  ON acceptance_criteria (project_id, task_id, status, position);

CREATE INDEX idx_evidence_claims_task
  ON evidence_claims (project_id, task_id);

CREATE INDEX idx_change_units_task_status
  ON change_units (project_id, task_id, status);

CREATE INDEX idx_evidence_capture_intents_task_expiry
  ON evidence_capture_intents (project_id, task_id, expires_at);

CREATE INDEX idx_evidence_capture_intents_connection_expiry
  ON evidence_capture_intents (
    project_id,
    requesting_connection_internal_id,
    expires_at
  );

CREATE INDEX idx_user_action_requests_task_basis_expiry
  ON user_action_requests (project_id, task_id, basis_status, expires_at);
CREATE INDEX idx_user_action_requests_task_kind
  ON user_action_requests (project_id, task_id, action_kind, requested_at);
CREATE INDEX idx_user_action_resolutions_request
  ON user_action_resolutions (project_id, user_action_request_id);

CREATE UNIQUE INDEX idx_user_action_requests_direct_origin
  ON user_action_requests (project_id, source_idempotency_key)
  WHERE source_method = 'volicord.request_user_action';

CREATE INDEX idx_project_continuity_records_status
  ON project_continuity_records (project_id, status, kind, updated_at);

CREATE INDEX idx_project_continuity_records_source_task
  ON project_continuity_records (project_id, source_task_id);

CREATE INDEX idx_write_tickets_task_status
  ON write_tickets (project_id, task_id, status);

CREATE INDEX idx_runs_task_created
  ON runs (project_id, task_id, created_at);

CREATE INDEX idx_artifact_staging_task_status
  ON artifact_staging (project_id, task_id, status);

CREATE INDEX idx_artifact_staging_actor_source
  ON artifact_staging (project_id, created_by_actor_source);

CREATE INDEX idx_evidence_capture_receipts_created
  ON evidence_capture_receipts (project_id, created_at);

CREATE INDEX idx_evidence_capture_source_claims_receipt
  ON evidence_capture_source_claims (
    project_id,
    evidence_capture_receipt_id,
    source_claim_kind,
    source_claim_id
  );

CREATE INDEX idx_artifacts_task_status
  ON artifacts (project_id, task_id, status);

CREATE INDEX idx_artifact_links_owner
  ON artifact_links (project_id, owner_record_kind, owner_record_id);

CREATE INDEX idx_evidence_summaries_task_status
  ON evidence_summaries (project_id, task_id, status);

CREATE INDEX idx_evidence_observations_task_target
  ON evidence_observations (
    project_id,
    task_id,
    acceptance_criterion_id,
    evidence_claim_id
  );

CREATE INDEX idx_evidence_observations_run
  ON evidence_observations (project_id, run_id);
CREATE INDEX idx_evidence_producers_task_run
  ON evidence_producers (project_id, task_id, run_id);

CREATE INDEX idx_blockers_task_status
  ON blockers (project_id, task_id, status);

CREATE INDEX idx_authority_events_task_seq
  ON authority_events (project_id, task_id, event_seq);
CREATE INDEX idx_authority_events_state_version
  ON authority_events (project_id, state_version, event_seq);
CREATE INDEX idx_authority_events_hash_chain
  ON authority_events (project_id, previous_event_hash, event_hash);
CREATE TABLE agent_sessions (
  project_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  runtime_session_id TEXT CHECK (
    runtime_session_id IS NULL OR length(trim(runtime_session_id)) > 0
  ),
  connection_internal_id TEXT NOT NULL,
  project_integration_revision TEXT NOT NULL CHECK (
    length(project_integration_revision) = 71
    AND substr(project_integration_revision, 1, 7) = 'sha256:'
    AND substr(project_integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  host_session_id TEXT NOT NULL CHECK (length(trim(host_session_id)) > 0),
  host_thread_id TEXT NOT NULL CHECK (length(trim(host_thread_id)) > 0),
  last_host_turn_id TEXT NOT NULL CHECK (length(trim(last_host_turn_id)) > 0),
  first_observed_at TEXT NOT NULL,
  last_observed_at TEXT NOT NULL,
  PRIMARY KEY (project_id, session_id),
  UNIQUE (project_id, session_id, connection_internal_id),
  CHECK (last_observed_at >= first_observed_at),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id)
);

CREATE TABLE guard_events (
  project_id TEXT NOT NULL,
  guard_event_id TEXT NOT NULL,
  session_id TEXT,
  connection_internal_id TEXT NOT NULL,
  guard_installation_id TEXT NOT NULL,
  policy_hash TEXT NOT NULL CHECK (
    length(policy_hash) = 71
    AND substr(policy_hash, 1, 7) = 'sha256:'
    AND substr(policy_hash, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  integration_revision TEXT NOT NULL CHECK (
    length(integration_revision) = 71
    AND substr(integration_revision, 1, 7) = 'sha256:'
    AND substr(integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  event_kind TEXT NOT NULL CHECK (event_kind IN ('pre_tool', 'post_tool', 'prompt_capture')),
  contract_status TEXT NOT NULL CHECK (contract_status IN ('compatible', 'malformed', 'incompatible')),
  decision TEXT NOT NULL CHECK (decision IN ('allow', 'deny', 'warn', 'inject_context')),
  subject_json TEXT NOT NULL DEFAULT '{}',
  result_json TEXT NOT NULL DEFAULT '{}',
  occurred_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, guard_event_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, session_id, connection_internal_id)
    REFERENCES agent_sessions (project_id, session_id, connection_internal_id)
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
  FOREIGN KEY (project_id, session_id, connection_internal_id)
    REFERENCES agent_sessions (project_id, session_id, connection_internal_id)
);

CREATE TABLE unrecorded_changes (
  project_id TEXT NOT NULL,
  unrecorded_change_id TEXT NOT NULL,
  session_id TEXT,
  connection_internal_id TEXT NOT NULL,
  task_id TEXT,
  status TEXT NOT NULL CHECK (status IN ('unresolved', 'resolved')),
  confidence TEXT NOT NULL CHECK (confidence IN ('confirmed', 'suspected')),
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
  FOREIGN KEY (project_id, session_id, connection_internal_id)
    REFERENCES agent_sessions (project_id, session_id, connection_internal_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id)
);

CREATE INDEX idx_agent_sessions_connection
  ON agent_sessions (project_id, connection_internal_id);
CREATE UNIQUE INDEX idx_agent_sessions_runtime_binding
  ON agent_sessions (project_id, runtime_session_id)
  WHERE runtime_session_id IS NOT NULL;
CREATE INDEX idx_agent_sessions_runtime_revision
  ON agent_sessions (project_id, runtime_session_id, project_integration_revision, last_observed_at);
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
  change_unit_id TEXT NOT NULL,
  write_ticket_ids_json TEXT NOT NULL DEFAULT '[]',
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
  FOREIGN KEY (project_id, session_id, connection_internal_id)
    REFERENCES agent_sessions (project_id, session_id, connection_internal_id),
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
CREATE TABLE project_workflow_policies (
  project_id TEXT PRIMARY KEY,
  policy_schema TEXT NOT NULL CHECK (policy_schema = 'volicord.workflow_policy'),
  policy_version INTEGER NOT NULL CHECK (policy_version > 0),
  policy_json TEXT NOT NULL,
  policy_fingerprint TEXT NOT NULL CHECK (
    length(policy_fingerprint) = 71
    AND substr(policy_fingerprint, 1, 7) = 'sha256:'
    AND substr(policy_fingerprint, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  source TEXT NOT NULL CHECK (length(trim(source)) > 0),
  applied_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES project_state (project_id)
);

```
<!-- canonical-storage-sql: project end -->

Project-state constraints:

- `project_state.storage_profile` is the required project manifest carrier. It stores the same complete current `StorageManifest` as `runtime_home.storage_profile`; strict application validation rejects a missing, malformed, non-current, or registry-mismatched manifest rather than selecting another format.
- `project_state.state_version` is the only public state clock and must advance monotonically according to [Storage Versioning](storage-versioning.md). It is a Core state clock, not a schema version.
- `project_state.updated_at` is the non-decreasing persisted floor of the
  canonical Core UTC clock. Store application validation must strict-parse it
  as canonical UTC owner state and fail closed on malformed values. A normal
  Core commit writes one exact `committed_at` value to this column, every
  event-batch `authority_events.created_at`, and optional replay-row
  `tool_invocations.created_at`. Mutation application also uses that exact
  value for Store transaction metadata that it generates, including applicable
  `created_at`, `updated_at`, `retired_at`, and `promoted_at` values. Semantic
  operation times such as `requested_at`, `resolved_at`, `closed_at`,
  `recorded_at`, and `consumed_at`, and separately owned facts such as
  `observed_at` and `started_at`, preserve their owner-defined operation sample
  or verified source time. These cross-row and monotonic constraints are
  transaction requirements; table-local `CHECK` constraints do not express
  them.
- Store application validation strict-validates every timestamp value written
  to a timestamp column as canonical RFC 3339 UTC owner state. TTL-derived
  values require checked addition and representability; overflow or an
  unrepresentable value rejects before any row, floor, event, replay, or
  state-version effect.
- Store reads that derive the latest Agent Session or Guard event strict-parse and normalize the
  applicable RFC 3339 timestamps and compare the UTC instants at nanosecond
  precision. SQLite `julianday()`, timestamp-text order, row order, and opaque
  IDs must not determine authority order. Equal maximum instants form a
  co-latest set. If a read requires one session or Guard event and multiple
  distinct candidates are co-latest, the selection fails closed as unavailable
  owner state rather than using an ID as a tie-breaker.
- `tasks.work_phase` and `tasks.acceptance_policy` are required controlled
  values. The policy reason is non-empty. A predecessor id, lineage relation,
  and non-empty lineage reason are all-null or all-present, remain in the same
  project through the foreign key, and cannot identify the Task itself.
- `tasks.carry_forward_json` stores typed carry-forward dispositions. It does
  not make a predecessor row, judgment, evidence set, baseline, or write ticket
  current by storage presence alone.
- `authority_events` stores one durable event row per committed authority event. Multiple event rows with the same `state_version` are one event batch for one committed state transition. `task_id` is required except for the exact project-scoped `project_workflow_policy_applied` event, which also requires `change_unit_id` null. Task-scoped reads select `authority_events` rows whose `task_id` is not null.
- `authority_events.actor_source`, `tasks.created_by_actor_source`, `user_action_requests.requested_by_actor_source`, `user_action_resolutions.resolved_by_actor_source`, `evidence_capture_intents.requested_by_actor_source`, `evidence_capture_receipts.observed_by_actor_source`, `write_tickets.created_by_actor_source`, `runs.created_by_actor_source`, `artifact_staging.created_by_actor_source`, `evidence_observations.observed_by_actor_source`, and `tool_invocations.actor_source` store actor provenance.
- `authority_events.operation_category` and `tool_invocations.operation_category` are constrained to `read`, `agent_workflow`, `user_only`, `admin_local`, or `local_recovery`.
- `authority_events.request_hash` stores the request identity for the committed authority event. `previous_event_hash` and `event_hash` store a local hash chain for integrity checking and export correlation; they are not tamper-proof audit guarantees.
- `user_action_requests` stores no composite lifecycle status. Core derives effective `pending`, `resolved`, `stale`, `superseded`, or `expired` from resolution presence, `basis_status`, expiry, and current time. `user_action_resolutions` is immutable and one-to-one with a request. The closed `resolution_json` carries either the stored option-derived choice action/outcome or the full Core-derived evidence-observation body; authority meaning also requires current basis, provenance, and the method owner.
- `user_action_resolutions.channel_submission_id` is constrained to 1 through
  256 bytes of visible ASCII `0x21..=0x7e`. BLOB length supplies the byte bound,
  equal TEXT/BLOB lengths reject non-ASCII and embedded-NUL shapes, and the
  `GLOB` check rejects every byte outside the visible range. Core applies the
  same bound before replay lookup or mutation planning.
- Store application validation strict-decodes request, basis, and resolution JSON, derives the capture form from the stored request, and requires matching closed tags and derived `action_kind`. It limits target and artifact candidates to 32 each, note-like text to 1,000 Unicode scalar values, observation summary to 4,000 Unicode scalar values, and canonical serialized forms to 32 KiB, rejecting rather than truncating excess.
- `write_tickets` records reusable-until-consumed, state-bound compatibility. `basis_state_version` is audit ordering and is not unique or a validity coordinate. Validity comes from `validity_basis_json`, status, stable invalidation reason, and optional `idle_expires_at`. The unique consumption indexes still prevent one consumption from forking across Runs. Prefix arrays are strict normalized repository-relative exact-or-descendant prefixes: no glob grammar, invalid absolute/empty/`..`/ambiguous entries, denied wins, and empty allowed means no product-file writes.
- `project_workflow_policies` stores only the authoritative canonical database copy and its `sha256:<64-lowercase-hex>` fingerprint; administrative file/CLI/host behavior is outside this owner. A changed fingerprint is written with the exact transaction `committed_at`, one state-version advance, and one project-scoped policy event. When the normalized write-authority fingerprint changes, that transaction also creates or updates the active Task's reevaluation metadata mark, including for same-level authority changes, and invalidates with `explicit_revoke` every active ticket whose stored binding is missing or different plus every active ticket for the marked Task. Privacy-bounded workflow metrics belong to the separate non-authority `diagnostics.sqlite` store, never this authority database.
- `artifact_staging.created_by_actor_source` records staging provenance. Staged bytes and notices remain artifact-owned and are not evidence authority by themselves.
- `evidence_capture_intents` binds one expiring request to exact current-basis,
  verified-command or verified-tool source input, connection/actor, and
  workspace facts.
  `evidence_capture_receipts` permits exactly one complete, content-bound safe
  receipt and staging handle per intent. `evidence_capture_source_claims`
  atomically claims each normalized host invocation used by that receipt. Its
  project-scoped primary key
  prevents the same source fact from fulfilling another intent or producer
  class. Host invocation claim IDs are canonical digests over the exact
  connection and invocation coordinates, so host-local IDs from different
  exact contexts do not collide. `evidence_producers` enforces
  one-to-one intent, receipt, observation, and artifact finalization, uses a
  composite foreign key to prevent cross-pairing an intent with another
  receipt, and links each producer to one Run. Those constraints do not replace
  Core freshness, relevance, or byte-integrity validation.
- Store application validation requires every command or tool receipt and
  producer to carry the exact `connection_id` and a nullable
  `host_invocation_id`. A non-null invocation ID requires exactly one matching
  exclusive host-invocation claim; null creates no invocation claim. Any
  selected invocation must be post-intent and all receipt-fixed identifiers,
  timestamps, and digests must match it.
  Receipt staging, receipt insertion, and all claims commit or roll back
  together.
- Store application validation enforces
  `intent.created_at <= receipt.observed_at < intent.expires_at`, receipt
  creation after observation and before intent expiry, and receipt staging
  expiry exactly equal to intent expiry; these cross-row time relations are not
  expressible by the table-local checks alone.
- Store application validation advances `project_state.updated_at` to at least
  `artifact_staging.created_at` in the same transaction that creates ordinary
  staging. Evidence-capture fulfillment does the same for receipt
  `created_at` in the transaction that creates its receipt, staging row, and
  source claims. These floor-only effects do not increment `state_version` or
  create event or replay rows.
- `evidence_summaries.produced_at_state_version` stores the resulting
  authority state version of its insert or latest update. The unique
  `(project_id, task_id, produced_at_state_version)` constraint prevents two
  summaries for the same `Task` from claiming one authority-order position.
  Current-summary selection uses this column alone; timestamps and opaque IDs
  are not authority-order tie-breakers.
- `evidence_observations.source_kind` and `assurance_level` distinguish cooperative agent reports, registered connection observations, external tool results, user observations, reused evidence, and unverified claims.
- `evidence_observations.metadata_json` is strict Core-derived producer-anchor
  and relevance-assessment JSON. A user action's local-user relevance detail
  and exact current-basis coordinates stay in the closed
  `user_action_resolutions.resolution_json` evidence-observation body.
- `tool_invocations` stores replay rows with the exact verified actor source,
  operation category, optional verification basis, and optional canonical
  `git_workspace_context_json`. Replay rows are not caller authority and do not
  bypass current connection, Git workspace, or User Channel requirements.
- `agent_sessions`, `guard_events`, `prompt_captures`, `expected_writes`, and `unrecorded_changes` are project-local Codex Record Guard and reconciliation records. They repeat `connection_internal_id` for connection scoping and use project-local keys so records do not leak across projects. `agent_sessions.runtime_session_id` is null only before the first matching managed MCP tool call; `first_observed_at` and `last_observed_at` bound its Guard/MCP observation history, and the partial unique index applies only after runtime attachment.
- `guard_events` binds every observation to a required typed hook phase, Guard installation, exact policy hash, and integration revision. Only current-owned `compatible` events satisfy a required phase; current `malformed` or `incompatible` events fail the Guard observation check, and older hashes or revisions do not satisfy it. `decision` is constrained to `allow`, `deny`, `warn`, or `inject_context`; these values record local host decision requests, not OS-level enforcement proof.
- `expected_writes.status` is constrained to `pending` or `matched`, and `path_policy` is constrained to `exact_paths`. Matched rows must carry the matched Guard observation event, matched paths JSON, and `matched_at`; pending rows must not carry those matched fields.
- `unrecorded_changes.status` is constrained to `unresolved` or `resolved`. Resolved rows must carry resolution JSON, `resolved_at`, and `resolved_by_actor_source`; unresolved rows must not carry those resolution fields.

## Related Owners

- [Storage Records](storage-records.md) defines persisted record families, placement, relationship layout, storage-owned values, and JSON placement.
- [Storage Effects](storage-effects.md) defines which method branches create, update, observe, or leave records untouched.
- [Storage Versioning](storage-versioning.md) defines `StorageManifest`
  identity and digests, enabled capabilities, exact-open comparison and failure
  classification, generated schema metadata, the `project_state.state_version`
  clock, canonical Core UTC clock and persisted floor, idempotency, replay,
  events, and locks.
- [Agent Connection](agent-connection.md) defines Agent Connection, Connection Projects, current connection context, mode gating, and Agent Connection versus User Channel boundaries.
- [Security](security.md) defines security boundaries and guarantee levels.
