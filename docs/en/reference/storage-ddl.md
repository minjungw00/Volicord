# Storage DDL

This document owns the baseline SQLite DDL contract for the storage layout described by [Storage Records](storage-records.md). It makes the baseline `registry.sqlite` and project `state.sqlite` layouts implementable without moving method effects, artifact lifecycle rules, state-version meaning, API schemas, or security guarantees into this document.

## Owner Boundaries

This document owns:

- baseline SQLite table shape for `registry.sqlite` and project `state.sqlite`
- baseline indexes, foreign keys, and physical constraints
- SQLite constraints for `project_state.state_version`, replay rows, current Change Unit uniqueness, write-ticket basis versions, staged artifact provenance, and host-observation records
- the DDL-level split between Runtime Home registration data and project-local Core state
- parity between the documented canonical SQL blocks and the canonical SQL source files

This document does not own:

- record-family purpose, storage locations, storage-owned values, or JSON placement categories; see [Storage Records](storage-records.md)
- method branch storage effects; see [Storage Effects](storage-effects.md)
- artifact staging, promotion, linking, body reads, retention, or integrity lifecycle; see [Artifact Storage](storage-artifacts.md)
- state-version, idempotency, event, lock, or incompatible-storage handling; see [Storage Versioning](storage-versioning.md)
- API request or response schemas; see the API schema owners routed from [API Schema Core](api/schema-core.md)
- runtime location boundaries; see [Runtime Boundaries](runtime-boundaries.md)
- security guarantee levels; see [Security](security.md)

<a id="surface-stability"></a>
## Surface Stability

For canonical vocabulary, see [Documentation Policy](../maintain/documentation-policy.md#surface-stability-labels). In this section, `stable` means a documented compatibility surface; `beta` means supported, but details may change; `internal` means an implementation or generated-integration detail, not a normal user input surface; and `diagnostic` means a troubleshooting or status-reporting surface whose prose or diagnostic wording is not a stable API contract.

| Surface | Stability | Notes |
|---|---|---|
| Baseline SQLite DDL, canonical SQL blocks, table constraints, indexes, foreign keys, `project_state.state_version` as the public baseline state clock, and `project_state.updated_at` as the physical canonical-UTC floor | `stable` | This is the implementable storage DDL contract for the current baseline profile. The UTC floor is not a second public state-version field. |
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

Baseline authority rows remain addressable unless an owning storage contract defines a repair or retention path. The registry may cascade-delete non-authority alias rows that are owned by a forgotten project registration; it must not use alias cleanup to imply deletion of project-local Core authority records.

SQLite `TEXT` columns ending in `_json` store JSON as a representation choice. JSON used for authority, lifecycle, scope, evidence, completion, close readiness, or write compatibility is typed owner state. Typed Core code must parse and validate those columns before commit against the applicable API schema owner, storage owner, or artifact owner. Failure to decode typed owner state is corruption and must never be converted to an empty object, empty array, false value, default enum, or "no requirement" interpretation. SQL `NULL` may mean absence only when the owning schema explicitly marks the field optional; malformed JSON in an optional column is corruption, not absence. Open-ended display metadata may remain untyped only when it is not used for authority or close decisions. Safe diagnostics may identify the table, record reference, logical column, and corruption category, but must not expose raw stored JSON, secrets, SQL text, or sensitive absolute paths. SQLite defaults such as `'{}'` and `'[]'` do not make API fields optional.

`project_state.state_version` is the only public baseline state clock.
`project_state.updated_at` is a distinct physical floor for the canonical Core
UTC clock, not a public conflict version or schema version. Baseline SQLite DDL
must not create `tasks.state_version`, storage `schema_version` columns, or a
migration ledger table.

The physical `write_tickets` table stores write-ticket authority records for product-file write attempts. These rows record Volicord-authorized write intent and compatibility state; they are not OS permissions, filesystem ACLs, sandboxing, network policy, secret isolation, global filesystem interception, or proof that a write occurred.

## Canonical SQL Sources

The executable canonical SQL sources are [`registry.sql`](../../../crates/volicord-store/src/schema/registry.sql) and [`project.sql`](../../../crates/volicord-store/src/schema/project.sql). Runtime Home initialization applies these sources to empty SQLite databases. Existing Runtime Homes whose storage shape is incompatible with these sources fail clearly and require Runtime Home recreation; baseline storage does not define a legacy migration path.

`docs-check` validates that the canonical SQL blocks below match those source files exactly. The focused `storage_ddl_contract` test validates the executable schema semantics.

## `registry.sqlite`

`registry.sqlite` stores Runtime Home identity, installation profile records, project registration, project aliases, Agent Connection records, Connection Projects membership, host-hook installation records, and host configuration inventory. It does not store project-local Core state.

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

CREATE TABLE guard_installations (
  guard_installation_id TEXT PRIMARY KEY,
  runtime_home_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  project_internal_id TEXT,
  host_kind TEXT NOT NULL CHECK (length(trim(host_kind)) > 0),
  guard_mode TEXT NOT NULL CHECK (guard_mode IN ('record', 'detective')),
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
<!-- canonical-storage-sql: registry end -->

Registry constraints:

- `runtime_home` is a singleton table. It stores Runtime Home identity, the Runtime Home path, the registry database path, storage profile, metadata, and timestamps. The stored `runtime_home_id` identifies the Runtime Home record; it is not a security guarantee.
- `installation_profile` stores the selected `volicord` command, MCP launch command, bin directory, default connection mode, metadata, and timestamps for the Runtime Home. It may be established by `volicord init`. It is not host trust, user authority, or public API state.
- `projects.project_internal_id` is the storage primary key for project records. `projects.project_name` is the display name. `projects.project_alias` is the CLI selection aid. `projects.repo_root` is the repository-root lookup key. `projects.project_alias`, `projects.repo_root`, `projects.project_home`, and `projects.state_db_path` are unique.
- `project_aliases` maps aliases to `project_internal_id` values. Alias rows are registry selection aids, not project-local Core authority records.
- `projects.state_db_path` is retained as a stored column. Store application-level current-registration validation must confirm it equals `project_home/state.sqlite` before operational `ProjectRecord` lookup or listing, writable project-state open, Agent Connection project routing, Core execution, profile reuse, or MCP project availability.
- `projects.status` is storage-owned and baseline-valid only as `active`.
- `agent_connections.connection_internal_id` is the storage primary key for Agent Connection records. The table stores host kind, connection intent in `intent`, host scope, optional `project_internal_id`, server name, config target, mode, enabled state, managed fingerprint, verification summary status, verification report JSON, user actions JSON, metadata, and timestamps.
- `agent_connections.intent` is constrained to `personal`, `shared`, or `global`.
- `agent_connections.host_scope` is constrained with `host_kind`: Codex supports `user` and `project`; Claude Code supports `local`, `project`, and `user`; generic records are limited to `export`.
- `agent_connections.mode` is constrained to `read_only` or `workflow`.
- `agent_connections.last_verification_report_json` stores the latest verification report JSON object. `agent_connections.last_user_actions_json` stores the latest user-action JSON array.
- `connection_projects` is the explicit project allowlist for one Agent Connection. It stores membership with `connection_internal_id` and `project_internal_id`. Deleting a project or connection that still has membership is restricted.
- `guard_installations` stores local host-hook setup lifecycle state and host capability for one Runtime Home, Agent Connection, and optional project scope. Its internal `guard_mode` values are `record` and `detective`. Its `installation_status` values are `absent`, `configured`, `reload_required`, `active`, `degraded`, `stale`, and `broken`. These rows are local authority records for host observation; they are not OS-level enforcement proof or write-prevention proof.

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
  close_summary_json TEXT NOT NULL DEFAULT '{}',
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
      'verified_tool_invocation',
      'registered_connection_observation'
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
  channel_kind TEXT NOT NULL CHECK (
    channel_kind IN ('mcp_elicitation', 'prompt_capture', 'local_web_consent', 'cli')
  ),
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
      'verified_tool_invocation',
      'registered_connection_observation'
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
    source_claim_kind IN (
      'host_invocation',
      'guard_event',
      'session_watch_observation'
    )
  ),
  source_claim_id TEXT NOT NULL CHECK (length(trim(source_claim_id)) > 0),
  evidence_capture_intent_id TEXT NOT NULL,
  evidence_capture_receipt_id TEXT NOT NULL,
  capture_kind TEXT NOT NULL CHECK (
    capture_kind IN (
      'verified_command_execution',
      'verified_tool_invocation',
      'registered_connection_observation'
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
      'verified_tool_invocation',
      'registered_connection_observation'
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
  task_id TEXT NOT NULL,
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
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, previous_event_hash)
    REFERENCES authority_events (project_id, event_hash)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE VIEW task_events AS
SELECT
  project_id,
  event_seq,
  event_id,
  task_id,
  change_unit_id,
  state_version,
  event_type AS event_kind,
  payload_json AS event_payload_json,
  created_at
FROM authority_events;

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
  connection_internal_id TEXT NOT NULL,
  guard_installation_id TEXT,
  host_kind TEXT NOT NULL CHECK (length(trim(host_kind)) > 0),
  guard_mode TEXT NOT NULL CHECK (guard_mode IN ('record', 'detective')),
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
CREATE TABLE user_action_channel_tokens (
  project_id TEXT NOT NULL,
  token_hash TEXT NOT NULL CHECK (length(token_hash) = 64),
  channel_kind TEXT NOT NULL CHECK (channel_kind = 'local_web_consent'),
  connection_internal_id TEXT NOT NULL,
  user_action_request_id TEXT NOT NULL,
  capture_basis TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending', 'consumed', 'expired')),
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  consumed_at TEXT,
  completed_at TEXT,
  created_metadata_json TEXT NOT NULL DEFAULT '{}',
  completion_metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, token_hash),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id) ON DELETE RESTRICT,
  FOREIGN KEY (project_id, user_action_request_id)
    REFERENCES user_action_requests (project_id, user_action_request_id)
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

CREATE INDEX idx_user_action_channel_tokens_request
  ON user_action_channel_tokens (project_id, user_action_request_id, status);
CREATE INDEX idx_user_action_channel_tokens_connection
  ON user_action_channel_tokens (project_id, connection_internal_id, channel_kind, status, expires_at);
CREATE INDEX idx_user_action_channel_tokens_expiry
  ON user_action_channel_tokens (project_id, status, expires_at);
```
<!-- canonical-storage-sql: project end -->

Project-state constraints:

- `project_state.state_version` is the only public baseline state clock and must advance monotonically according to [Storage Versioning](storage-versioning.md). It is a Core state clock, not a schema version.
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
- Store reads that derive the latest Agent Session, guard event, session-watch
  baseline, or session-watch observation strict-parse and normalize the
  applicable RFC 3339 timestamps and compare the UTC instants at nanosecond
  precision. SQLite `julianday()`, timestamp-text order, row order, and opaque
  IDs must not determine authority order. Equal maximum instants form a
  co-latest set. Close- and security-relevant guard issue predicates are the
  conservative union across every co-latest guard event. If a read requires
  one session or watch record and multiple distinct candidates are co-latest,
  the selection fails closed as unavailable owner state rather than using an
  ID as a tie-breaker.
- `tasks.work_phase` and `tasks.acceptance_policy` are required controlled
  values. The policy reason is non-empty. A predecessor id, lineage relation,
  and non-empty lineage reason are all-null or all-present, remain in the same
  project through the foreign key, and cannot identify the Task itself.
- `tasks.carry_forward_json` stores typed carry-forward dispositions. It does
  not make a predecessor row, judgment, evidence set, baseline, or write ticket
  current by storage presence alone.
- `authority_events` stores one durable event row per committed authority event. Multiple event rows with the same `state_version` are one event batch for one committed state transition.
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
- `user_action_channel_tokens` stores hashed one-time local-web User Channel tokens for pending user actions. The raw token is not stored. `created_metadata_json` strict-decodes as exactly `{fallback_kind, delivery_surface, endpoint, form_digest}` with `fallback_kind=local_web_consent`, `delivery_surface=model_invisible_user_surface`, `endpoint=/consent`, and a digest matching the canonical form derived from the stored closed request. Missing, extra, wrong-typed, or mismatched metadata—including a legacy row without `delivery_surface`—makes the row permanently unusable under corrected code. The four stored timestamps used to derive and validate the issuance window—`user_action_requests.requested_at`, optional `user_action_requests.expires_at`, token `created_at`, and token `expires_at`—must be the canonical RFC 3339 UTC strings for their instants. Store application validation requires token `created_at >= user_action_requests.requested_at` and requires token `expires_at` to equal exactly `min(user_action_requests.expires_at, created_at + 600 seconds)` when the request has an expiry, or exactly `created_at + 600 seconds` otherwise. A token is valid only in the half-open interval `created_at <= now < expires_at`. Invalid creation metadata, a noncanonical issuance-window timestamp, an earlier or later token expiry, or any other window mismatch is corrupt stored state: token validation, local-web GET or POST, expiry cleanup, and token consumption fail closed without rendering a form or changing token status, the project state or UTC floor, or user-action resolution state. Issuance advances `project_state.updated_at` to at least token `created_at` atomically with insertion. Token consumption and immutable resolution insertion commit together. These rows are transient capture metadata and are not Core user authority by themselves.
- `write_tickets` records single-use write-ticket compatibility. The unique `(project_id, task_id, basis_state_version)` constraint makes the state-version ordering key unique within a Task. The unique indexes on `write_tickets.consumed_by_run_id` and `runs.write_ticket_id` prevent one write-ticket consumption from forking across multiple runs.
- `artifact_staging.created_by_actor_source` records staging provenance. Staged bytes and notices remain artifact-owned and are not evidence authority by themselves.
- `evidence_capture_intents` binds one expiring request to exact current-basis,
  command/tool source input or the Core-derived connection-source selector,
  connection/actor, and workspace facts. A connection intent does not bind a
  future source identifier, timestamp, raw-event digest, snapshot digest, or
  selection digest; those facts are fixed by the source-owned receipt.
  `evidence_capture_receipts` permits exactly one complete, content-bound safe
  receipt and staging handle per intent. `evidence_capture_source_claims`
  atomically claims every normalized underlying host invocation, guard event,
  or session-watch observation for that receipt. Its project-scoped primary key
  prevents the same source fact from fulfilling another intent or producer
  class. Host invocation claim ids are canonical digests over connection,
  session, installation, and invocation coordinates, so host-local ids from
  different exact contexts do not collide. `evidence_producers` enforces
  one-to-one intent, receipt, observation, and artifact finalization, uses a
  composite foreign key to prevent cross-pairing an intent with another
  receipt, and links each producer to one Run. Those constraints do not replace
  Core freshness, relevance, or byte-integrity validation.
- Store application validation rejects missing or extra source coordinates by
  capture class. Command receipts require and claim one host invocation. Tool
  receipts require an exact session and installation, two distinct guard events,
  and one host invocation, and claim all three source facts. Guard connection
  receipts claim exactly one guard event whose kind matches the intent selector;
  watcher receipts claim exactly one session-watch observation from the unique
  current active baseline for the intent-bound connection and session. The
  selected source must be post-intent, complete, and nondegraded, and all
  receipt-fixed identifiers, timestamps, and digests must match that source.
  Receipt staging, receipt insertion, and all claims commit or roll back
  together.
- Store application validation enforces
  `intent.created_at <= receipt.observed_at < intent.expires_at`, receipt
  creation after observation and before intent expiry, and receipt staging
  expiry exactly equal to intent expiry; these cross-row time relations are not
  expressible by the table-local checks alone.
- Store application validation advances `project_state.updated_at` to at least
  `artifact_staging.created_at` in the same transaction that creates ordinary
  staging. Registered evidence-capture fulfillment does the same for receipt
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
- `agent_sessions`, `guard_events`, `prompt_captures`, `expected_writes`, `unrecorded_changes`, `session_watch_baselines`, and `session_watch_observations` are project-local host-observation and session-watch records. They repeat `connection_internal_id` for connection scoping and use project-local keys so records do not leak across projects.
- `guard_events.decision` is constrained to `allow`, `deny`, `warn`, or `inject_context`. These values record local host decision requests; they are not OS-level enforcement proof.
- `expected_writes.status` is constrained to `pending` or `matched`, and `path_policy` is constrained to `exact_paths`. Matched rows must carry the matched post-tool host-hook event, matched paths JSON, and `matched_at`; pending rows must not carry those matched fields.
- `unrecorded_changes.status` is constrained to `unresolved` or `resolved`. Resolved rows must carry resolution JSON, `resolved_at`, and `resolved_by_actor_source`; unresolved rows must not carry those resolution fields.
- `session_watch_baselines.status` is constrained to `disabled`, `active`, `degraded`, or `unavailable`, and `scope_kind` is constrained to `repository` or `path_set`.
- `session_watch_observations.observation_status` is constrained to `unresolved` or `linked`. Linked rows must carry `unrecorded_change_id` and `linked_at`; unresolved rows must not carry those link fields.

## Related Owners

- [Storage Records](storage-records.md) defines persisted record families, placement, relationship layout, storage-owned values, and JSON placement.
- [Storage Effects](storage-effects.md) defines which method branches create, update, observe, or leave records untouched.
- [Storage Versioning](storage-versioning.md) defines the
  `project_state.state_version` clock, canonical Core UTC clock and persisted
  floor, idempotency, replay, events, locks, and incompatible-storage handling.
- [Agent Connection](agent-connection.md) defines Agent Connection, Connection Projects, current connection context, mode gating, and Agent Connection versus User Channel boundaries.
- [Security](security.md) defines security boundaries and guarantee levels.
