# Storage

## What This Document Owns

This is reference documentation for a future local Harness Server. No database,
migration runner, server, or runtime exists in this repository yet. Current
repository phase and implementation handoff status are tracked in
[Implementation Overview](../build/implementation-overview.md#documentation-acceptance-status).

This page owns the MVP-1 persistence model, Runtime Home identity, SQLite schema
sketches, storage-owned JSON `TEXT` rules, enum hardening, artifact/evidence
reference storage, and the boundary between persisted records and derived views.
Use [Build: MVP-1 User Work Loop](../build/mvp-user-work-loop.md) and
[Engineering Checkpoint](../build/engineering-checkpoint.md) for stage order and exit
criteria.

## Read This When

- You need the smallest storage model needed for MVP-1.
- You are separating Core-owned state from chat, Markdown projections, connector
  output, tool output, and report text.
- You are checking which fields are needed to calculate close blockers.
- You are making sure later-profile tables do not become MVP-1 requirements.

## Related Owners

| Concern | Owner |
|---|---|
| Public MCP request/response shapes | [MVP API](api/mvp-api.md) and [API Schema Core](api/schema-core.md) |
| `ArtifactRef`, staged active ref kinds, idempotency, and state conflict behavior | [API Schema Core](api/schema-core.md#artifactref), [API Schema Core: Stage-Specific Active Value Sets](api/schema-core.md#stage-specific-active-value-sets), and [API Errors](api/errors.md) |
| Task lifecycle, gates, `prepare_write`, `record_run`, `close_task`, stable events | [Core Model Reference](core-model.md) |
| Core process model, transaction order, locks, projection/reconcile placement | [Runtime Architecture Reference](runtime-architecture.md) |
| Projection authority, freshness, managed blocks, rendered templates | [Projection And Templates Reference](projection-and-templates.md) and [Template Reference](templates/README.md) |
| Operator behavior, doctor/recover/export/reconcile/conformance entrypoints | [Operations And Conformance Reference](operations-and-conformance.md) |
| Fixture format and assertion semantics | [Conformance Fixtures Reference](conformance-fixtures.md) |
| Stage sequence and implementation readiness | [Build: MVP-1 User Work Loop](../build/mvp-user-work-loop.md), [Implementation Overview](../build/implementation-overview.md) |

## MVP-1 Storage Goal

MVP-1 storage keeps the smallest local authority record that lets a user and an
agent understand current work without trusting chat memory or generated
Markdown. It stores project identity, one or more tracked Tasks, task scope,
user-owned judgments, cooperative write-check results, Runs, evidence pointers,
and blockers.

MVP-1 storage is not a Journey system, report system, projection job system,
conformance runner, QA database, Eval store, export pipeline, or dashboard data
model. Those records may be useful later, but they are outside MVP-1 unless an
owner profile explicitly promotes them.

MVP-1 storage must preserve these authority boundaries:

- Core-owned state rows are the source of truth for current Harness state.
- `task_events`, when retained, is an audit and ordering trail. It is not the
  normal source used to reconstruct current state.
- Evidence pointers are not evidence authority until Core records them and links
  them to a compatible owner record.
- Chat, Markdown projections, generated reports, connector manifests, tool
  output, and operator output are not authority unless a Core mutation records an
  owner-valid state row or evidence ref.
- Status cards, task summaries, close readiness, evidence summaries, next
  actions, and projection freshness are derived status/views. They can be stale,
  failed, absent, or recomputed without changing the persisted state records.
- Future/profile tables become required only when the owning profile or tool path
  is active or used.

## Runtime Home Identity And Risks

Harness keeps one local Runtime Home and one state database per registered
project. The default reference location is `~/.harness`, but the implementation
may choose a configured equivalent.

Reference layout:

```text
~/.harness/
  registry.sqlite
  projects/
    PRJ-0001/
      project.yaml
      state.sqlite
      evidence/
        tmp/
        diffs/
        logs/
        screenshots/
        checkpoints/
```

`registry.sqlite` stores Runtime Home identity and project registration.
`project.yaml` stores static project configuration only. `state.sqlite` stores
project-local Core state. Evidence directories store registered files or
pointers after Core applies the evidence registration boundary.

`project.yaml` must not store current Task state, current gates, write authority,
evidence sufficiency, work acceptance, or residual-risk acceptance.

```yaml
project_id: PRJ-0001
display_name: my-app
repo_root: /abs/path/to/my-app
default_agent_surface: reference

default_checks:
  lint: []
  test: []
  build: []
```

Runtime Home identity should not depend only on a path. A copied or moved Runtime
Home may keep the same stored `runtime_home_id`; a new Runtime Home should get a
new id. `doctor` and recovery flows can use that identity to report suspicious
copies, duplicate registrations, or path drift, but the id does not provide
tamper-proofing.

Runtime Home contains local operational authority and sensitive support data.
Broad write access is a tampering and evidence-poisoning risk. Broad read access
can expose secrets, PII, tokens, logs, screenshots, diffs, verification bundles,
and exports.

Engineering Checkpoint and MVP-1 storage are cooperative/detective unless a
later profile proves stronger controls. File permissions, owner checks, hashes,
and `doctor` findings are defense in depth; they do not create OS-level
sandboxing, arbitrary-tool control, tamper-proof storage, or pre-execution
blocking by themselves.

| Observation | Storage meaning |
|---|---|
| Runtime Home or project storage owner/mode cannot be determined. | Report unknown or weak local file posture. Do not claim an OS-level guarantee. |
| Runtime Home, `state.sqlite`, `registry.sqlite`, or evidence directories are writable by unrelated users, shared groups, shared containers, or broad local processes. | Report tampering and evidence-poisoning risk. Core must still validate rows, owner links, hashes, and evidence registration before trusting meaning. |
| Evidence storage or exports are readable by unrelated users, shared groups, shared containers, or broad local processes. | Report confidentiality risk without echoing sensitive values. |
| A registered evidence hash, size, owner link, or path no longer matches storage metadata. | Treat as evidence integrity failure or recovery input, not as projection drift. |

## Core Records

MVP-1 has a small set of persisted records. A future implementation may choose a
slightly different physical table layout, but it must not turn later-profile
records into MVP-1 requirements.

| Record | Minimal persisted purpose | Notes |
|---|---|---|
| `project` | Local project identity, Runtime Home registration, state database location, active Task pointer. | Stored across `registry_meta`, `projects`, and `project_state` in the sketch below. |
| `task` | Tracked work item: user request, current summary, lifecycle, result, active scope, and state clock. | A Task is the user-value unit. It is not a report, Journey, or projection. |
| `task_scope` / `change_unit` | Current scope, non-goals, success criteria, allowed paths, denied paths, and scoped-write status. | Existing Core/API names use `Change Unit` and `record_kind=change_unit`; MVP-1 storage only needs a single active task-scope row or equivalent Task scope fields, not a DAG. |
| `user_judgment` | User-owned product/UX choice, technical choice, sensitive-action approval, work acceptance, and residual-risk acceptance. | Full-format Decision Packet is presentation, not a separate authority table. Committed `approvals` are later-profile. |
| `write_check` / `write_authorization` | Cooperative `prepare_write` result for the exact proposed write. Allowed results create a single-use Write Authorization; blocked results create blockers. | This is Harness authority for a Core path, not OS-level permission or arbitrary-tool prevention. |
| `run` | Agent work run or observed execution result, linked to Task, scope, optional Write Authorization, and evidence refs. | A Run can support evidence only through registered refs. It does not prove verification, QA, acceptance, or close by itself. |
| `evidence_ref` | Pointer and short summary for evidence such as a diff, log, screenshot, checkpoint, or existing artifact ref. | MVP-1 does not need a detailed Evidence Manifest. Large bytes remain referenced, not embedded in state. |
| `blocker` | Close blocker or next-action blocker with owner refs and the smallest required next action. | Close readiness is derived from open blockers and owner records; it does not require a separate `close_readiness` table in MVP-1. |

Support rows such as `tool_invocations` and `task_events` help replay,
idempotency, audit, and ordering. They are not user-facing domain records and do
not expand the MVP-1 product surface.

## Persisted State Vs Derived Status/View

Persisted MVP-1 state is the set of rows Core commits. Derived status/view is
what Core or a renderer computes from those rows for users, agents, or operators.

| Derived output | Source records | MVP-1 storage rule |
|---|---|---|
| Status card / task summary | `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `evidence_refs`, `blockers` | Derived view. It may be recomputed on read; no `projection_status_cards` or `projection_jobs` table is required. |
| Next safe actions | Open blockers, pending user judgments, write-check state, evidence refs, Task lifecycle | Derived view. It does not create a Task, judgment, Run, evidence, or Write Authorization. |
| Evidence summary | `runs` and `evidence_refs` | Derived summary. MVP-1 stores refs and short summaries, not a full `evidence_summaries` or `evidence_manifests` authority table. |
| Close readiness | Task lifecycle, scope state, pending user judgments, evidence refs, open blockers, work-acceptance and residual-risk user judgments | Derived check. MVP-1 stores the blockers and owner records used by the check, not a separate `close_readiness` source of truth. |
| Projection freshness | Current state version compared with a returned/read view source version | Derived diagnostic. Full `projection_jobs` storage is Operations Profile or profile-promoted. |

## Minimal DDL Or Schema Sketch

The DDL below is a reference sketch for planning. It is not proof that a migration
runner exists. It keeps MVP-1 focused on the minimal records above.

<a id="core-authority-smoke-schema"></a>
<a id="mvp-1-minimal-storage-schema"></a>

### `registry.sqlite`

```sql
CREATE TABLE registry_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE projects (
  project_id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  repo_root TEXT NOT NULL,
  project_dir TEXT NOT NULL,
  config_path TEXT NOT NULL,
  registered_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

Required `registry_meta` keys for MVP-1 are `runtime_home_id` and
`schema_version`.

### `state.sqlite`

```sql
CREATE TABLE project_state (
  project_id TEXT PRIMARY KEY,
  schema_version INTEGER NOT NULL,
  state_version INTEGER NOT NULL DEFAULT 0,
  active_task_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE tasks (
  task_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  title TEXT NOT NULL,
  user_request TEXT NOT NULL,
  mode TEXT NOT NULL,
  lifecycle_phase TEXT NOT NULL,
  result TEXT NOT NULL DEFAULT 'none',
  summary TEXT NOT NULL DEFAULT '',
  active_change_unit_id TEXT,
  state_version INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  closed_at TEXT
);

CREATE TABLE change_units (
  change_unit_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  scope_summary TEXT NOT NULL,
  non_goals_json TEXT NOT NULL DEFAULT '[]',
  success_criteria_json TEXT NOT NULL DEFAULT '[]',
  allowed_paths_json TEXT NOT NULL DEFAULT '[]',
  denied_paths_json TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE user_judgments (
  user_judgment_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT REFERENCES change_units(change_unit_id),
  judgment_type TEXT NOT NULL,
  presentation TEXT NOT NULL,
  display_label TEXT NOT NULL,
  status TEXT NOT NULL,
  question TEXT NOT NULL,
  options_json TEXT NOT NULL DEFAULT '[]',
  selected_option_json TEXT,
  judgment_payload_json TEXT NOT NULL DEFAULT '{}',
  affected_scope_json TEXT NOT NULL DEFAULT '{}',
  affected_gates_json TEXT NOT NULL DEFAULT '[]',
  context_refs_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  expires_at TEXT,
  resolved_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE write_authorizations (
  write_authorization_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT NOT NULL REFERENCES change_units(change_unit_id),
  status TEXT NOT NULL,
  decision TEXT NOT NULL,
  basis_state_version INTEGER NOT NULL,
  intended_operation TEXT NOT NULL,
  allowed_paths_json TEXT NOT NULL DEFAULT '[]',
  denied_paths_json TEXT NOT NULL DEFAULT '[]',
  related_user_judgment_refs_json TEXT NOT NULL DEFAULT '[]',
  blocker_refs_json TEXT NOT NULL DEFAULT '[]',
  consumed_by_run_id TEXT,
  expires_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE runs (
  run_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT REFERENCES change_units(change_unit_id),
  write_authorization_id TEXT REFERENCES write_authorizations(write_authorization_id),
  kind TEXT NOT NULL,
  status TEXT NOT NULL,
  summary TEXT NOT NULL,
  observed_changes_json TEXT NOT NULL DEFAULT '[]',
  command_results_json TEXT NOT NULL DEFAULT '[]',
  evidence_ref_ids_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL
);

CREATE TABLE evidence_refs (
  evidence_ref_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  run_id TEXT REFERENCES runs(run_id),
  owner_record_kind TEXT NOT NULL,
  owner_record_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  uri TEXT NOT NULL,
  summary TEXT NOT NULL,
  sha256 TEXT,
  size_bytes INTEGER,
  content_type TEXT,
  redaction_state TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE blockers (
  blocker_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  blocked_action TEXT NOT NULL,
  blocker_kind TEXT NOT NULL,
  status TEXT NOT NULL,
  message TEXT NOT NULL,
  owner_ref_json TEXT NOT NULL DEFAULT '{}',
  related_refs_json TEXT NOT NULL DEFAULT '[]',
  required_next_action TEXT NOT NULL,
  created_at TEXT NOT NULL,
  resolved_at TEXT
);

CREATE TABLE task_events (
  event_id TEXT PRIMARY KEY,
  task_id TEXT REFERENCES tasks(task_id),
  event_seq INTEGER NOT NULL,
  event_type TEXT NOT NULL,
  state_version INTEGER NOT NULL,
  actor TEXT NOT NULL,
  payload_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  UNIQUE(event_seq)
);

CREATE TABLE tool_invocations (
  invocation_id TEXT PRIMARY KEY,
  idempotency_key TEXT NOT NULL,
  request_hash TEXT NOT NULL,
  tool_name TEXT NOT NULL,
  task_id TEXT REFERENCES tasks(task_id),
  basis_state_version INTEGER NOT NULL,
  response_json TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE(tool_name, idempotency_key)
);
```

Recommended MVP-1 indexes:

```sql
CREATE INDEX idx_tasks_project_phase ON tasks(project_id, lifecycle_phase);
CREATE INDEX idx_change_units_task_status ON change_units(task_id, status);
CREATE INDEX idx_user_judgments_task_status ON user_judgments(task_id, status);
CREATE INDEX idx_write_authorizations_task_status ON write_authorizations(task_id, status);
CREATE UNIQUE INDEX uq_runs_write_authorization_consumed
  ON runs(write_authorization_id)
  WHERE write_authorization_id IS NOT NULL;
CREATE INDEX idx_evidence_refs_owner ON evidence_refs(owner_record_kind, owner_record_id);
CREATE INDEX idx_blockers_task_status ON blockers(task_id, status);
CREATE INDEX idx_task_events_task_seq ON task_events(task_id, event_seq);
```

If an implementation keeps separate `artifacts` and `artifact_links` tables in
MVP-1, those tables are only a physical representation of `evidence_ref` storage.
They do not create the full artifact-integrity, export, projection-linking, or
Evidence Manifest profiles.

## Fields Needed For Close-Blocker Calculation

MVP-1 close-blocker calculation reads current persisted records and derives the
close result. It does not need Journey, Spine, detailed Evidence Manifest, Eval,
Manual QA, export/report tables, projection jobs, or validator-run storage.

| Blocker or close fact | Minimum source fields |
|---|---|
| Active Task exists and is closeable | `project_state.active_task_id`, `tasks.lifecycle_phase`, `tasks.result`, `tasks.closed_at` |
| Scope is present and current | `tasks.active_change_unit_id`, `change_units.status`, `change_units.scope_summary`, `change_units.non_goals_json`, `change_units.success_criteria_json` |
| User-owned judgment is unresolved | `user_judgments.judgment_type`, `user_judgments.status`, `user_judgments.affected_gates_json`, `user_judgments.context_refs_json` |
| Sensitive-action permission is missing or denied | `user_judgments` rows with `judgment_type=sensitive_action_approval`, plus current `write_authorizations.related_user_judgment_refs_json` when a write is involved |
| Write authority is missing, stale, or already consumed | `write_authorizations.status`, `write_authorizations.basis_state_version`, `write_authorizations.consumed_by_run_id`, current `tasks.state_version` |
| Run or evidence support is missing | `runs.status`, `runs.evidence_ref_ids_json`, `evidence_refs.status`, `evidence_refs.owner_record_kind`, `evidence_refs.owner_record_id` |
| Work acceptance is required but missing | `user_judgments` rows with `judgment_type=work_acceptance` and compatible `status` / `selected_option_json` |
| Residual risk is not visible or not accepted | `blockers` rows with residual-risk blocker kinds, plus `user_judgments` rows with `judgment_type=residual_risk_acceptance` when acceptance is required |
| A blocker is still open | `blockers.status`, `blockers.blocker_kind`, `blockers.blocked_action`, `blockers.related_refs_json`, `blockers.required_next_action` |
| Readable status is stale | Current `tasks.state_version` compared with the source version returned by the read/card response; later `projection_jobs` only when Operations Profile is active |

The close response may expose a compact close-readiness summary, evidence
summary, and next action. Those are derived outputs. Persisting a
`close_readiness`, `evidence_summary`, or status-card cache is optional/later
unless an owner profile promotes it.

## Later-Profile Storage

Later-profile storage is useful design inventory. It must not be read as an
MVP-1 DDL bundle.

### Assurance Profile

Assurance Profile or profile-promoted storage may add:

| Candidate table | Why it may matter later | Not required for MVP-1 |
|---|---|---|
| `approvals` | Committed sensitive-action approval lifecycle and drift handling | MVP-1 sensitive-action approval user judgments |
| `baselines` | Repository baseline capture for assurance, approval, and verification freshness | MVP-1 write checks unless a promoted profile needs baseline checks |
| `residual_risks` | Rich residual-risk lifecycle separate from blocker rows | MVP-1 residual-risk visibility or acceptance prompts |
| `evidence_manifests` | Full criteria-to-evidence coverage | MVP-1 evidence refs and evidence summaries derived from refs |
| `evals` | Detached verification or evaluator review | MVP-1 status, close blockers, or self-checked evidence |
| `manual_qa_records` | Manual QA result, findings, setup, evidence refs | MVP-1 unless profile or user request requires Manual QA support |
| `validator_runs` | Persisted `ValidatorResult` rows | MVP-1 blockers unless a narrow owner explicitly promotes validator storage |
| `feedback_loops` | Feedback-loop policy support | MVP-1 unless a profile is selected |
| `tdd_traces` | Red/green/refactor evidence when TDD profile is selected | MVP-1 |

### Operations Profile

Operations Profile or profile-promoted storage may add:

| Candidate table | Why it may matter later | Not required for MVP-1 |
|---|---|---|
| `projection_jobs` | Durable outbox for rendered Markdown or managed outputs | MVP-1 status cards or next-action summaries |
| `reconcile_items` | Route human edits or projection drift into Core decisions | MVP-1 |
| `connector_manifests` | Track connector-managed files and drift | MVP-1 |
| `persistent_locks` | Durable lock/recovery metadata if needed beyond process locks | MVP-1 |
| `export_manifests` | Release handoff or export package metadata | MVP-1 |
| `recover_items` | Recovery findings, repair plan, and operator follow-up | MVP-1 |

### Future Or Diagnostic

Future or diagnostic candidates stay outside MVP-1 until an owner promotes them:

- Journey/spine continuity: `task_spine_entries`, `journey_cards`
- Domain and stewardship: `domain_terms`, `module_map_items`,
  `interface_contracts`
- Rich design support: `shared_designs`, `change_unit_dependencies`
- Diagnostics and polish: metrics, dashboards, context indexes, connector
  analytics, richer projection caches, export/recover detail tables

## Removed Or Relocated Future Records

These records are deliberately outside the MVP-1 storage path:

| Record family | MVP-1 replacement | Later location |
|---|---|---|
| Journey, Journey Card, Journey Spine, Spine entries | MVP-1 compact views derived from Task, scope, judgments, evidence refs, and blockers | Future/diagnostic projections or owner-promoted continuity support |
| Detailed Evidence Manifest | `evidence_refs` plus derived evidence summary | Assurance Profile full evidence coverage |
| Eval / detached verification records | Run/evidence refs and self-check labels when applicable | Assurance Profile verification |
| Manual QA detailed records | User judgment, blocker, or profile-specific prompt when QA is required but not implemented | Assurance Profile Manual QA |
| Export, report, and bundle tables | Evidence refs and optional artifact pointers only | Operations/export profile |
| Projection job tables | Ephemeral or read-time compact-view freshness | Operations Profile projection rendering |
| Future validation/conformance tables | Direct blockers and owner-field validation | Assurance/conformance profiles after executable runtime suites exist |
| Committed Approval table | `user_judgment` with `judgment_type=sensitive_action_approval` | Approval/Assurance Profile |
| Separate `task_intake` table | `tasks.user_request`, `tasks.summary`, and `change_units` scope fields | Later intake workflow profile if needed |
| Separate `evidence_summaries` / `close_readiness` / `projection_status_cards` tables | Derived status/view from `runs`, `evidence_refs`, `blockers`, and current state version | Optional profile-promoted caches only |

## Event Semantics

`task_events` is an append-only audit trail and event-order support table when
the implementation retains it. It records what Core committed and in what order.
It is not the normal authority source for current state, and MVP-1 state should
not be reconstructed by replaying events during ordinary operation.

Current state tables are authoritative:

- `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`,
  `evidence_refs`, and `blockers` are MVP-1 current records.
- Events support audit, debugging, idempotency explanation, projection freshness,
  and recovery history.

Required event emission applies only to committed state mutations. Malformed
requests, dry runs, pre-commit state conflicts, and invalid requests that do not
mutate state do not need `task_events` rows. If a blocked request creates or
updates a stored blocker, that blocker mutation is the event-worthy state change.

## Migration And Validation Notes

No migration runner exists in this repository. The notes below describe
constraints a future implementation must satisfy when it chooses a migration
mechanism.

### Storage Hardening As An Authority Boundary

SQLite can store malformed rows unless Core and migrations prevent them. A row
is authoritative only when it matches the owner schema, owner value set,
state-version basis, idempotency key, and evidence owner-link contract.

`doctor`, `recover`, evidence checks, and conformance runners should report
malformed JSON, unknown owner-bound values, mismatched replay rows, stale
state-version claims, evidence hash mismatch, and invalid owner links as storage
integrity findings, not projection drift.

### JSON `TEXT` Validation

JSON `TEXT` columns are storage flexibility, not permission to store arbitrary
JSON. Before Core commits a JSON `TEXT` value, it must parse the value and
validate the parsed shape against the owner:

- API-shaped payloads validate against [MVP API](api/mvp-api.md) and
  [API Schema Core](api/schema-core.md).
- Storage-only JSON validates against this page or the owner document named by
  this page.
- SQLite defaults such as `'{}'` and `'[]'` are storage representation rules;
  they do not make public API fields optional.

Malformed JSON and schema-incompatible JSON are invalid state. If a SQLite build
supports JSON checks, migrations may add `CHECK (json_valid(column_name))` as
defense in depth, but Core shape validation before commit still owns meaning.

### Canonical Enum Hardening

Status-like `TEXT` columns are not open strings. Core validation owns allowed
values; database `CHECK` constraints or lookup tables are defense in depth.

Early hardening should cover:

| Field(s) | Owner/value source |
|---|---|
| `tasks.mode`, `tasks.lifecycle_phase`, `tasks.result` | [Core Model Reference](core-model.md) |
| `change_units.status` | Core Model / Change Unit owner rules |
| `user_judgments.status`, `judgment_type`, `presentation` | user-judgment API/kernel owners |
| `write_authorizations.status`, `write_authorizations.decision` | [Core Model `prepare_write`](core-model.md#prepare_write) and [`harness.prepare_write`](api/mvp-api.md#harnessprepare_write) |
| `runs.kind`, `runs.status` | [`harness.record_run`](api/mvp-api.md#harnessrecord_run) and storage compatibility notes |
| `evidence_refs.kind`, `evidence_refs.redaction_state`, `evidence_refs.status` | `ArtifactRef`/evidence owners and storage compatibility notes |
| `blockers.status`, `blocked_action`, `blocker_kind` | Core Model and API blocker owners |
| `tool_invocations.status` | storage idempotency replay semantics |
| Future `projection_jobs.status`, `projection_jobs.projection_kind` | Projection/API owners when Operations Profile is active |
| Future `validator_runs.status` | `ValidatorResult` owner when assurance profile is active |
| Future `approvals.status` | Approval lifecycle owner when approval profile is active |
| Future `evidence_manifests.status` | Evidence profile owner when full Evidence Manifest profile is active |

Unknown owner-bound values are invalid state unless a fixture explicitly
exercises invalid-state recovery. Migrations must stop before tightening if
unknown values are present; they must not silently map unknown values to fallback
values that no owner defines.

Storage-owned compatibility values promoted here:

| Field | Durable values | Meaning |
|---|---|---|
| `runs.status` | `completed`, `interrupted`, `blocked`, `violation` | A committed Run row. Only `completed` can support evidence through normal owner refs. Other values are audit/recovery records and do not satisfy evidence, QA, verification, acceptance, or close readiness by themselves. |
| `change_units.status` | `planned`, `active`, `completed`, `deferred`, `superseded` | Scope lifecycle. Only the active compatible scope row scopes new writes. |
| `user_judgments.status` | `proposed`, `pending_user`, `resolved`, `deferred`, `rejected`, `blocked`, `superseded` | User judgment lifecycle. A resolved judgment affects only the judgment type and payload it records. |
| `write_authorizations.status` | `allowed`, `consumed`, `expired`, `stale`, `revoked` | Durable authorization lifecycle, matching the Core/API owner value set. Only `allowed` and compatible rows can be consumed by `record_run`. |
| `write_authorizations.decision` | `allowed`, `blocked`, `approval_required`, `decision_required`, `state_conflict` | Cooperative `prepare_write` decision. It does not imply OS-level authority. |
| `evidence_refs.status` | `available`, `missing`, `stale`, `blocked` | Evidence pointer availability. It is a pointer/status fact, not full evidence sufficiency. |
| `blockers.status` | `open`, `resolved`, `superseded` | Stored blocker lifecycle. Open blockers remain visible until Core resolves or supersedes them. |
| `tool_invocations.status` | `committed` | A row exists only for a committed replayable response. |

Future table value sets should be used only when the table's owner profile is
active, a fixture explicitly seeds that optional table, or the owner document
explicitly promotes the values.

### Migrations

Future migrations should:

- Record schema/profile version in `registry_meta` and `project_state` or an
  equivalent chosen metadata mechanism.
- Validate JSON and owner-bound status values before tightening constraints.
- Preserve `task_events.event_seq` order when `task_events` is retained.
- Preserve evidence hashes and owner links, or mark affected refs invalid for
  recovery.
- Stop on unknown owner-bound enum/status values instead of inventing fallback
  meanings.
- Treat status cards, projection/card/job freshness, evidence summaries, and
  close readiness as derived state, not canonical state.

These notes do not require a specific migration runner, migration file format, or
CLI command in MVP-1.

### Lock Policy

Runtime mutations should serialize through the Core transaction order owned by
[Runtime Architecture](runtime-architecture.md#state-transaction-flow). MVP-1 can
use ordinary SQLite transactions plus a process/project lock if needed.
`persistent_locks` is a later Operations candidate, not an MVP-1 table.

Locks protect concurrent writes; they do not provide OS sandboxing, evidence
integrity, or tamper-proof storage.
