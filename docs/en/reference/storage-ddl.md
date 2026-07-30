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

Labels use
[Documentation Policy](../maintain/documentation-policy.md#surface-stability-labels).

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

Production code opens `registry.sqlite` or project `state.sqlite` for writing
only through crate-private helpers that require a live
`RuntimeHomeMutationContext`. The project record and database path must belong
to that context's exact canonical Runtime Home. Read-only helpers remain
separate and require no mutation context. Setup-only staged database creation
requires an exclusive setup context and stays within bootstrap.

Mutating transactions must use `BEGIN IMMEDIATE` or an equivalent serialized
write boundary before reading freshness, write-ticket compatibility rows,
staging, replay rows, or the persisted canonical-UTC floor for a commit.

Authority rows remain addressable unless an owning storage contract defines a repair or retention path. The registry may cascade-delete non-authority alias rows that are owned by a forgotten project registration; it must not use alias cleanup to imply deletion of project-local Core authority records.

SQLite `TEXT` columns ending in `_json` store JSON as a representation choice. JSON used for authority, lifecycle, scope, evidence, completion, close readiness, or write compatibility is typed owner state. Typed Core code must parse and validate those columns before commit against the applicable API schema owner, storage owner, or artifact owner. Failure to decode typed owner state is corruption and must never be converted to an empty object, empty array, false value, default enum, or "no requirement" interpretation. SQL `NULL` may mean absence only when the owning schema explicitly marks the field optional; malformed JSON in an optional column is corruption, not absence. Open-ended display metadata may remain untyped only when it is not used for authority or close decisions. Safe diagnostics may identify the table, record reference, logical column, and corruption category, but must not expose raw stored JSON, secrets, SQL text, or sensitive absolute paths. SQLite defaults such as `'{}'` and `'[]'` do not make API fields optional.

`project_state.state_version` is the only public state clock.
`project_state.updated_at` is a distinct physical floor for the canonical Core
UTC clock, not a public conflict version or storage-format identity. Canonical
SQLite DDL exposes no other public state clock.

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

`storage_profile` stores the one deterministic canonical UTF-8 JSON encoding
of the complete current `StorageManifest`. The object
has exactly `contract_id`, `canonical_ddl_digest`,
`integrity_constraints_digest`, and `enabled_capabilities`; missing, unknown,
or duplicate members are invalid. The capability array must preserve the
complete sorted, duplicate-free set owned by
[Storage Versioning](storage-versioning.md).

Fresh initialization writes the same current manifest value into the registry
carrier and every newly created project carrier. A fresh Runtime Home Registry
is created only in a same-parent staging directory; its singleton and initial
installation row are committed and the exact DDL inventory and manifest
carrier are validated before the directory is atomically published without
replacement. Store strict-decodes each carrier independently before reading
authority or policy records. It requires the persisted value to equal the
current built-in manifest and requires a selected project's manifest to equal
the registry manifest. It does not parse an integer, compare versions, inspect
field presence to select a decoder, or try another profile. Existing carrier
inspection is read-only and preserves an incompatible database. The exact
open result and failure category remain with
[Storage Versioning](storage-versioning.md).

The current project schema normalizes source-aware host correlation into
`host_sessions`, `host_turns`, `host_tool_invocations`, and the MCP-only
`managed_mcp_sessions` table. Their constraints and the phase-discriminated
Guard columns are part of both current schema digests. Strict open accepts only
the complete current manifest identity.

The application selects the distinct `CodexMcpTurnMetadata` and
`CodexCommandHooks` markers before storage; the host-contract owner maps them to the
reviewed profile IDs. DDL stores only the resulting bounded,
source-discriminated coordinates plus the owned profile/digest fields; it does
not store raw host envelopes or infer a decoder from column presence.

## Canonical SQL Sources

The only executable DDL sources are
[`registry.sql`](../../../crates/volicord-store/src/schema/registry.sql) and
[`project.sql`](../../../crates/volicord-store/src/schema/project.sql), in that
fixed source order. Fresh initialization applies them only to empty SQLite
databases. Existing databases are accepted only through exact current-manifest
and physical-schema validation.

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

`registry.sqlite` stores Runtime Home identity, installation profile records, project registration, project aliases, Agent Connection records, Connection Projects membership, structured diagnostic findings and cause edges, authoritative MCP runtime sessions and project reservations, bounded in-chat integration-verification runs, host-hook installation records, and host configuration inventory. It does not store project-local Core state.

<!-- canonical-storage-sql: registry start -->
```sql
CREATE TABLE runtime_home (
  singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
  runtime_home_id TEXT NOT NULL UNIQUE,
  publication_id TEXT NOT NULL UNIQUE CHECK (
    length(publication_id) = 61
    AND substr(publication_id, 1, 25) = 'runtime_home_publication_'
    AND substr(publication_id, 34, 1) = '-'
    AND substr(publication_id, 39, 1) = '-'
    AND substr(publication_id, 44, 1) = '-'
    AND substr(publication_id, 49, 1) = '-'
    AND substr(publication_id, 26, 8) NOT GLOB '*[^0-9a-f]*'
    AND substr(publication_id, 35, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(publication_id, 40, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(publication_id, 45, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(publication_id, 50, 12) NOT GLOB '*[^0-9a-f]*'
    AND substr(publication_id, 40, 1) = '4'
    AND substr(publication_id, 45, 1) GLOB '[89ab]'
  ),
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

CREATE TABLE diagnostic_findings (
  finding_id TEXT PRIMARY KEY CHECK (
    length(CAST(finding_id AS BLOB)) BETWEEN 1 AND 192
    AND substr(finding_id, 1, 1) GLOB '[a-z]'
    AND substr(finding_id, -1, 1) GLOB '[a-z0-9]'
    AND finding_id NOT GLOB '*[^a-z0-9_.:-]*'
  ),
  lifecycle TEXT NOT NULL CHECK (lifecycle IN ('occurrence', 'current_state')),
  current_identity_digest TEXT CHECK (
    current_identity_digest IS NULL
    OR (
      length(current_identity_digest) = 64
      AND current_identity_digest NOT GLOB '*[^0-9a-f]*'
    )
  ),
  current_subject_identity TEXT CHECK (
    current_subject_identity IS NULL
    OR (
      length(current_subject_identity) = 71
      AND substr(current_subject_identity, 1, 7) = 'sha256:'
      AND substr(current_subject_identity, 8) NOT GLOB '*[^0-9a-f]*'
    )
  ),
  diagnostic_scope_kind TEXT CHECK (
    diagnostic_scope_kind IS NULL
    OR diagnostic_scope_kind IN ('connection', 'project', 'runtime_home', 'installation', 'process')
  ),
  diagnostic_scope_identity TEXT CHECK (
    diagnostic_scope_identity IS NULL
    OR length(CAST(diagnostic_scope_identity AS BLOB)) BETWEEN 1 AND 1024
  ),
  current_state_status TEXT CHECK (
    current_state_status IS NULL
    OR current_state_status IN ('active', 'resolved')
  ),
  resolved_at TEXT,
  code TEXT NOT NULL CHECK (
    length(CAST(code AS BLOB)) BETWEEN 3 AND 192
    AND instr(code, '.') > 1
    AND code NOT GLOB '*[^a-z0-9_.]*'
  ),
  domain TEXT NOT NULL CHECK (
    length(CAST(domain AS BLOB)) BETWEEN 1 AND 128
    AND substr(domain, 1, 1) GLOB '[a-z]'
    AND domain NOT GLOB '*[^a-z0-9_]*'
  ),
  stage TEXT NOT NULL CHECK (
    length(CAST(stage AS BLOB)) BETWEEN 1 AND 128
    AND substr(stage, 1, 1) GLOB '[a-z]'
    AND stage NOT GLOB '*[^a-z0-9_]*'
  ),
  severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'error')),
  source TEXT NOT NULL CHECK (
    length(CAST(source AS BLOB)) BETWEEN 1 AND 128
    AND substr(source, 1, 1) GLOB '[a-z]'
    AND source NOT GLOB '*[^a-z0-9_]*'
  ),
  subject_json TEXT NOT NULL CHECK (
    json_valid(subject_json)
    AND json_type(subject_json) = 'object'
    AND length(CAST(subject_json AS BLOB)) <= 4096
  ),
  facts_json TEXT NOT NULL CHECK (
    json_valid(facts_json)
    AND json_type(facts_json) = 'object'
    AND length(CAST(facts_json AS BLOB)) <= 16384
  ),
  actions_json TEXT NOT NULL CHECK (
    json_valid(actions_json)
    AND json_type(actions_json) = 'array'
    AND length(CAST(actions_json AS BLOB)) <= 65536
  ),
  correlation_id TEXT CHECK (
    correlation_id IS NULL
    OR length(CAST(correlation_id AS BLOB)) BETWEEN 1 AND 192
  ),
  connection_internal_id TEXT CHECK (
    connection_internal_id IS NULL
    OR length(CAST(connection_internal_id AS BLOB)) BETWEEN 1 AND 192
  ),
  project_internal_id TEXT CHECK (
    project_internal_id IS NULL
    OR length(CAST(project_internal_id AS BLOB)) BETWEEN 1 AND 192
  ),
  runtime_session_id TEXT CHECK (
    runtime_session_id IS NULL
    OR length(CAST(runtime_session_id AS BLOB)) BETWEEN 1 AND 192
  ),
  integration_revision TEXT CHECK (
    integration_revision IS NULL
    OR (
      length(integration_revision) = 71
      AND substr(integration_revision, 1, 7) = 'sha256:'
      AND substr(integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
    )
  ),
  observed_at TEXT NOT NULL,
  UNIQUE (finding_id, runtime_session_id),
  CHECK (
    runtime_session_id IS NULL
    OR (connection_internal_id IS NOT NULL AND integration_revision IS NOT NULL)
  ),
  CHECK (
    (
      lifecycle = 'occurrence'
      AND current_identity_digest IS NULL
      AND current_subject_identity IS NULL
      AND diagnostic_scope_kind IS NULL
      AND diagnostic_scope_identity IS NULL
      AND current_state_status IS NULL
      AND resolved_at IS NULL
    )
    OR (
      lifecycle = 'current_state'
      AND current_identity_digest IS NOT NULL
      AND current_subject_identity IS NOT NULL
      AND diagnostic_scope_kind IS NOT NULL
      AND diagnostic_scope_identity IS NOT NULL
      AND current_state_status IS NOT NULL
      AND runtime_session_id IS NULL
      AND finding_id = 'finding.current.sha256:' || current_identity_digest
      AND (
        (current_state_status = 'active' AND resolved_at IS NULL)
        OR (current_state_status = 'resolved' AND resolved_at IS NOT NULL)
      )
    )
  )
);

CREATE TABLE diagnostic_cause_edges (
  finding_id TEXT NOT NULL,
  cause_finding_id TEXT NOT NULL,
  PRIMARY KEY (finding_id, cause_finding_id),
  FOREIGN KEY (finding_id)
    REFERENCES diagnostic_findings (finding_id)
    ON DELETE CASCADE,
  FOREIGN KEY (cause_finding_id)
    REFERENCES diagnostic_findings (finding_id)
    ON DELETE RESTRICT,
  CHECK (finding_id <> cause_finding_id)
);

CREATE INDEX idx_diagnostic_findings_runtime_session
  ON diagnostic_findings (runtime_session_id, observed_at, finding_id)
  WHERE lifecycle = 'occurrence' AND runtime_session_id IS NOT NULL;
CREATE UNIQUE INDEX idx_diagnostic_findings_current_identity
  ON diagnostic_findings (current_identity_digest)
  WHERE lifecycle = 'current_state';
CREATE INDEX idx_diagnostic_findings_active_current_scope
  ON diagnostic_findings (
    diagnostic_scope_kind, diagnostic_scope_identity, observed_at, finding_id
  )
  WHERE lifecycle = 'current_state' AND current_state_status = 'active';
CREATE INDEX idx_diagnostic_findings_project
  ON diagnostic_findings (project_internal_id, observed_at, finding_id)
  WHERE project_internal_id IS NOT NULL;
CREATE INDEX idx_diagnostic_cause_edges_cause
  ON diagnostic_cause_edges (cause_finding_id, finding_id);

CREATE TRIGGER diagnostic_cause_edges_acyclic
BEFORE INSERT ON diagnostic_cause_edges
BEGIN
  SELECT CASE WHEN EXISTS (
    WITH RECURSIVE causes(finding_id) AS (
      SELECT cause_finding_id
        FROM diagnostic_cause_edges
       WHERE finding_id = NEW.cause_finding_id
      UNION
      SELECT edge.cause_finding_id
        FROM diagnostic_cause_edges AS edge
        JOIN causes ON edge.finding_id = causes.finding_id
    )
    SELECT 1 FROM causes WHERE finding_id = NEW.finding_id
  ) THEN RAISE(ABORT, 'diagnostic cause cycle') END;
END;

CREATE TRIGGER diagnostic_occurrence_findings_immutable
BEFORE UPDATE ON diagnostic_findings
WHEN OLD.lifecycle = 'occurrence'
BEGIN
  SELECT RAISE(ABORT, 'diagnostic occurrence findings are immutable');
END;

CREATE TRIGGER diagnostic_current_identity_immutable
BEFORE UPDATE OF
  finding_id,
  lifecycle,
  current_identity_digest,
  current_subject_identity,
  diagnostic_scope_kind,
  diagnostic_scope_identity,
  code,
  domain,
  stage,
  source
ON diagnostic_findings
WHEN OLD.lifecycle = 'current_state'
BEGIN
  SELECT RAISE(ABORT, 'diagnostic current identity is immutable');
END;

CREATE TABLE managed_mcp_launch_leases (
  launch_lease_id TEXT PRIMARY KEY CHECK (
    length(launch_lease_id) = 53
    AND substr(launch_lease_id, 1, 17) = 'mcp_launch_lease_'
    AND substr(launch_lease_id, 26, 1) = '-'
    AND substr(launch_lease_id, 31, 1) = '-'
    AND substr(launch_lease_id, 36, 1) = '-'
    AND substr(launch_lease_id, 41, 1) = '-'
    AND substr(launch_lease_id, 18, 8) NOT GLOB '*[^0-9a-f]*'
    AND substr(launch_lease_id, 27, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(launch_lease_id, 32, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(launch_lease_id, 37, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(launch_lease_id, 42, 12) NOT GLOB '*[^0-9a-f]*'
    AND substr(launch_lease_id, 32, 1) = '4'
    AND substr(launch_lease_id, 37, 1) GLOB '[89ab]'
  ),
  connection_internal_id TEXT NOT NULL,
  host_kind TEXT NOT NULL CHECK (host_kind = 'codex'),
  expected_integration_revision TEXT NOT NULL CHECK (
    length(expected_integration_revision) = 71
    AND substr(expected_integration_revision, 1, 7) = 'sha256:'
    AND substr(expected_integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  expected_launch_fingerprint TEXT NOT NULL CHECK (
    length(CAST(expected_launch_fingerprint AS BLOB)) BETWEEN 1 AND 1024
  ),
  issued_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  consumed_at TEXT,
  terminal_state TEXT NOT NULL CHECK (
    terminal_state IN ('issued', 'consumed', 'cancelled', 'expired')
  ),
  FOREIGN KEY (connection_internal_id)
    REFERENCES agent_connections (connection_internal_id)
    ON DELETE RESTRICT,
  CHECK (expires_at > issued_at),
  CHECK (
    (terminal_state = 'consumed' AND consumed_at IS NOT NULL)
    OR (terminal_state <> 'consumed' AND consumed_at IS NULL)
  ),
  CHECK (consumed_at IS NULL OR consumed_at >= issued_at),
  CHECK (consumed_at IS NULL OR consumed_at < expires_at)
);

CREATE INDEX idx_managed_mcp_launch_leases_cleanup
  ON managed_mcp_launch_leases (
    connection_internal_id, terminal_state, expires_at
  );


CREATE TABLE mcp_runtime_sessions (
  runtime_session_id TEXT PRIMARY KEY,
  connection_internal_id TEXT NOT NULL,
  session_source TEXT NOT NULL CHECK (
    session_source IN ('managed_host', 'manual_cli', 'cli_preflight', 'integration_probe')
  ),
  connection_integration_revision TEXT NOT NULL CHECK (
    length(connection_integration_revision) = 71
    AND substr(connection_integration_revision, 1, 7) = 'sha256:'
    AND substr(connection_integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  observed_host_executable_version TEXT,
  attempted_client_name TEXT,
  attempted_client_version TEXT,
  requested_protocol_version TEXT,
  selected_protocol_version TEXT,
  negotiated_protocol_version TEXT,
  process_id INTEGER NOT NULL CHECK (process_id > 0),
  process_started_at TEXT NOT NULL,
  initialize_completed_at TEXT,
  initialized_notification_at TEXT,
  tools_list_observed_at TEXT,
  returned_tool_identities_json TEXT CHECK (
    returned_tool_identities_json IS NULL
    OR (
      json_valid(returned_tool_identities_json)
      AND json_type(returned_tool_identities_json) = 'array'
    )
  ),
  required_tools_present INTEGER CHECK (required_tools_present IN (0, 1)),
  required_tools_validated_at TEXT,
  verification_tool_name TEXT CHECK (
    verification_tool_name IS NULL
    OR (
      length(CAST(verification_tool_name AS BLOB)) BETWEEN 1 AND 128
      AND length(verification_tool_name) = length(CAST(verification_tool_name AS BLOB))
      AND verification_tool_name NOT GLOB '*[^A-Za-z0-9_.-]*'
    )
  ),
  verification_tool_observed_at TEXT,
  last_observed_at TEXT NOT NULL,
  terminal_finding_id TEXT,
  graceful_close_at TEXT,
  UNIQUE (runtime_session_id, connection_internal_id),
  FOREIGN KEY (connection_internal_id)
    REFERENCES agent_connections (connection_internal_id)
    ON DELETE RESTRICT,
  FOREIGN KEY (terminal_finding_id, runtime_session_id)
    REFERENCES diagnostic_findings (finding_id, runtime_session_id)
    ON DELETE RESTRICT,
  CHECK (
    (attempted_client_name IS NULL AND attempted_client_version IS NULL)
    OR (attempted_client_name IS NOT NULL AND attempted_client_version IS NOT NULL)
  ),
  CHECK (
    (initialize_completed_at IS NULL AND selected_protocol_version IS NULL)
    OR (initialize_completed_at IS NOT NULL AND selected_protocol_version IS NOT NULL)
  ),
  CHECK (selected_protocol_version IS NULL OR requested_protocol_version IS NOT NULL),
  CHECK (selected_protocol_version IS NULL OR attempted_client_name IS NOT NULL),
  CHECK (
    (initialized_notification_at IS NULL AND negotiated_protocol_version IS NULL)
    OR (initialized_notification_at IS NOT NULL AND negotiated_protocol_version IS NOT NULL)
  ),
  CHECK (
    (
      tools_list_observed_at IS NULL
      AND returned_tool_identities_json IS NULL
      AND required_tools_present IS NULL
      AND required_tools_validated_at IS NULL
    )
    OR (
      tools_list_observed_at IS NOT NULL
      AND returned_tool_identities_json IS NOT NULL
      AND required_tools_present = 0
      AND required_tools_validated_at IS NULL
    )
    OR (
      tools_list_observed_at IS NOT NULL
      AND returned_tool_identities_json IS NOT NULL
      AND required_tools_present = 1
      AND required_tools_validated_at IS NOT NULL
    )
  ),
  CHECK (
    (verification_tool_name IS NULL AND verification_tool_observed_at IS NULL)
    OR (verification_tool_name IS NOT NULL AND verification_tool_observed_at IS NOT NULL)
  ),
  CHECK (initialized_notification_at IS NULL OR initialize_completed_at IS NOT NULL),
  CHECK (negotiated_protocol_version IS NULL OR negotiated_protocol_version = selected_protocol_version),
  CHECK (tools_list_observed_at IS NULL OR initialize_completed_at IS NOT NULL),
  CHECK (required_tools_validated_at IS NULL OR required_tools_validated_at >= tools_list_observed_at),
  CHECK (verification_tool_observed_at IS NULL OR required_tools_validated_at IS NOT NULL),
  CHECK (terminal_finding_id IS NULL OR graceful_close_at IS NULL),
  CHECK (last_observed_at >= process_started_at),
  CHECK (initialize_completed_at IS NULL OR initialize_completed_at >= process_started_at),
  CHECK (initialized_notification_at IS NULL OR initialized_notification_at >= initialize_completed_at),
  CHECK (tools_list_observed_at IS NULL OR tools_list_observed_at >= initialize_completed_at),
  CHECK (verification_tool_observed_at IS NULL OR verification_tool_observed_at >= initialized_notification_at),
  CHECK (verification_tool_observed_at IS NULL OR verification_tool_observed_at >= required_tools_validated_at),
  CHECK (terminal_finding_id IS NULL OR last_observed_at >= process_started_at),
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
    verification_tool_observed_at
  )
  WHERE session_source = 'managed_host'
    AND initialized_notification_at IS NOT NULL
    AND required_tools_validated_at IS NOT NULL
    AND verification_tool_name IS NOT NULL
    AND verification_tool_observed_at IS NOT NULL;

CREATE TABLE mcp_runtime_project_session_bindings (
  runtime_session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  project_internal_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  project_integration_revision TEXT NOT NULL CHECK (
    length(project_integration_revision) = 71
    AND substr(project_integration_revision, 1, 7) = 'sha256:'
    AND substr(project_integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
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
  ON mcp_runtime_project_session_bindings (
    project_internal_id, connection_internal_id, project_integration_revision, bound_at
  );

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

CREATE TABLE guard_integration_verification_runs (
  verification_id TEXT PRIMARY KEY CHECK (
    length(CAST(verification_id AS BLOB)) BETWEEN 1 AND 192
    AND substr(verification_id, 1, 19) = 'guard_verification_'
  ),
  connection_internal_id TEXT NOT NULL,
  project_internal_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  runtime_session_id TEXT NOT NULL,
  host_session_id TEXT NOT NULL,
  host_turn_id TEXT NOT NULL,
  integration_revision TEXT NOT NULL CHECK (
    length(integration_revision) = 71
    AND substr(integration_revision, 1, 7) = 'sha256:'
    AND substr(integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  guard_installation_id TEXT NOT NULL,
  host_contract_profile TEXT NOT NULL CHECK (
    host_contract_profile = 'codex-command-hooks'
  ),
  hook_definition_digest TEXT NOT NULL CHECK (
    length(hook_definition_digest) = 71
    AND substr(hook_definition_digest, 1, 7) = 'sha256:'
    AND substr(hook_definition_digest, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  policy_digest TEXT NOT NULL CHECK (
    length(policy_digest) = 71
    AND substr(policy_digest, 1, 7) = 'sha256:'
    AND substr(policy_digest, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  expected_probe_tool TEXT NOT NULL CHECK (
    expected_probe_tool = 'volicord.guard_probe'
  ),
  expected_host_callable_name TEXT NOT NULL CHECK (
    length(CAST(expected_host_callable_name AS BLOB)) BETWEEN 1 AND 64
    AND expected_host_callable_name NOT GLOB '*[^A-Za-z0-9_]*'
  ),
  observation_policy_kind TEXT NOT NULL CHECK (
    observation_policy_kind IN ('synchronous', 'deferred')
  ),
  observation_deadline_at TEXT,
  allowed_status_reads INTEGER NOT NULL CHECK (
    allowed_status_reads BETWEEN 1 AND 255
  ),
  status_read_count INTEGER NOT NULL DEFAULT 0 CHECK (
    status_read_count BETWEEN 0 AND allowed_status_reads
  ),
  created_at TEXT NOT NULL,
  cleanup_after TEXT NOT NULL,
  status TEXT NOT NULL CHECK (
    status IN ('awaiting_probe', 'awaiting_observation', 'complete', 'repair_required')
  ),
  probe_acknowledged_at TEXT,
  completed_at TEXT,
  matched_prompt_event_id TEXT NOT NULL,
  matched_pre_tool_event_id TEXT,
  matched_post_tool_event_id TEXT,
  repair_reason TEXT CHECK (
    repair_reason IS NULL
    OR repair_reason IN (
      'hook_event_not_observed',
      'hook_payload_incompatible',
      'callable_identity_mismatch',
      'verification_id_mismatch',
      'session_mismatch',
      'turn_mismatch',
      'tool_use_mismatch',
      'integration_revision_changed',
      'hook_definition_changed',
      'policy_changed',
      'observation_deadline_exceeded'
    )
  ),
  retry_policy TEXT CHECK (
    retry_policy IS NULL
    OR retry_policy IN (
      'no_automatic_retry',
      'new_turn_required',
      'host_reload_required',
      'hook_review_required',
      'repair_required'
    )
  ),
  terminal_finding_code TEXT CHECK (
    terminal_finding_code IS NULL
    OR (
      length(CAST(terminal_finding_code AS BLOB)) BETWEEN 1 AND 128
      AND substr(terminal_finding_code, 1, 1) GLOB '[a-z]'
      AND terminal_finding_code NOT GLOB '*[^a-z0-9_]'
    )
  ),
  terminal_finding_summary TEXT CHECK (
    terminal_finding_summary IS NULL
    OR length(CAST(terminal_finding_summary AS BLOB)) BETWEEN 1 AND 4096
  ),
  FOREIGN KEY (runtime_session_id, connection_internal_id)
    REFERENCES mcp_runtime_sessions (runtime_session_id, connection_internal_id)
    ON DELETE RESTRICT,
  FOREIGN KEY (connection_internal_id, project_internal_id)
    REFERENCES connection_projects (connection_internal_id, project_internal_id)
    ON DELETE RESTRICT,
  FOREIGN KEY (guard_installation_id)
    REFERENCES guard_installations (guard_installation_id)
    ON DELETE RESTRICT,
  CHECK (cleanup_after > created_at),
  CHECK (
    (observation_policy_kind = 'synchronous' AND observation_deadline_at IS NULL)
    OR (
      observation_policy_kind = 'deferred'
      AND (
        (status = 'awaiting_probe' AND observation_deadline_at IS NULL)
        OR observation_deadline_at > probe_acknowledged_at
      )
    )
  ),
  CHECK (probe_acknowledged_at IS NULL OR probe_acknowledged_at >= created_at),
  CHECK (
    (status = 'awaiting_probe' AND probe_acknowledged_at IS NULL
      AND completed_at IS NULL AND repair_reason IS NULL AND retry_policy IS NULL
      AND terminal_finding_code IS NULL AND terminal_finding_summary IS NULL)
    OR (status = 'awaiting_observation' AND probe_acknowledged_at IS NOT NULL
      AND completed_at IS NULL AND repair_reason IS NULL AND retry_policy IS NULL
      AND terminal_finding_code IS NULL AND terminal_finding_summary IS NULL)
    OR (status = 'complete' AND probe_acknowledged_at IS NOT NULL
      AND completed_at IS NOT NULL AND repair_reason IS NULL AND retry_policy IS NULL
      AND terminal_finding_code IS NULL AND terminal_finding_summary IS NULL
      AND matched_pre_tool_event_id IS NOT NULL AND matched_post_tool_event_id IS NOT NULL)
    OR (status = 'repair_required' AND completed_at IS NOT NULL
      AND repair_reason IS NOT NULL AND retry_policy IS NOT NULL
      AND terminal_finding_code IS NOT NULL AND terminal_finding_summary IS NOT NULL)
  ),
  CHECK (
    (matched_pre_tool_event_id IS NULL AND matched_post_tool_event_id IS NULL)
    OR matched_pre_tool_event_id IS NOT NULL
  )
);

CREATE UNIQUE INDEX idx_guard_integration_verification_coordinate
  ON guard_integration_verification_runs (
    connection_internal_id, project_id, runtime_session_id, host_session_id,
    host_turn_id, integration_revision, guard_installation_id,
    host_contract_profile, hook_definition_digest, policy_digest
  );
CREATE UNIQUE INDEX idx_guard_integration_verification_prompt_attempt
  ON guard_integration_verification_runs (project_internal_id, matched_prompt_event_id);
CREATE INDEX idx_guard_integration_verification_project
  ON guard_integration_verification_runs (
    project_internal_id, connection_internal_id, created_at, verification_id
  );

CREATE TRIGGER guard_integration_verification_coordinate_immutable
BEFORE UPDATE OF
  connection_internal_id, project_internal_id, project_id, runtime_session_id,
  host_session_id, host_turn_id, integration_revision, guard_installation_id,
  host_contract_profile, hook_definition_digest, policy_digest
ON guard_integration_verification_runs
BEGIN
  SELECT RAISE(ABORT, 'guard integration verification coordinate is immutable');
END;

CREATE TRIGGER guard_integration_verification_probe_ack_immutable
BEFORE UPDATE OF probe_acknowledged_at
ON guard_integration_verification_runs
WHEN OLD.probe_acknowledged_at IS NOT NULL
BEGIN
  SELECT RAISE(ABORT, 'guard integration verification probe acknowledgement is immutable');
END;

CREATE TRIGGER guard_integration_verification_terminal_immutable
BEFORE UPDATE ON guard_integration_verification_runs
WHEN OLD.status IN ('complete', 'repair_required')
BEGIN
  SELECT RAISE(ABORT, 'guard integration verification terminal state is immutable');
END;

CREATE TABLE guard_probe_observations (
  observation_id TEXT PRIMARY KEY CHECK (
    length(CAST(observation_id AS BLOB)) BETWEEN 1 AND 192
  ),
  verification_id TEXT NOT NULL,
  guard_event_id TEXT CHECK (
    guard_event_id IS NULL
    OR length(CAST(guard_event_id AS BLOB)) BETWEEN 1 AND 192
  ),
  stage TEXT NOT NULL CHECK (
    stage IN (
      'probe_acknowledged',
      'unrelated_routed_tool',
      'hook_event_not_observed',
      'hook_payload_incompatible',
      'callable_identity_unknown',
      'callable_identity_mismatch',
      'verification_id_mismatch',
      'session_mismatch',
      'turn_mismatch',
      'tool_use_mismatch',
      'pre_tool_matched',
      'post_tool_matched'
    )
  ),
  expected_agent_tool_id TEXT NOT NULL CHECK (
    expected_agent_tool_id = 'volicord.guard_probe'
  ),
  expected_host_callable_name TEXT NOT NULL CHECK (
    length(CAST(expected_host_callable_name AS BLOB)) BETWEEN 1 AND 64
    AND expected_host_callable_name NOT GLOB '*[^A-Za-z0-9_]*'
  ),
  observed_callable_name TEXT CHECK (
    observed_callable_name IS NULL
    OR length(CAST(observed_callable_name AS BLOB)) BETWEEN 1 AND 256
  ),
  hook_event_kind TEXT CHECK (
    hook_event_kind IS NULL OR hook_event_kind IN ('pre_tool', 'post_tool')
  ),
  verification_id_present INTEGER NOT NULL CHECK (
    verification_id_present IN (0, 1)
  ),
  verification_id_matches INTEGER NOT NULL CHECK (
    verification_id_matches IN (0, 1)
  ),
  guard_installation_id TEXT NOT NULL,
  integration_revision TEXT NOT NULL CHECK (
    length(integration_revision) = 71
    AND substr(integration_revision, 1, 7) = 'sha256:'
    AND substr(integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  observed_at TEXT NOT NULL,
  CHECK (verification_id_matches = 0 OR verification_id_present = 1),
  FOREIGN KEY (verification_id)
    REFERENCES guard_integration_verification_runs (verification_id)
    ON DELETE CASCADE,
  FOREIGN KEY (guard_installation_id)
    REFERENCES guard_installations (guard_installation_id)
    ON DELETE RESTRICT
);

CREATE INDEX idx_guard_probe_observations_verification
  ON guard_probe_observations (verification_id, observed_at, observation_id);
```
<!-- canonical-storage-sql: registry end -->

Registry constraints:

- `runtime_home` is a singleton table. Its `storage_profile` column is the required manifest carrier and stores the complete current `StorageManifest`; the row also stores Runtime Home identity, the unique `runtime_home_publication_`-prefixed lowercase UUIDv4 publication provenance, the Runtime Home path, the registry database path, metadata, and timestamps. The publication ID identifies one preparation invocation and is not a credential, OS actor identity, or numeric schema selector. The stored `runtime_home_id` identifies the Runtime Home record; it is not a security guarantee.
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
- The integration generation distinguishes revisions within one physical Connection instance, while `integration_instance_id` distinguishes physical deletion and recreation. They are Store-owned local lifecycle and correlation coordinates, and callers cannot select either value.
- `agent_connections.verification_report_json` is SQL null when no completed report exists. A non-null value stores one strict canonical `ConnectionVerificationReport`, including its derived status and actions; absent optional members are omitted rather than encoded as explicit null. Store does not persist those components independently.
- `connection_projects` is the explicit project allowlist for one Agent Connection. It stores membership with `connection_internal_id` and `project_internal_id`. Deleting a project or connection that still has membership is restricted.
- `managed_mcp_launch_leases` stores short-lived, one-time hidden-launcher authority. Its expected Connection, `codex` host kind, integration revision, and managed launch fingerprint must still be current when Store atomically changes `issued` to `consumed` and inserts the `managed_host` runtime. Replay, expiry, mismatch, or cancellation cannot create a runtime. Bounded cleanup expires or removes old rows. The lease is an evidence-integrity coordinate, not an OS actor credential.
- `diagnostic_findings.lifecycle` is exactly `occurrence` or `current_state`. Occurrence rows have no current identity or status fields and are immutable. Current rows require a full 64-character lowercase identity digest, a validated `sha256:<64 lowercase hex>` `current_subject_identity`, scope kind and complete scope identity, active/resolved status, no runtime-session coordinate, and an ID exactly equal to `finding.current.sha256:` plus that digest. Active rows have no `resolved_at`; resolved rows require it. The unique digest index, active-scope index, lifecycle checks, and identity update trigger enforce those physical distinctions. The trigger keeps the subject identity immutable while allowing `subject_json` to change as replaceable safe presentation. `facts_json` remains a valid JSON object bounded to 16,384 bytes; `subject_json` and `actions_json` are likewise bounded typed representations.
- `diagnostic_cause_edges` stores unique finding-to-cause pairs with foreign keys on both ends. `diagnostic_cause_edges_acyclic` rejects an insert that would close a directed cycle, while the cause-side index supports deterministic reverse and bounded traversal. Replacing a current-state finding deletes its prior outgoing edges and inserts the replacement edges in the same immediate transaction as the row replacement; any failure preserves the prior row and edge set.
- `mcp_runtime_sessions.attempted_client_name` and `attempted_client_version` form the bounded parsed client pair. `requested_protocol_version` is client input; `selected_protocol_version` is the server-selected initialize result; `negotiated_protocol_version` is present only with handshake completion and must equal the selected revision. `initialize_completed_at`, `initialized_notification_at`, and `tools_list_observed_at` are distinct lifecycle milestones; `tools/list` may follow initialize completion before the initialized notification. `returned_tool_identities_json` is the canonical exact inventory for that list observation, and `required_tools_validated_at` is present only for a successful required set. The bounded MCP tool name `verification_tool_name` and `verification_tool_observed_at` form an exact null-or-present pair; the observation requires same-session required-tool validation and cannot precede it. `terminal_finding_id` is a same-runtime foreign key to one structured error finding and is mutually exclusive with graceful close.
- `mcp_runtime_sessions.session_source` is exactly `managed_host`, `manual_cli`, `cli_preflight`, or `integration_probe`. Only the lease-consumption transaction may insert `managed_host`; managed-session lookups exclude the other three values.
- `guard_installations` stores one stable project-scoped Guard installation identity and its canonical typed Guard manifest. The manifest is bound to the row, Agent Connection, project, current integration revision, policy hash, runtime commands, complete managed-file inventory, required hook phases, exact `host_contract_profile`, and deterministic `host_contract_digest`. The current Guard selection is `codex-command-hooks`. File state is audited from the manifest and current files, while observation state requires compatible current-owned `guard_events` for every required phase. These cooperative checks do not provide OS-level enforcement or write prevention.
- `guard_integration_verification_runs` stores one immutable managed-host attempt per full semantic coordinate: Connection, project, current MCP runtime, native session and turn, integration revision, Guard Installation, host-contract profile, hook-definition digest, and policy digest. Its unconditional unique index includes terminal rows, and prompt ownership prevents separate attempts from sharing one prompt event. The row also stores the semantic observation policy, bounded status-read count, cleanup boundary, first-write acknowledgement, matched events, closed state, and typed repair/retry fields. Coordinate, acknowledgement, and terminal triggers prevent identity mutation, a second acknowledgement, terminal reactivation, or terminal replacement. The current Codex semantic contract uses the stored synchronous policy with one allowed status read. `cleanup_after` is retention metadata, not attempt expiry, polling time, or retry eligibility.
- `guard_probe_observations` stores only the closed acquisition stage, expected agent-tool/callable identity, optional bounded observed callable, optional hook kind, verification-ID presence/match flags, Guard Installation, integration revision, and observation time. It cannot store prompts or unrestricted hook/tool payloads. Its foreign keys attach each observation to one verification run and current installation; `hook_event_not_observed` records only absence at the Volicord boundary. `unrelated_routed_tool` is nonterminal trace and is not a repair reason, proof, acknowledgement, retry input, root finding, or status-read-budget effect.
- Connection Project retirement by explicit removal or replacement cleanup satisfies the restrictive Registry foreign keys by owner-ordered deletion in one immediate transaction. It deletes selected project-session bindings and integration-verification runs before the selected Guard Installation and membership. Multi-project replacement cleanup leaves unrelated project rows and connection-wide runtime sessions intact. Last-project replacement cleanup retains the complete disabled membership, binding, Guard Installation, and pending-cleanup-marker inventory until host cleanup and final revalidation succeed, then deletes only the project-owned rows and membership. Explicit final-membership removal deletes every remaining connection-owned binding, integration-verification run, and Guard Installation, then `mcp_runtime_sessions`, then `managed_mcp_launch_leases`, and finally `agent_connections`; structured findings remain durable historical diagnostics. No path cascades into `projects`, `runtime_home`, `installation_profile`, or a project `state.sqlite` database.

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
  bounded_context_json TEXT NOT NULL DEFAULT '{}',
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
CREATE TABLE host_sessions (
  project_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  project_integration_revision TEXT NOT NULL CHECK (
    length(project_integration_revision) = 71
    AND substr(project_integration_revision, 1, 7) = 'sha256:'
    AND substr(project_integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  host_session_id TEXT NOT NULL CHECK (length(trim(host_session_id)) > 0),
  first_observed_at TEXT NOT NULL,
  last_observed_at TEXT NOT NULL,
  PRIMARY KEY (project_id, session_id),
  UNIQUE (project_id, session_id, connection_internal_id),
  CHECK (last_observed_at >= first_observed_at),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id)
);

CREATE TRIGGER host_sessions_project_integration_revision_immutable
BEFORE UPDATE OF project_integration_revision ON host_sessions
BEGIN
  SELECT RAISE(ABORT, 'host_sessions.project_integration_revision is immutable');
END;

CREATE TABLE host_turns (
  project_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  host_turn_id TEXT NOT NULL CHECK (length(trim(host_turn_id)) > 0),
  first_observed_at TEXT NOT NULL,
  last_observed_at TEXT NOT NULL,
  PRIMARY KEY (project_id, session_id, host_turn_id),
  UNIQUE (project_id, session_id, connection_internal_id, host_turn_id),
  CHECK (last_observed_at >= first_observed_at),
  FOREIGN KEY (project_id, session_id, connection_internal_id)
    REFERENCES host_sessions (project_id, session_id, connection_internal_id)
);

CREATE TABLE host_tool_invocations (
  project_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  host_turn_id TEXT NOT NULL,
  host_tool_use_id TEXT NOT NULL CHECK (length(trim(host_tool_use_id)) > 0),
  host_tool_name TEXT NOT NULL CHECK (length(trim(host_tool_name)) > 0),
  first_observed_at TEXT NOT NULL,
  last_observed_at TEXT NOT NULL,
  PRIMARY KEY (project_id, session_id, host_tool_use_id),
  UNIQUE (
    project_id, session_id, connection_internal_id, host_turn_id,
    host_tool_use_id, host_tool_name
  ),
  CHECK (last_observed_at >= first_observed_at),
  FOREIGN KEY (project_id, session_id, connection_internal_id, host_turn_id)
    REFERENCES host_turns (project_id, session_id, connection_internal_id, host_turn_id)
);

CREATE TABLE managed_mcp_sessions (
  project_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  runtime_session_id TEXT CHECK (
    runtime_session_id IS NULL OR length(trim(runtime_session_id)) > 0
  ),
  connection_internal_id TEXT NOT NULL,
  host_thread_id TEXT NOT NULL CHECK (length(trim(host_thread_id)) > 0),
  last_host_turn_id TEXT NOT NULL CHECK (length(trim(last_host_turn_id)) > 0),
  first_observed_at TEXT NOT NULL,
  last_observed_at TEXT NOT NULL,
  PRIMARY KEY (project_id, session_id),
  CHECK (last_observed_at >= first_observed_at),
  FOREIGN KEY (project_id, session_id, connection_internal_id)
    REFERENCES host_sessions (project_id, session_id, connection_internal_id),
  FOREIGN KEY (project_id, session_id, connection_internal_id, last_host_turn_id)
    REFERENCES host_turns (project_id, session_id, connection_internal_id, host_turn_id)
);

CREATE TABLE guard_events (
  project_id TEXT NOT NULL,
  guard_event_id TEXT NOT NULL,
  session_id TEXT,
  connection_internal_id TEXT NOT NULL,
  correlation_kind TEXT CHECK (
    correlation_kind IN ('codex_hook_prompt', 'codex_hook_tool')
  ),
  host_turn_id TEXT,
  host_tool_use_id TEXT,
  host_tool_name TEXT,
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
  UNIQUE (
    project_id, guard_event_id, session_id, connection_internal_id,
    host_turn_id, host_tool_use_id, host_tool_name
  ),
  CHECK (
    (
      correlation_kind IS NULL
      AND session_id IS NULL
      AND host_turn_id IS NULL
      AND host_tool_use_id IS NULL
      AND host_tool_name IS NULL
    )
    OR (
      correlation_kind = 'codex_hook_prompt'
      AND event_kind = 'prompt_capture'
      AND session_id IS NOT NULL
      AND host_turn_id IS NOT NULL
      AND host_tool_use_id IS NULL
      AND host_tool_name IS NULL
    )
    OR (
      correlation_kind = 'codex_hook_tool'
      AND event_kind IN ('pre_tool', 'post_tool')
      AND session_id IS NOT NULL
      AND host_turn_id IS NOT NULL
      AND host_tool_use_id IS NOT NULL
      AND host_tool_name IS NOT NULL
    )
  ),
  CHECK (contract_status != 'compatible' OR correlation_kind IS NOT NULL),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, session_id, connection_internal_id, host_turn_id)
    REFERENCES host_turns (project_id, session_id, connection_internal_id, host_turn_id),
  FOREIGN KEY (
    project_id, session_id, connection_internal_id, host_turn_id,
    host_tool_use_id, host_tool_name
  ) REFERENCES host_tool_invocations (
    project_id, session_id, connection_internal_id, host_turn_id,
    host_tool_use_id, host_tool_name
  )
);

CREATE TABLE prompt_captures (
  project_id TEXT NOT NULL,
  prompt_capture_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  host_turn_id TEXT NOT NULL,
  capture_kind TEXT NOT NULL,
  prompt_sha256 TEXT NOT NULL,
  prompt_text TEXT,
  captured_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, prompt_capture_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, session_id, connection_internal_id, host_turn_id)
    REFERENCES host_turns (project_id, session_id, connection_internal_id, host_turn_id)
);

CREATE TABLE unrecorded_changes (
  project_id TEXT NOT NULL,
  unrecorded_change_id TEXT NOT NULL,
  repository_observation_id TEXT NOT NULL,
  task_id TEXT,
  status TEXT NOT NULL CHECK (status IN ('unresolved', 'resolved')),
  summary TEXT NOT NULL CHECK (length(trim(summary)) > 0),
  observed_paths_json TEXT NOT NULL,
  unmatched_delta_digest TEXT NOT NULL CHECK (
    length(unmatched_delta_digest) = 71
    AND substr(unmatched_delta_digest, 1, 7) = 'sha256:'
    AND substr(unmatched_delta_digest, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  detection_json TEXT NOT NULL DEFAULT '{}',
  resolution_json TEXT,
  detected_at TEXT NOT NULL,
  resolved_at TEXT,
  resolved_by_actor_source TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, unrecorded_change_id),
  UNIQUE (project_id, repository_observation_id, unmatched_delta_digest),
  CHECK (observed_paths_json != '[]'),
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
  FOREIGN KEY (project_id, repository_observation_id)
    REFERENCES repository_observations (project_id, repository_observation_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id)
);

CREATE INDEX idx_unrecorded_changes_status
  ON unrecorded_changes (project_id, status, detected_at);
CREATE INDEX idx_unrecorded_changes_observation
  ON unrecorded_changes (project_id, repository_observation_id);
CREATE INDEX idx_unrecorded_changes_task
  ON unrecorded_changes (project_id, task_id, status);
CREATE TABLE expected_writes (
  project_id TEXT NOT NULL,
  expected_write_id TEXT NOT NULL,
  repository_observation_id TEXT NOT NULL,
  command_kind TEXT NOT NULL CHECK (length(trim(command_kind)) > 0),
  path_policy TEXT NOT NULL CHECK (path_policy IN ('exact_paths')),
  expected_paths_json TEXT NOT NULL CHECK (expected_paths_json != '[]'),
  task_id TEXT NOT NULL,
  change_unit_id TEXT NOT NULL,
  write_ticket_ids_json TEXT NOT NULL CHECK (write_ticket_ids_json != '[]'),
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version >= 0),
  status TEXT NOT NULL CHECK (status IN ('pending', 'matched')),
  matched_paths_json TEXT,
  created_at TEXT NOT NULL,
  matched_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, expected_write_id),
  UNIQUE (project_id, repository_observation_id),
  CHECK (
    (
      status = 'pending'
      AND matched_paths_json IS NULL
      AND matched_at IS NULL
    )
    OR (
      status = 'matched'
      AND matched_paths_json IS NOT NULL
      AND matched_paths_json != '[]'
      AND matched_at IS NOT NULL
    )
  ),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, repository_observation_id)
    REFERENCES repository_observations (project_id, repository_observation_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id)
);

CREATE INDEX idx_expected_writes_observation
  ON expected_writes (project_id, repository_observation_id, status);
CREATE INDEX idx_expected_writes_task
  ON expected_writes (project_id, task_id, status);
CREATE TABLE repository_observations (
  project_id TEXT NOT NULL,
  repository_observation_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  host_turn_id TEXT NOT NULL,
  host_tool_use_id TEXT NOT NULL,
  host_tool_name TEXT NOT NULL,
  guard_installation_id TEXT NOT NULL,
  observer_contract_digest TEXT NOT NULL CHECK (
    length(observer_contract_digest) = 71
    AND substr(observer_contract_digest, 1, 7) = 'sha256:'
    AND substr(observer_contract_digest, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  pre_tool_guard_event_id TEXT,
  post_tool_guard_event_id TEXT,
  state TEXT NOT NULL CHECK (state IN ('open', 'complete', 'unavailable')),
  pre_snapshot_json TEXT,
  pre_snapshot_digest TEXT CHECK (
    pre_snapshot_digest IS NULL
    OR (
      length(pre_snapshot_digest) = 71
      AND substr(pre_snapshot_digest, 1, 7) = 'sha256:'
      AND substr(pre_snapshot_digest, 8) NOT GLOB '*[^0-9a-f]*'
    )
  ),
  post_snapshot_json TEXT,
  post_snapshot_digest TEXT CHECK (
    post_snapshot_digest IS NULL
    OR (
      length(post_snapshot_digest) = 71
      AND substr(post_snapshot_digest, 1, 7) = 'sha256:'
      AND substr(post_snapshot_digest, 8) NOT GLOB '*[^0-9a-f]*'
    )
  ),
  delta_json TEXT,
  delta_digest TEXT CHECK (
    delta_digest IS NULL
    OR (
      length(delta_digest) = 71
      AND substr(delta_digest, 1, 7) = 'sha256:'
      AND substr(delta_digest, 8) NOT GLOB '*[^0-9a-f]*'
    )
  ),
  unavailable_reason TEXT CHECK (
    unavailable_reason IS NULL OR unavailable_reason IN (
      'invalid_observer_limits',
      'invalid_repository_root',
      'not_git_repository',
      'git_layout_unavailable',
      'git_command_unavailable',
      'git_command_failed',
      'process_timeout',
      'git_output_limit_exceeded',
      'process_input_limit_exceeded',
      'candidate_path_limit_exceeded',
      'total_hash_bytes_limit_exceeded',
      'file_size_limit_exceeded',
      'serialization_depth_limit_exceeded',
      'serialization_size_limit_exceeded',
      'invalid_relative_path',
      'non_utf8_path',
      'path_outside_repository',
      'inaccessible_path',
      'unsupported_path_state',
      'unstable_repository',
      'repository_identity_changed',
      'observer_contract_mismatch',
      'git_object_unavailable',
      'invocation_denied',
      'missing_open_observation'
    )
  ),
  started_at TEXT NOT NULL,
  completed_at TEXT,
  terminal_result_json TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, repository_observation_id),
  UNIQUE (
    project_id, session_id, connection_internal_id, host_turn_id,
    host_tool_use_id, host_tool_name
  ),
  CHECK (
    (
      state = 'open'
      AND (
        (
          pre_tool_guard_event_id IS NOT NULL
          AND pre_snapshot_json IS NOT NULL
          AND pre_snapshot_digest IS NOT NULL
        )
        OR (
          pre_tool_guard_event_id IS NULL
          AND pre_snapshot_json IS NULL
          AND pre_snapshot_digest IS NULL
        )
      )
      AND post_tool_guard_event_id IS NULL
      AND post_snapshot_json IS NULL
      AND post_snapshot_digest IS NULL
      AND delta_json IS NULL
      AND delta_digest IS NULL
      AND unavailable_reason IS NULL
      AND completed_at IS NULL
      AND terminal_result_json IS NULL
    )
    OR (
      state = 'complete'
      AND pre_tool_guard_event_id IS NOT NULL
      AND pre_snapshot_json IS NOT NULL
      AND pre_snapshot_digest IS NOT NULL
      AND post_tool_guard_event_id IS NOT NULL
      AND post_snapshot_json IS NOT NULL
      AND post_snapshot_digest IS NOT NULL
      AND delta_json IS NOT NULL
      AND delta_digest IS NOT NULL
      AND unavailable_reason IS NULL
      AND completed_at IS NOT NULL
      AND terminal_result_json IS NOT NULL
    )
    OR (
      state = 'unavailable'
      AND ((pre_snapshot_json IS NULL AND pre_snapshot_digest IS NULL)
        OR (pre_snapshot_json IS NOT NULL AND pre_snapshot_digest IS NOT NULL))
      AND (pre_snapshot_json IS NULL OR pre_tool_guard_event_id IS NOT NULL)
      AND post_snapshot_json IS NULL
      AND post_snapshot_digest IS NULL
      AND delta_json IS NULL
      AND delta_digest IS NULL
      AND unavailable_reason IS NOT NULL
      AND completed_at IS NOT NULL
      AND terminal_result_json IS NOT NULL
    )
  ),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (
    project_id, session_id, connection_internal_id, host_turn_id,
    host_tool_use_id, host_tool_name
  ) REFERENCES host_tool_invocations (
    project_id, session_id, connection_internal_id, host_turn_id,
    host_tool_use_id, host_tool_name
  ),
  FOREIGN KEY (
    project_id, pre_tool_guard_event_id, session_id, connection_internal_id,
    host_turn_id, host_tool_use_id, host_tool_name
  ) REFERENCES guard_events (
    project_id, guard_event_id, session_id, connection_internal_id,
    host_turn_id, host_tool_use_id, host_tool_name
  ),
  FOREIGN KEY (
    project_id, post_tool_guard_event_id, session_id, connection_internal_id,
    host_turn_id, host_tool_use_id, host_tool_name
  ) REFERENCES guard_events (
    project_id, guard_event_id, session_id, connection_internal_id,
    host_turn_id, host_tool_use_id, host_tool_name
  )
);

CREATE INDEX idx_host_sessions_connection
  ON host_sessions (project_id, connection_internal_id, last_observed_at);
CREATE INDEX idx_host_turns_session
  ON host_turns (project_id, session_id, last_observed_at);
CREATE INDEX idx_host_tool_invocations_session
  ON host_tool_invocations (project_id, session_id, last_observed_at);
CREATE INDEX idx_managed_mcp_sessions_runtime
  ON managed_mcp_sessions (project_id, runtime_session_id, last_observed_at);
CREATE UNIQUE INDEX idx_managed_mcp_sessions_runtime_binding
  ON managed_mcp_sessions (project_id, runtime_session_id)
  WHERE runtime_session_id IS NOT NULL;
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
CREATE INDEX idx_repository_observations_state
  ON repository_observations (project_id, state, started_at);
CREATE INDEX idx_repository_observations_connection
  ON repository_observations (project_id, connection_internal_id, state);
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
- Store reads that derive the latest managed MCP session or Guard event strict-parse and normalize the
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
- Store application validation strict-decodes request, basis, and resolution JSON, derives the adapter-neutral resolution form from the stored request, and requires matching closed tags and derived `action_kind`. It limits target and artifact candidates to 32 each, note-like text to 1,000 Unicode scalar values, observation summary to 4,000 Unicode scalar values, and canonical serialized forms to 32 KiB, rejecting rather than truncating excess.
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
- `host_sessions`, `host_turns`, and `host_tool_invocations` normalize project-local host correlation. Composite keys preserve project, Connection, session, turn, tool-use, and tool-name ownership. A tool-use ID cannot be rebound to another turn or tool name. `managed_mcp_sessions` alone stores a required `host_thread_id` and optional `runtime_session_id`; the partial unique index applies only after runtime attachment. A null runtime after Registry reservation failure or before final project attachment is not authorization.
- `guard_events` binds every observation to a required typed hook phase, Guard installation, exact policy hash, and integration revision. `correlation_kind=codex_hook_prompt` is valid only for `prompt_capture` with session and turn present and tool fields absent. `correlation_kind=codex_hook_tool` is valid only for `pre_tool` or `post_tool` with session, turn, tool-use ID, and tool name present. A compatible event must have one of those exact shapes. Hook rows have no thread column. Only current-owned `compatible` events satisfy a required phase; current `malformed` or `incompatible` events fail the Guard observation check, and older hashes or revisions do not satisfy it. `decision` is constrained to `allow`, `deny`, `warn`, or `inject_context`; these values record local host decision requests, not OS-level enforcement proof.
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
