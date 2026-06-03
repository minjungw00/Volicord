# Storage And DDL

## What This Document Owns

This is reference documentation for a future local Harness Server. No database, migration runner, server, or runtime exists in this repository yet. Current repository phase and implementation handoff status are tracked in [Implementation Overview](../build/implementation-overview.md#documentation-acceptance-status).

Use this page to review storage authority, Runtime Home identity, staged SQLite table needs, event semantics, artifact registration, and migration/validation constraints. Use [Build: MVP Plan](../build/mvp-plan.md) and [First Runnable Slice](../build/first-runnable-slice.md) for stage order and exit criteria.

## Read This When

- You need to know which storage tables are required for v0.1 or v0.2.
- You are separating Core-owned state from chat, Markdown projections, connector output, and tool output.
- You are checking Runtime Home risks, artifact poisoning controls, event/audit behavior, JSON validation, enum hardening, or future schema candidates.
- You are making sure later profile tables do not inflate the first server batch.

## Related Owners

| Concern | Owner |
|---|---|
| Public MCP request/response shapes, `ArtifactRef`, `ValidatorResult`, idempotency and state conflict behavior | [MCP API And Schemas](mcp-api-and-schemas.md) |
| Task lifecycle, gates, `prepare_write`, `record_run`, `close_task`, stable events | [Kernel Reference](kernel.md) |
| Core process model, transaction order, locks, projection/reconcile placement | [Runtime Architecture Reference](runtime-architecture.md) |
| Projection authority, freshness, managed blocks, rendered templates | [Document Projection Reference](document-projection.md) and [Template Reference](templates/README.md) |
| Operator behavior, doctor/recover/export/reconcile/conformance entrypoints | [Operations And Conformance Reference](operations-and-conformance.md) |
| Fixture format and assertion semantics | [Conformance Fixtures Reference](conformance-fixtures.md) |
| Stage sequence and implementation readiness | [Build: MVP Plan](../build/mvp-plan.md), [Implementation Overview](../build/implementation-overview.md) |

## Storage Role And Authority Model

Harness storage keeps local Core-owned operational state. It records scope, write authorization, user-owned judgments, evidence references, close readiness, acceptance, and residual risk as durable records when the active stage needs them. Storage does not make every useful future feature mandatory in the first implementation slice.

Authority boundaries:

- Core-owned state tables are the authority for current Harness state.
- `task_events` is an append-only audit and ordering trail, not the normal source used to reconstruct current state.
- Artifact files are not evidence authority until Core registers them and links them to a compatible owner record.
- Chat, Markdown projections, generated reports, connector manifests, tool output, and operator output are not authority unless a Core mutation records an owner-valid state row or artifact link.
- Projections and status cards are readable derived views. They can be stale, failed, or absent without changing canonical state.
- Future/profile tables become required only when the owning profile or tool path is active or used.

The first server batch should prove a narrow local authority loop: project identity, one Task, one scoped boundary, `prepare_write`, one single-use Write Authorization, one Run, one artifact/evidence reference, task events, and structured blockers. It should not build dozens of tables just because later profile contracts are documented.

## Runtime Home Identity And Risks

Harness keeps one local Runtime Home and one state database per registered project. The default reference location is `~/.harness`, but the implementation may choose a configured equivalent.

### Runtime home layout

Reference layout:

```text
~/.harness/
  registry.sqlite
  projects/
    PRJ-0001/
      project.yaml
      state.sqlite
      artifacts/
        bundles/
        diffs/
        logs/
        screenshots/
        checkpoints/
        manifests/
        qa/
        tdd/
        designs/
        prototypes/
        architecture/
        decisions/
        exports/
        tmp/
```

`registry.sqlite` stores Runtime Home identity and project registration. `project.yaml` stores static project configuration only. `state.sqlite` stores project-local Core state. Artifact directories store registered files after Core applies the artifact registration boundary.

Runtime Home identity should not depend only on a path. A copied or moved Runtime Home may keep the same stored `runtime_home_id`; a new Runtime Home should get a new id. `doctor` and recovery flows can use that identity to report suspicious copies, duplicate registrations, or path drift, but the id does not provide tamper-proofing.

### `project.yaml`

`project.yaml` is static project configuration. It must not store current Task state, current gates, write authority, evidence sufficiency, acceptance, or residual risk.

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

### Runtime home permissions and tampering

Runtime Home contains local operational authority and sensitive support data. Broad write access is a tampering and artifact-poisoning risk. Broad read access can expose secrets, PII, tokens, logs, screenshots, diffs, verification bundles, and exports.

v0.1 and v0.2 storage are cooperative/detective unless a later profile proves stronger controls. File permissions, owner checks, hashes, and `doctor` findings are defense in depth; they do not create OS-level sandboxing, arbitrary-tool control, tamper-proof storage, or pre-execution blocking by themselves.

| Observation | Storage meaning |
|---|---|
| Runtime Home or project storage owner/mode cannot be determined. | Report unknown or weak local file posture. Do not claim an OS-level guarantee. |
| Runtime Home, `state.sqlite`, `registry.sqlite`, or artifact directories are writable by unrelated users, shared groups, shared containers, or broad local processes. | Report tampering and artifact-poisoning risk. Core must still validate rows, owner links, hashes, and artifact registration before trusting meaning. |
| Artifact storage or exports are readable by unrelated users, shared groups, shared containers, or broad local processes. | Report confidentiality risk without echoing sensitive values. |
| A registered artifact hash, size, owner link, or path no longer matches storage metadata. | Treat as artifact integrity failure or recovery input, not as projection drift. |

## Table-To-Stage Matrix

This matrix is the main table list. It separates small v0.1/v0.2 storage from later profile candidates.

Public API refs are owned by [MCP API And Schemas](mcp-api-and-schemas.md#artifactref). For the minimum v0.2 storage slice, `evidence_summaries.evidence_summary_id` is addressable as `StateRecordRef.record_kind=evidence_summary`, and `close_readiness.close_readiness_id` is addressable as `StateRecordRef.record_kind=close_readiness`. Approval-shaped sensitive-action permission is addressable through `StateRecordRef.record_kind=decision_packet`; `StateRecordRef.record_kind=approval` remains later-profile unless the `approvals` table is explicitly promoted. `change_unit_dependencies` remains future/diagnostic storage, so `record_kind=change_unit_dependency` is not a v0.2 active public ref.

| Table | Purpose | First active stage | Authority or auxiliary | User-facing or internal | Later status |
|---|---|---|---|---|---|
| `registry_meta` | Runtime Home id and registry schema version | v0.1 | auxiliary identity | internal | active early |
| `projects` | Registered project identity and state location | v0.1 | authority for registration | user-facing via project selection | active early |
| `project_surfaces` | Surface/capability declaration and guarantee display when surface profiles are installed | v0.3/v0.4 or profile-promoted | auxiliary capability state | internal/user-facing diagnostics | future/later |
| `project_state` | Project-local clock and active Task pointer | v0.1 | authority | internal | active early |
| `tasks` | Current Task record and task state clock | v0.1 | authority | user-facing summary | active early |
| `change_units` | Minimal scoped work boundary for writes | v0.1 | authority | user-facing when scope is explained | active early |
| `write_authorizations` | Durable single-use `prepare_write` allow record | v0.1 | authority | internal with user-visible blockers | active early |
| `runs` | Committed observed Run record | v0.1 | authority | user-facing evidence/status refs | active early |
| `artifacts` | Registered artifact/evidence file metadata | v0.1 | artifact metadata authority | internal, surfaced by refs | active early |
| `artifact_links` | Compatible link from artifact to Task/Run/owner record | v0.1 | artifact owner-link authority | internal | active early |
| `task_blockers` | Structured status/blocker rows | v0.1 | authority for stored blockers | user-facing | active early |
| `task_events` | Append-only audit and event-order trail | v0.1 | audit trail and projection support | mostly internal | active early |
| `tool_invocations` | Committed idempotency replay row | v0.1 | replay support | internal | active early |
| `task_intake` | Ordinary-language intake and tracked clarification state | v0.2 | auxiliary shaping state | user-facing | not v0.1 |
| `decision_packets` | Simplified user judgment records and recorded answers | v0.2 | authority for user judgments | user-facing | not v0.1 |
| `decision_requests` | Optional prompt routing, replay, or handoff metadata linked to Decision Packets | v0.2 optional | auxiliary routing state | internal/user-facing prompt support | optional, not authority by itself |
| `residual_risks` | Minimal visible residual-risk rows | v0.2 | authority for stored residual risks | user-facing | not v0.1 |
| `evidence_summaries` | Minimal evidence summary over artifact/run refs | v0.2 | auxiliary summary over authority refs | user-facing | not v0.1 |
| `close_readiness` | Minimal close readiness and close-blocker snapshot | v0.2 | auxiliary display/check snapshot | user-facing | not v0.1 |
| `projection_status_cards` | Optional freshness/status card state without a projection job system | v0.2 optional | auxiliary derived display state | user-facing | optional, not authority |
| `approvals` | Sensitive-action approval lifecycle | v0.3 or profile-promoted | authority when profile is active | user-facing | future/later |
| `baselines` | Repository baseline capture | v0.3 or profile-promoted | auxiliary support for assurance | internal | future/later |
| `evidence_manifests` | Full criteria-to-evidence coverage | v0.3 or profile-promoted | authority for full evidence profile | user-facing summary | future/later |
| `evals` | Detached verification/eval records | v0.3 or profile-promoted | authority when profile is active | user-facing summary | future/later |
| `manual_qa_records` | Manual QA profile, result, findings | v0.3 or profile-promoted | authority when profile is active | user-facing summary | future/later |
| `validator_runs` | Persisted validator results | v0.3 or profile-promoted | diagnostic state | internal/user-facing findings | future/later |
| `feedback_loops` | Feedback-loop policy records | v0.3 or profile-promoted | policy support | internal/user-facing summary | future/later |
| `tdd_traces` | TDD trace records | v0.3 or profile-promoted | policy/evidence support | internal/user-facing summary | future/later |
| `projection_jobs` | Durable projection outbox and rendered-output freshness | v0.4 or profile-promoted | auxiliary derived-view job state | internal/user-facing freshness | future/later |
| `reconcile_items` | Human-editable projection drift/proposal handling | v0.4 or profile-promoted | auxiliary until accepted through Core | user-facing | future/later |
| `connector_manifests` | Connector-managed file manifest and drift state | v0.4 or profile-promoted | diagnostic/support | internal | future/later |
| `persistent_locks` | Durable lock/recovery metadata if process locks are insufficient | v0.4 or profile-promoted | auxiliary | internal | future/later |
| `export_manifests` | Export/recover package manifest | v0.4 or profile-promoted | auxiliary support | internal/user-facing report | future/later |
| `recover_items` | Recovery findings and repair plan state | v0.4 or profile-promoted | diagnostic/support | internal/user-facing report | future/later |
| `task_spine_entries` | Journey/spine continuity records | future/diagnostic | supplemental | user-facing | non-stage-required |
| `journey_cards` | Render/cache support for journey views, if ever stored | future/diagnostic | derived display support | user-facing | non-stage-required |
| `shared_designs` | Shared design basis records when design-support profiles are promoted | future/diagnostic | policy support | user-facing summary | non-stage-required |
| `change_unit_dependencies` | Dependency/ordering visibility between Change Units | future/diagnostic | policy support | internal/user-facing summary | non-stage-required |
| `domain_terms` | Domain language/stewardship terms | future/diagnostic | policy support | user-facing summary | non-stage-required |
| `module_map_items` | Module map/stewardship records | future/diagnostic | policy support | internal/user-facing summary | non-stage-required |
| `interface_contracts` | Interface contract/stewardship records | future/diagnostic | policy support | internal/user-facing summary | non-stage-required |

## v0.1 Physical Schema

v0.1 is the Core Authority Smoke. It is intentionally small. It should be enough to register a project, create or load one Task, define one scoped work boundary, authorize one write, record one Run, register one artifact/evidence ref, append events, and return structured blockers.

The DDL below is a reference fragment for planning. It is not proof that a migration runner exists.

### Schema profile metadata

| Profile | Stage | Required for | Explicitly not required for this profile |
|---|---|---|---|
| Core Authority Smoke schema | v0.1 | narrow local authority loop | Decision Packets, Evidence Manifests, Manual QA, Eval, residual-risk acceptance, projection jobs, reconcile, validators, Journey, stewardship maps |
| First User-Value Slice schema | v0.2 | first user-value records and readable status | detached verification, full Manual QA, full projection job system, export/recover, broad operations |
| Agency Assurance schema | v0.3 or promoted profile | verification, QA, approval, feedback/TDD, validator support | v0.1/v0.2 exit unless promoted |
| Operations schema | v0.4 or promoted profile | projection jobs, reconcile, connector manifests, recover/export | v0.1/v0.2 exit unless promoted |
| Future / diagnostic schema | future/diagnostic | journey/spine, domain/module/interface diagnostics | all current stage exits unless promoted |

### Core Authority Smoke schema

Main v0.1 table count: 12 tables total, with 2 in `registry.sqlite` and 10 in project `state.sqlite`. This count is intentionally small enough for a first implementation slice.

#### `registry.sqlite`

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

Required `registry_meta` keys for v0.1 are `runtime_home_id` and `schema_version`. A later implementation may replace this with a more formal metadata table, but v0.1 only needs durable identity and version facts.

#### `state.sqlite`

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
  mode TEXT NOT NULL,
  lifecycle_phase TEXT NOT NULL,
  result TEXT,
  active_change_unit_id TEXT,
  state_version INTEGER NOT NULL DEFAULT 0,
  status_summary_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  closed_at TEXT
);

CREATE TABLE change_units (
  change_unit_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  summary TEXT NOT NULL,
  status TEXT NOT NULL,
  allowed_paths_json TEXT NOT NULL DEFAULT '[]',
  denied_paths_json TEXT NOT NULL DEFAULT '[]',
  touched_paths_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE write_authorizations (
  write_authorization_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT NOT NULL REFERENCES change_units(change_unit_id),
  status TEXT NOT NULL,
  basis_state_version INTEGER NOT NULL,
  allowed_paths_json TEXT NOT NULL DEFAULT '[]',
  denied_paths_json TEXT NOT NULL DEFAULT '[]',
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
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL
);

CREATE TABLE artifacts (
  artifact_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  kind TEXT NOT NULL,
  uri TEXT NOT NULL,
  sha256 TEXT NOT NULL,
  size_bytes INTEGER NOT NULL,
  content_type TEXT,
  redaction_state TEXT NOT NULL,
  retention_class TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE artifact_links (
  artifact_link_id TEXT PRIMARY KEY,
  artifact_id TEXT NOT NULL REFERENCES artifacts(artifact_id),
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  record_kind TEXT NOT NULL,
  record_id TEXT NOT NULL,
  relation TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE task_blockers (
  blocker_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  blocked_action TEXT NOT NULL,
  blocker_kind TEXT NOT NULL,
  status TEXT NOT NULL,
  message TEXT NOT NULL,
  owner_ref_json TEXT NOT NULL DEFAULT '{}',
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

Recommended v0.1 indexes:

```sql
CREATE INDEX idx_tasks_project_phase ON tasks(project_id, lifecycle_phase);
CREATE INDEX idx_change_units_task_status ON change_units(task_id, status);
CREATE INDEX idx_write_authorizations_task_status ON write_authorizations(task_id, status);
CREATE UNIQUE INDEX uq_runs_write_authorization_consumed
  ON runs(write_authorization_id)
  WHERE write_authorization_id IS NOT NULL;
CREATE INDEX idx_artifact_links_record ON artifact_links(record_kind, record_id);
CREATE INDEX idx_task_blockers_task_status ON task_blockers(task_id, status);
CREATE INDEX idx_task_events_task_seq ON task_events(task_id, event_seq);
```

v0.1 may store initial task creation through a narrow owner-valid setup path instead of a full natural-language intake system. It may return status/blocker output directly from `tasks`, `change_units`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, and `task_blockers`.

### Artifact directory layout

The directory layout is staged. v0.1 needs only the directories it actually writes, usually `artifacts/tmp/`, `artifacts/diffs/`, `artifacts/logs/`, and possibly `artifacts/bundles/`. Other directories in the reference layout are allowed but not v0.1 requirements.

### Artifact Kind Storage Notes

Artifact kind names describe registered files, not authority by themselves. A `diff`, `log`, `screenshot`, `bundle`, `manifest`, `checkpoint`, `qa`, `tdd`, `design`, `architecture`, `decision`, or `export_component` file becomes meaningful only after the `artifacts` row and compatible `artifact_links` row are committed.

### Artifact Registration Contract

Artifact registration is the storage boundary for artifact poisoning. A staged path, captured file, declared content type, and requested owner relation are untrusted until Core validates the path, rejects traversal or symlink escape, computes stored-byte integrity, applies redaction or omission rules, writes the `artifacts` row, and links it to a compatible owner record.

A committed artifact that supports state needs:

- a registered `ArtifactRef` shape, using the active stage value sets, owned by [MCP API And Schemas](mcp-api-and-schemas.md#artifactref)
- an `artifacts` row with `sha256`, `size_bytes`, `redaction_state`, and `retention_class`
- at least one compatible `artifact_links` row for the Task-scoped owner record
- a `task_events` row for the committed artifact registration or the state mutation that registered it

An `artifacts` row without a compatible owner link is not enough to satisfy evidence, QA, verification, projection, export, or close-related checks.

## v0.2 Additions

v0.2 is the First User-Value Slice. It should add records that help a person understand the work: intake state, simplified user judgments, approval-shaped sensitive-action Decision Packets, visible residual risk, evidence summaries, close blockers/readiness, and optional status-card freshness. It should still avoid committed Approval lifecycle storage, full assurance, projection job, reconciliation, and operations systems.

### First User-Value Slice schema

Main v0.2 addition count: 5 tables, plus optional `decision_requests` and `projection_status_cards` tables. These tables build on the v0.1 schema.

```sql
CREATE TABLE task_intake (
  intake_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  user_request TEXT NOT NULL,
  clarified_summary TEXT,
  open_questions_json TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE decision_packets (
  decision_packet_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  judgment_route TEXT NOT NULL,
  judgment_category TEXT NOT NULL,
  display_depth TEXT NOT NULL,
  judgment_payload_json TEXT NOT NULL DEFAULT '{}',
  status TEXT NOT NULL,
  question TEXT NOT NULL,
  options_json TEXT NOT NULL DEFAULT '[]',
  selected_option_json TEXT,
  affected_scope_json TEXT NOT NULL DEFAULT '{}',
  affected_gates_json TEXT NOT NULL DEFAULT '[]',
  context_refs_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  expires_at TEXT,
  resolved_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE residual_risks (
  residual_risk_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  status TEXT NOT NULL,
  visibility_status TEXT NOT NULL,
  summary TEXT NOT NULL,
  impact TEXT,
  mitigation TEXT,
  related_decision_packet_id TEXT REFERENCES decision_packets(decision_packet_id),
  accepted_at TEXT,
  accepted_by TEXT,
  accepted_risk_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE evidence_summaries (
  evidence_summary_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  status TEXT NOT NULL,
  summary TEXT NOT NULL,
  run_refs_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  gaps_json TEXT NOT NULL DEFAULT '[]',
  updated_at TEXT NOT NULL
);

CREATE TABLE close_readiness (
  close_readiness_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  status TEXT NOT NULL,
  blocker_refs_json TEXT NOT NULL DEFAULT '[]',
  evidence_summary_id TEXT REFERENCES evidence_summaries(evidence_summary_id),
  residual_risk_refs_json TEXT NOT NULL DEFAULT '[]',
  checked_state_version INTEGER NOT NULL,
  updated_at TEXT NOT NULL
);
```

Optional v0.2 prompt routing table:

Public refs for these v0.2 additions are intentionally small. `evidence_summaries` and `close_readiness` may be surfaced through `StateRecordRef` as `evidence_summary` and `close_readiness`; they summarize or check authority refs and do not imply the full `evidence_manifests`, verification, Manual QA, projection, or report/export profiles are active.

```sql
CREATE TABLE decision_requests (
  decision_request_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  decision_packet_id TEXT REFERENCES decision_packets(decision_packet_id),
  status TEXT NOT NULL,
  request_payload_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  expires_at TEXT
);
```

`decision_requests` never satisfies a judgment, gate, waiver, residual-risk acceptance, or close condition by itself. It is only routing or replay metadata for a compatible `decision_packets` row.

For `judgment_route=approve-sensitive-action`, minimum v0.2 stores the requested `judgment_payload.approval_scope` in `decision_packets.judgment_payload_json` and resolves the user's grant, denial, or expiry on the Decision Packet. It does not require a row in `approvals`, an Approval `StateRecordRef`, `approval_id`, `approval_refs`, or an `APR` projection.

Optional v0.2 status-card freshness table:

```sql
CREATE TABLE projection_status_cards (
  card_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  card_kind TEXT NOT NULL,
  source_state_version INTEGER NOT NULL,
  rendered_state_version INTEGER,
  status TEXT NOT NULL,
  summary_json TEXT NOT NULL DEFAULT '{}',
  updated_at TEXT NOT NULL
);
```

`projection_status_cards` is not a projection job system. It is an optional display/freshness cache for status or next-action cards. If omitted, v0.2 can compute freshness directly by comparing current `tasks.state_version` with the source version returned in a read response.

## Future / Later Profile Schema Candidates

This section preserves useful future schema candidates without putting them on the v0.1/v0.2 implementation path. Do not treat this inventory as a required DDL bundle.

### Agency Assurance schema

Agency Assurance profile storage is v0.3 or profile-promoted, not v0.1/v0.2. Candidate tables:

| Candidate table | Why it may matter later | Not required for |
|---|---|---|
| `approvals` | Sensitive-action approval lifecycle and drift handling | v0.1 authority loop, ordinary v0.2 judgment display including approval-shaped Decision Packets |
| `baselines` | Repository baseline capture for assurance, approval, and verification freshness | v0.1/v0.2 unless a promoted profile needs baseline checks |
| `evidence_manifests` | Full criteria-to-evidence coverage | v0.1 single artifact/evidence ref, v0.2 evidence summary |
| `evals` | Detached verification or evaluator review | v0.1/v0.2 |
| `manual_qa_records` | Manual QA result, findings, setup, evidence refs | v0.1/v0.2 |
| `validator_runs` | Persisted `ValidatorResult` rows | v0.1/v0.2 unless a narrow owner promotes a validator |
| `feedback_loops` | Policy support for selected feedback loop | v0.1/v0.2 |
| `tdd_traces` | Red/green/refactor evidence when TDD profile is selected | v0.1/v0.2 |

### Operations schema

Operations profile storage is v0.4 or profile-promoted. Candidate tables:

| Candidate table | Why it may matter later | Not required for |
|---|---|---|
| `projection_jobs` | Durable outbox for rendered Markdown or managed outputs | v0.1/v0.2; optional status cards do not require it |
| `reconcile_items` | Route human edits or projection drift into Core decisions | v0.1/v0.2 |
| `connector_manifests` | Track connector-managed files and drift | v0.1/v0.2 |
| `persistent_locks` | Durable lock/recovery metadata if needed beyond process locks | v0.1/v0.2 |
| `export_manifests` | Release handoff or export package metadata | v0.1/v0.2 |
| `recover_items` | Recovery findings, repair plan, and operator follow-up | v0.1/v0.2 |

### Future / diagnostic schema

Future or diagnostic schema candidates are non-stage-required until an owner promotes them:

- Journey/spine: `task_spine_entries`, `journey_cards`
- Domain and stewardship: `domain_terms`, `module_map_items`, `interface_contracts`
- Rich design support: `shared_designs`, `change_unit_dependencies`
- Diagnostics and polish: metrics, dashboards, context indexes, connector analytics, export/recover detail tables, richer projection caches

These records may be useful, but they must not become prerequisites for v0.1 Core Authority Smoke or v0.2 First User-Value Slice.

### Baseline capture format

Baseline capture is a future assurance/profile feature. When promoted, baseline storage should record enough data to prove the repository state used for approval, verification, or evidence freshness. Until that profile is active, v0.1/v0.2 do not need a `baselines` table or a baseline capture runner.

### Verification Bundle Shape

Verification bundles are future assurance/profile artifacts. They may combine baseline refs, run refs, artifact refs, evaluator inputs, and validation output after the verification profile is active. They are not required to record a v0.1 Run or a v0.2 evidence summary.

### Projection job table

`projection_jobs` is Operations profile storage. It is the durable outbox for projection rendering when full projection support is enabled. It is not part of v0.1 and is not required for v0.2 status or next-action cards.

When promoted, projection jobs should record `projection_kind`, `target_ref`, `source_state_version`, job status, output location or artifact ref, and failure information. These fields describe derived output freshness; they do not make rendered Markdown authoritative.

### Projection Worker Execution

Projection workers consume committed Core state and produce derived files or cards. Projection failure must not roll back committed Core state. A worker may update freshness/job state through Core-compatible ordering, but stale or failed projection output cannot authorize writes, satisfy evidence, satisfy acceptance, or close a Task.

### Validator runner skeleton

Persisted `validator_runs` are Agency Assurance profile behavior unless an owner explicitly promotes a narrow validator earlier. v0.1/v0.2 can return structured blockers without creating persisted validator-run storage.

### Evidence and Verification Profile Implementation Notes

Full evidence sufficiency, detached verification, Manual QA, and validator-backed assurance read committed state and registered artifacts from the profiles that are installed. They must not be simulated through Markdown, chat, or unregistered tool output.

## Event Semantics

### `task_events`

`task_events` is an append-only audit trail and event-order support table. It records what Core committed and in what order. It is not the normal authority source for current state, and v0.1/v0.2 state should not be reconstructed by replaying events during ordinary operation.

Current state tables are authoritative:

- `tasks`, `change_units`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, and `task_blockers` are v0.1 authority records.
- `decision_packets`, `residual_risks`, and other v0.2 rows become authority only for their own record family when their profile is active.
- Events support audit, debugging, idempotency explanation, projection freshness, and recovery history.

Deterministic event order is ascending `event_seq` in `state.sqlite`. Task-scoped readers filter by `task_id`. `created_at` is audit metadata; it is not enough for ordering when events share a timestamp.

Required event emission:

| Stage | Mutation | Event expectation |
|---|---|---|
| v0.1 | Project registration or project path/config update | Emit a project or task-scoped event if the project state changes; registry-only events may use `task_id=NULL`. |
| v0.1 | Task create/update/close state change | Emit an event with the new state version. |
| v0.1 | Change Unit or task boundary create/update | Emit an event and update affected state version. |
| v0.1 | `prepare_write` allow creates or refreshes a Write Authorization | Emit authorization-created or authorization-updated event. |
| v0.1 | `prepare_write` blocks and stores/updates a structured blocker | Emit blocker-opened or blocker-updated event. |
| v0.1 | `record_run` commits a Run | Emit run-recorded event. If a Write Authorization is consumed, emit the authorization-consumed relation in the same transaction or payload. |
| v0.1 | Artifact registration/link commit | Emit artifact-registered or include artifact refs in the owning mutation event. |
| v0.1 | Blocker resolved or superseded | Emit blocker-resolved or blocker-superseded event. |
| v0.1 | Idempotent replay returning an existing committed response | Do not append a new semantic event. The original event remains the committed audit fact. |
| v0.2 | Intake state create/update | Emit intake-updated event when persisted. |
| v0.2 | User judgment requested, answered, expired, or superseded | Emit decision event tied to the `decision_packets` row. |
| v0.2 | Residual risk opened, changed, accepted, mitigated, deferred, or superseded | Emit residual-risk event. |
| v0.2 | Evidence summary or close readiness changes | Emit evidence-summary-updated or close-readiness-updated event when persisted. |
| v0.2 optional | Projection/status-card freshness changes | Emit freshness/status-card event only if that optional table is installed. |

Malformed requests, dry runs, pre-commit state conflicts, and invalid requests that do not mutate state do not need `task_events` rows. If a blocked request creates or updates a stored blocker, that blocker mutation is the event-worthy state change.

### Projection freshness without projection authority

Projection freshness is a comparison between a readable output's `source_state_version` and current Core state. It does not make the readable output a state authority.

- v0.1 may have no projection freshness table. Reads can return current state directly.
- v0.2 may store optional `projection_status_cards` for status or next-action cards.
- v0.4 or a promoted profile may add `projection_jobs` for durable rendering.

In every stage, stale Markdown or a stale card can warn or block user trust through the owner path, but it cannot authorize writes, satisfy evidence, record acceptance, accept residual risk, or close a Task.

## Migration And Validation Notes

No migration runner exists in this repository. The notes below describe constraints a future implementation must satisfy when it chooses a migration mechanism.

### Storage hardening as an authority boundary

SQLite can store malformed rows unless Core and migrations prevent them. A row is authoritative only when it matches the owner schema, owner value set, state-version basis, idempotency key, and artifact owner-link contract.

`doctor`, `recover`, artifact checks, and conformance runners should report malformed JSON, unknown owner-bound values, mismatched replay rows, stale state-version claims, artifact hash mismatch, and invalid owner links as storage integrity findings, not projection drift.

### JSON TEXT validation

JSON `TEXT` columns are storage flexibility, not permission to store arbitrary JSON. Before Core commits a JSON `TEXT` value, it must parse the value and validate the parsed shape against the owner:

- API-shaped payloads validate against [MCP API And Schemas](mcp-api-and-schemas.md).
- Storage-only JSON validates against this page or the owner document named by this page.
- SQLite defaults such as `'{}'` and `'[]'` are storage representation rules; they do not make public API fields optional.

Malformed JSON and schema-incompatible JSON are invalid state. If a SQLite build supports JSON checks, migrations may add `CHECK (json_valid(column_name))` as defense in depth, but Core shape validation before commit still owns meaning.

### Canonical enum hardening

Status-like `TEXT` columns are not open strings. Core validation owns allowed values; database `CHECK` constraints or lookup tables are defense in depth.

Early hardening should cover:

| Field(s) | Owner/value source |
|---|---|
| `tasks.mode`, `tasks.lifecycle_phase`, `tasks.result` | [Kernel Reference](kernel.md) |
| `change_units.status` | Kernel/Change Unit owner rules |
| `write_authorizations.status` | [Kernel `prepare_write`](kernel.md#prepare_write) and [`harness.prepare_write`](mcp-api-and-schemas.md#harnessprepare_write) |
| `runs.kind`, `runs.status` | [`harness.record_run`](mcp-api-and-schemas.md#harnessrecord_run) and storage compatibility notes |
| `task_blockers.status`, `blocked_action`, `blocker_kind` | Kernel/API blocker owners |
| `tool_invocations.status` | storage idempotency replay semantics |
| `decision_packets.status`, `judgment_route`, `judgment_category`, `display_depth` | user-judgment API/kernel owners |
| `residual_risks.status`, `visibility_status` | close and residual-risk owners |
| `evidence_summaries.status`, `close_readiness.status` | evidence/close-readiness owner behavior |
| Future `projection_jobs.status`, `projection_jobs.projection_kind` | Projection/API owners when Operations profile is active |
| Future `validator_runs.status` | `ValidatorResult` owner when assurance profile is active |
| Future `project_surfaces.guarantee_level`, `write_authorizations.guarantee_level`, `validator_runs.guarantee_level` | Security threat model and agent-integration guarantee-level owners when the relevant profile is active |
| Future `approvals.status` | Approval lifecycle owner when approval profile is active |
| Future `evidence_manifests.status` | Evidence profile owner when full Evidence Manifest profile is active |
| Future `feedback_loops.loop_kind`, `feedback_loops.status`, `tdd_traces.status` | Design-quality/API owners when feedback/TDD profiles are active |
| Future `connector_manifests.status`, `baselines.status`, `decision_requests.status`, `task_spine_entries.status`, `change_unit_dependencies.status`, `shared_designs.status`, `reconcile_items.status`, `domain_terms.status`, `module_map_items.status`, `interface_contracts.review_status` | Storage compatibility values below, only when the optional/future table is retained, seeded, or active |

Unknown owner-bound values are invalid state unless a fixture explicitly exercises invalid-state recovery. Migrations must stop before tightening if unknown values are present; they must not silently map unknown values to fallback values that no owner defines.

Storage-owned compatibility values promoted here:

| Field | Durable values | Meaning |
|---|---|---|
| `runs.status` | `completed`, `interrupted`, `blocked`, `violation` | A committed Run row. Only `completed` can support evidence through normal owner refs. Other values are audit/recovery records and do not satisfy evidence, QA, verification, acceptance, or close readiness by themselves. |
| `change_units.status` | `planned`, `active`, `completed`, `deferred`, `superseded` | Scope lifecycle. Only the active compatible Change Unit scopes new writes. |
| `write_authorizations.status` | `active`, `consumed`, `expired`, `revoked`, `blocked` | Durable authorization lifecycle. Only `active` and compatible rows can be consumed by `record_run`. |
| `task_blockers.status` | `open`, `resolved`, `superseded` | Stored blocker lifecycle. Open blockers remain visible until Core resolves or supersedes them. |
| `tool_invocations.status` | `committed` | A row exists only for a committed replayable response. |
| `residual_risks.status` | `open`, `accepted`, `mitigated`, `deferred`, `superseded` | Residual-risk lifecycle. Accepted risk remains separate from work acceptance. |

Profile-only compatibility values retained for future seed loaders and optional profile implementations:

| Field | Durable values | Meaning |
|---|---|---|
| `baselines.status` | `captured`, `stale` | Baseline freshness for assurance profiles. |
| `connector_manifests.status` | `current`, `drifted` | Connector-managed file state; drift must route through the owning reconcile/operations path. |
| `decision_requests.status` | `open`, `linked`, `closed`, `expired`, `cancelled`, `superseded` | Prompt routing lifecycle only; authority comes through linked `decision_packets`. |
| `task_spine_entries.status` | `current`, `superseded` | Journey/spine continuity support, not current state authority. |
| `change_unit_dependencies.status` | `open`, `satisfied`, `blocked`, `deferred`, `superseded` | Dependency visibility; not a scheduler or parallel-lane authority. |
| `shared_designs.status` | `proposed`, `active`, `stale`, `deferred`, `superseded` | Design-support basis; not Approval, work acceptance, or residual-risk acceptance. |
| `reconcile_items.status` | `pending`, `merged`, `rejected`, `converted_to_note`, `decision_created`, `deferred` | Reconcile outcome state; only accepted Core mutations change authority. |
| `domain_terms.status` | `active`, `conflict` | Domain-language support. Conflicts remain visible until resolved by the owner path. |
| `module_map_items.status` | `active` | Current usable module-map support record when that profile is active. |
| `interface_contracts.review_status` | `pending`, `reviewed` | Interface review support; does not waive risk or override gates. |

Future table value sets should be used only when the table's owner profile is active, a fixture explicitly seeds that optional table, or the owner document explicitly promotes the values.

### Migrations

Future migrations should:

- Record schema/profile version in `registry_meta` and `project_state` or an equivalent chosen metadata mechanism.
- Validate JSON and owner-bound status values before tightening constraints.
- Preserve `task_events.event_seq` order and never rewrite historical ordering.
- Preserve artifact hashes and owner links, or mark affected artifacts invalid for recovery.
- Stop on unknown owner-bound enum/status values instead of inventing fallback meanings.
- Treat projection/card/job freshness as derived state, not as canonical state.

These notes do not require a specific migration runner, migration file format, or CLI command in v0.1.

### Lock policy

Runtime mutations should serialize through the Core transaction order owned by [Runtime Architecture](runtime-architecture.md#state-transaction-flow). v0.1 can use ordinary SQLite transactions plus a process/project lock if needed. `persistent_locks` is a later Operations candidate, not a v0.1 table.

Locks protect concurrent writes; they do not provide OS sandboxing, artifact integrity, or tamper-proof storage.
