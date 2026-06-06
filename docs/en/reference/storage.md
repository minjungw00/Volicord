# Storage

## What This Document Owns

This is reference documentation for a future local Harness Server. No database,
migration runner, server, runtime state, generated artifact, or generated
projection exists in this repository yet. Current repository phase and handoff
status are tracked in [MVP Plan](../build/mvp-plan.md#documentation-acceptance-status).

This page owns the initial storage design for the active first implementation
slice: Runtime Home identity, project-local persisted records, artifact storage
metadata, artifact links, the minimal evidence coverage record, storage-owned
JSON `TEXT` rules, storage-owned enum hardening, and the boundary between
active persistence and later/profile storage.

Lifecycle meanings, gate meanings, public API payloads, and user-visible
decisions stay with their owner documents. This page names the records and
essential persisted fields needed to implement those contracts; it does not
redefine Core or API state machines.

## Read This When

- You need the smallest storage slice for the first executable authority loop
  and MVP-1 User Work Loop.
- You are separating Core-owned state from chat, Markdown projections, connector
  output, tool output, and report text.
- You are checking which persisted records are needed for write authorization,
  evidence linkage, blockers, status, and close readiness.
- You are making sure later/profile tables do not become MVP prerequisites.

## Related Owners

| Concern | Owner |
|---|---|
| Public MCP request/response shapes | [MVP API](api/mvp-api.md) and [API Schema Core](api/schema-core.md) |
| Active MVP method row mutations, dry-run/failure side effects, and response refs | [MVP API: Active MVP transition matrix](api/mvp-api.md#active-mvp-transition-matrix) |
| `ArtifactRef`, staged active ref kinds, idempotency, and state conflict behavior | [API Schema Core](api/schema-core.md#artifactref), [API Schema Core: Stage-Specific Active Value Sets](api/schema-core.md#stage-specific-active-value-sets), and [API Errors](api/errors.md) |
| Task lifecycle, gates, `prepare_write`, Write Authorization, `record_run`, `close_task`, and stable events | [Core Model Reference](core-model.md) |
| Core process model, transaction order, locks, projection/reconcile placement | [Runtime Architecture Reference](runtime-architecture.md) |
| Projection authority, freshness, managed blocks, rendered templates | [Projection And Templates Reference](projection-and-templates.md) and [Template Reference](templates/README.md) |
| Operator behavior, doctor/recover/export/reconcile/conformance entrypoints | [Operations And Conformance Reference](operations-and-conformance.md) |
| Fixture format and assertion semantics | [Conformance Fixtures Reference](conformance-fixtures.md) |
| Stage sequence and implementation readiness | [Build: MVP-1 User Work Loop](../build/mvp-plan.md#user-work-loop), [Engineering Checkpoint](../build/mvp-plan.md#first-internal-smoke-target), and [MVP Plan](../build/mvp-plan.md) |

## Active First Implementation Storage Slice

The active first implementation storage slice is the smallest persisted set
needed for the first executable authority loop and the MVP-1 user work loop. It
is an initial schema design, not an accepted migration plan or proof that
runtime data exists.

The active persisted records are:

- `project_state`
- `surfaces`, or an equivalent reference-surface registration record
- `tasks`
- `task_events`
- `change_units`
- `user_judgments`
- `write_authorizations`
- `runs`
- `artifacts`
- `artifact_links`
- `evidence_summaries`, or an equivalent minimal evidence coverage record
- `blockers`
- `tool_invocations`

No other persisted table family is an MVP prerequisite. Future implementations
may choose physical indexes, lookup tables, or JSON decomposition for these
records, but those choices must not pull later/profile records into the active
slice.

Active storage preserves these authority boundaries:

- Core-owned state rows are the source of truth for current Harness state.
- `task_events` is an audit and ordering trail when retained. It is not the
  normal source used to reconstruct current state.
- `tool_invocations` supports committed idempotency replay. It is not a separate
  user-facing domain record.
- Requirements shaping output is stored on active `tasks`, `change_units`,
  `user_judgments`, `evidence_summaries`, and `blockers` as needed. MVP-1 has no
  committed `shared_designs` table, Discovery Brief table, Question Queue table,
  Assumption Register table, First Safe Change Unit Candidate table, or required
  Shared Design projection cache.
- `artifacts` store registered evidence bytes or safe metadata with integrity,
  redaction, producer, retention, and availability facts. They do not prove
  sufficiency until Core links them through owner-valid rows.
- `artifact_links` connect artifacts to owner records. A link is not a report,
  projection, QA result, Eval, final acceptance, or residual-risk acceptance.
- `evidence_summaries` is the minimal coverage and gap record for MVP-1. It is
  not a full Evidence Manifest report table.
- Raw secrets, tokens, and full sensitive logs are not valid evidence bytes.
  Store redacted bytes, `secret_omitted` / `blocked` notices, safe handles, or
  other owner-approved safe representations instead.
- Chat, Markdown projections, generated reports, connector manifests, tool
  output, and operator output are not authority unless a Core mutation records an
  owner-valid state row, artifact, or artifact link.
- Status cards, close results, next actions, run/evidence summaries, and compact
  views are derived outputs. They can be stale, failed, absent, or recomputed
  without changing persisted authority records.
- Future/profile tables become required only when the owning profile or tool
  path is active or used.

## Runtime Home Identity And Risks

Harness keeps one local Runtime Home and one project-local state database per
registered project. The default reference location is `~/.harness`, but an
implementation may choose a configured equivalent.

Reference layout:

```text
~/.harness/
  registry.sqlite
  projects/
    PRJ-0001/
      project.yaml
      state.sqlite
      artifacts/
        tmp/
        diffs/
        logs/
        screenshots/
        checkpoints/
```

`registry.sqlite` stores Runtime Home identity and minimal project registration
metadata. `project.yaml` stores static project configuration only. `state.sqlite`
stores project-local Core state. Artifact directories store registered files or
safe metadata after Core applies the artifact registration boundary.

`project.yaml` must not store current Task state, current gates, Write
Authorization state, evidence sufficiency, final acceptance, or residual-risk
acceptance.

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

Runtime Home identity should not depend only on a path. A copied or moved
Runtime Home may keep the same stored `runtime_home_id`; a new Runtime Home
should get a new id. `doctor` and recovery flows can use that identity to report
suspicious copies, duplicate registrations, or path drift, but the id does not
provide tamper-proofing.

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
| Runtime Home, `state.sqlite`, `registry.sqlite`, or artifact directories are writable by unrelated users, shared groups, shared containers, or broad local processes. | Report tampering and evidence-poisoning risk. Core must still validate rows, owner links, hashes, and artifact registration before trusting meaning. |
| Artifact storage or exports are readable by unrelated users, shared groups, shared containers, or broad local processes. | Report confidentiality risk without echoing sensitive values. |
| A registered artifact `sha256`, `size_bytes`, `content_type`, `redaction_state`, owner link, or resolved storage location no longer matches storage metadata. | Treat as evidence integrity failure or recovery input, not projection drift. Missing bytes or a diagnostic such as `hash_mismatch` makes related evidence stale or blocked. |

## Active Record Contracts

The table below gives the contract-level persisted fields for the active slice.
Exact lifecycle meanings and API response meanings stay with [Core Model
Reference](core-model.md), [MVP API](api/mvp-api.md), and [API Schema
Core](api/schema-core.md).
The method-by-method index for which API call creates or updates these rows is
[MVP API: Active MVP transition matrix](api/mvp-api.md#active-mvp-transition-matrix).

| Record | Minimal persisted role | Essential fields |
|---|---|---|
| `project_state` | Project-local state header, state clock, active Task pointer, and active/default surface pointer. | `project_id`, `schema_version`, `storage_profile`, `state_version`, `active_task_id`, `default_surface_id`, `created_at`, `updated_at`. |
| `surfaces` | Reference surface registration for the local caller/display path. This records what surface Core believes it is talking to; it is not a broad connector ecosystem table. | `surface_id`, `project_id`, `surface_kind`, `display_name`, `registration_source`, `local_access_posture`, `capability_profile_json`, `guarantee_level`, `status`, `created_at`, `updated_at`. |
| `tasks` | User-value work unit, task-scoped state clock, and active requirements-shaping summary. | `task_id`, `project_id`, `title`, `user_request`, `current_goal_summary`, `mode`, `lifecycle_phase`, `result`, `summary`, `success_criteria_json`, `non_goals_json`, `affected_areas_json`, `affected_path_candidates_json`, `constraints_json`, `confirmed_facts_json`, `remaining_uncertainties_json`, `blocking_question`, `next_safe_action`, `active_change_unit_id`, `state_version`, `created_at`, `updated_at`, `closed_at`. |
| `task_events` | Append-only audit/order trail for committed Core mutations. | `event_id`, `task_id` or project scope, `event_seq`, `event_type`, `state_version`, `actor_kind`, `surface_id`, `payload_json`, `created_at`. |
| `change_units` | Proposed or current scoped work boundary for product writes and close basis. | `change_unit_id`, `task_id`, `scope_summary`, `affected_areas_json`, `affected_path_candidates_json`, `non_goals_json`, `success_criteria_json`, `allowed_paths_json`, `denied_paths_json`, `sensitive_categories_json`, `baseline_ref`, `autonomy_boundary_json`, `status`, `created_at`, `updated_at`. |
| `user_judgments` | User-owned judgment record for product decision, technical decision, scope decision, sensitive approval, QA waiver, verification-risk acceptance, final acceptance, residual-risk acceptance, and cancellation. | `user_judgment_id`, `task_id`, `change_unit_id`, `judgment_kind`, `presentation`, `status`, `state_summary_at_request_json`, `question`, `judgment_context_json`, `boundary_text_json`, `options_json`, `recommendation_json`, `selected_option_json`, `judgment_payload_json`, `affected_scope_json`, `affected_gates_json`, `affected_acceptance_criteria_json`, `context_refs_json`, `artifact_refs_json`, `resolution_json`, `expires_at`, `resolved_at`, `created_at`, `updated_at`. |
| `write_authorizations` | Durable single-use cooperative record created only by non-dry-run `prepare_write.decision=allowed`. The row preserves the full active MVP `AuthorizedAttemptScope` used by Core comparison. | `write_authorization_id`, `task_id`, `change_unit_id`, `surface_id`, `status`, `basis_state_version`, `attempt_scope_json`, `consumed_by_run_id`, `expires_at`, `created_at`, `updated_at`, `consumed_at`. |
| `runs` | Committed execution or observation record, including compatible write consumption when a product write happened. | `run_id`, `task_id`, `change_unit_id`, `write_authorization_id`, `surface_id`, `kind`, `status`, `product_write`, `baseline_ref`, `summary`, `observed_attempt_json`, `observed_changes_json`, `command_results_json`, `tool_invocations_json`, `network_accesses_json`, `secret_accesses_json`, `evidence_updates_json`, `observation_capability_json`, `created_at`. |
| `artifacts` | Registered durable evidence bytes or safe metadata with integrity and redaction facts. | `artifact_id`, `project_id`, `task_id`, `run_id`, `kind`, `uri`, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `retention_class`, `produced_by`, `status`, `created_at`, `updated_at`. |
| `artifact_links` | Owner relation from an artifact to the Core/API record it supports. | `artifact_link_id`, `artifact_id`, `task_id`, `owner_record_kind`, `owner_record_id`, `relation`, `created_at`. |
| `evidence_summaries` | Minimal evidence coverage and gap record for MVP-1 status and close. It replaces full Evidence Manifest tables in the active slice. | `evidence_summary_id`, `task_id`, `change_unit_id`, `status`, `coverage_items_json`, `summary`, `supporting_run_ids_json`, `supporting_artifact_link_ids_json`, `gap_blocker_ids_json`, `updated_at`. |
| `blockers` | Structured blocker for next action, write compatibility, evidence gaps, close readiness, or recovery. | `blocker_id`, `task_id`, `blocked_action`, `blocker_kind`, `status`, `message`, `owner_ref_json`, `related_refs_json`, `required_next_action`, `created_at`, `resolved_at`. |
| `tool_invocations` | Committed idempotency replay row for non-dry-run state-changing tool responses. | `invocation_id`, `project_id`, `tool_name`, `idempotency_key`, `request_hash`, `task_id`, `basis_state_version`, `response_json`, `status`, `created_at`. |

`tool_invocations` rows exist only for committed replayable non-dry-run
responses. Dry runs and pre-commit conflicts do not reserve idempotency keys in
storage.

`tasks.user_request` stores the original user request. Shaping updates clarify
the current goal, success criteria, non-goals, affected areas, path candidates,
constraints, confirmed facts, remaining uncertainties, blocking question, and
next safe action without replacing the original wording. `tasks.constraints_json`
preserves the active `TaskShapingUpdate.constraints` content, currently allowed
paths and sensitive categories. A blocking question that belongs to the user
becomes a `UserJudgmentCandidate` and then a `user_judgments` row when
requested/recorded; a non-judgment blocker uses the active `blockers` path.

`change_units.status` may represent a proposed candidate or active/superseded
scope according to the Core/API owner rules. A "First Safe Change Unit
Candidate" is a proposed Change Unit boundary carried by this record family, not
a separate active table or ref kind. `change_units` stores the active
`ChangeUnitShapingUpdate` scope content, including affected areas and path
candidates, allowed and denied paths, non-goals, success criteria, sensitive
categories, `baseline_ref`, and the compact `autonomy_boundary_json`.

`user_judgments.judgment_kind` is the stored judgment identity. Display labels
are derived at read/render time from `judgment_kind` and locale; active storage
does not keep a canonical `display_label` column or use display text for
compatibility checks, validators, gates, close aggregation, or owner refs.

`write_authorizations.attempt_scope_json` is the storage serialization of
`AuthorizedAttemptScope` from [API Schema Core](api/schema-core.md#evidence-and-pre-write-scope-schemas).
It must preserve the intended operation, intended paths, intended tools,
intended commands and command classes, product-file-write intent, intended
network targets, intended secret handles/scope, sensitive categories,
`baseline_ref`, `task_id`, `change_unit_id`, `basis_state_version`,
`surface_id`, related user judgment refs, and `guarantee_level`. The top-level
`task_id`, `change_unit_id`, `surface_id`, and `basis_state_version` columns are
query/index fields; Core comparison uses the stored attempt scope as the
authoritative authorization boundary.

`runs.observed_attempt_json` is the normalized storage bundle for
`record_run` compatibility comparison. It preserves the reported product-write
flag, baseline, observed changed paths, command and command-class observations,
tool use, network observations, secret-access observations, sensitive categories
when observed, Task/Change Unit/surface context, and the comparison outcome. The
more specific JSON columns keep the active `RecordRunPayload` branches available
for evidence and read responses. `observation_capability_json` records fields
the active surface could not honestly observe or attest. Unsupported or absent
observation is stored as unsupported/unknown, not as verified success; if a
required comparison fact is unsupported, Core must narrow the claim, block,
record a violation/audit path when explicitly supported, or return/report
insufficient surface capability rather than consuming the authorization as fully
compatible.

State clocks are scoped, not global. Task-scoped mutations use
`tasks.state_version`; project-scoped mutations with no Core-resolved primary
Task use `project_state.state_version`. `tool_invocations.basis_state_version`
stores the affected-scope version used as the compatibility basis before the
committed mutation; `response_json` stores the original response, including the
resulting `ToolResponseBase.state_version`.

## Artifact And Evidence Boundary

MVP-1 storage uses three small records for evidence linkage:

| Record | Active responsibility | Not responsible for |
|---|---|---|
| `artifacts` | Register durable bytes or safe metadata and integrity facts. | Evidence sufficiency, QA, Eval, final acceptance, residual-risk acceptance, export bundles, report prose. |
| `artifact_links` | Link an artifact to a Task, Change Unit, Run, user judgment, blocker, or minimal evidence summary. | Creating the owner record, satisfying a gate by itself, rendering a report. |
| `evidence_summaries` | Persist the minimal coverage/gap result Core needs for status, run/evidence summary, and close. | Full criteria matrix, Evidence Manifest report tables, detached verification, Manual QA matrix, long-term analytics. |

The evidence-eligible artifact contract is the combination of `artifacts` and
`artifact_links`. Together they must provide `artifact_id`, Task or equivalent
owner scope, kind, `uri`, `sha256`, `size_bytes`, `content_type`,
`redaction_state`, `produced_by`, relation owner, and `retention_class`.
Critical or close-relevant evidence missing any required metadata, owner link,
availability fact, or integrity match cannot be `sufficient`; Core marks the
affected coverage `stale` or `blocked`, and close remains blocked when required
evidence is affected.

An implementation may name the minimal coverage record differently only if it
preserves the same role and owner links. The active slice must not require full
Evidence Manifest storage when `evidence_summaries` or an equivalent minimal
coverage record is enough.

## Persisted State Vs Derived Status/View

Persisted active state is the set of rows Core commits. Derived status/view is
what Core or a renderer computes from those rows for users, agents, or
operators.

| Derived output | Source records | Active storage rule |
|---|---|---|
| Status card / task summary | `project_state`, `surfaces`, `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `evidence_summaries`, `blockers` | Derived view. It may be recomputed on read; no `projection_status_cards` or `projection_jobs` table is required. |
| Next safe actions | Open blockers, pending user judgments, write-check state, evidence summaries, Task lifecycle | Derived view. It does not create a Task, judgment, Run, evidence summary, artifact, or Write Authorization. |
| Run/evidence summary | `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers` | Derived view over active records. It is not a full Evidence Manifest or report projection. |
| Close readiness | Task lifecycle, scope state, pending user judgments, evidence coverage, artifact availability, open blockers, final-acceptance and residual-risk user judgments | Derived check. Active storage keeps the owner records and blockers used by the check, not a separate `close_readiness` source of truth. |
| Projection freshness | Current state version compared with the source version returned by the read/view response | Derived diagnostic. Full `projection_jobs` storage is Operations Profile or profile-promoted storage. |

## Fields Needed For Close-Blocker Calculation

MVP-1 close-blocker calculation reads current persisted records and derives the
close result. It does not need Journey, Spine, full Evidence Manifest report
tables, Eval, full Manual QA matrix, export/recover tables, projection jobs,
broad validator-run archives, long-term metrics, or connector ecosystem tables.

| Blocker or close fact | Minimum source fields |
|---|---|
| Active Task exists and is closeable | `project_state.active_task_id`, `tasks.lifecycle_phase`, `tasks.result`, `tasks.closed_at` |
| Scope is present and current | `tasks.active_change_unit_id`, `change_units.status`, `change_units.scope_summary`, `change_units.non_goals_json`, `change_units.success_criteria_json` |
| User-owned judgment is unresolved | `user_judgments.judgment_kind`, `user_judgments.status`, `user_judgments.affected_scope_json`, `user_judgments.context_refs_json` |
| Sensitive-action permission is missing or denied | `user_judgments` rows with `judgment_kind=sensitive_approval`, plus current `write_authorizations.attempt_scope_json.related_user_judgment_refs` when a write is involved |
| Write Authorization is missing, expired, stale, revoked, consumed, or incompatible | `write_authorizations.status`, `write_authorizations.basis_state_version`, `write_authorizations.attempt_scope_json`, `write_authorizations.consumed_by_run_id`, current `tasks.state_version`, current `tasks.active_change_unit_id`, and the current surface/profile facts |
| Run or artifact support is missing or stale | `runs.status`, `runs.product_write`, `runs.baseline_ref`, `runs.observed_attempt_json`, `runs.observation_capability_json`, `runs.observed_changes_json`, `runs.command_results_json`, `runs.tool_invocations_json`, `runs.network_accesses_json`, `runs.secret_accesses_json`, `artifacts.status`, `artifacts.sha256`, `artifacts.size_bytes`, `artifacts.content_type`, `artifacts.redaction_state`, `artifact_links.owner_record_kind`, `artifact_links.owner_record_id` |
| Evidence coverage is missing, insufficient, or stale | `evidence_summaries.status`, `evidence_summaries.coverage_items_json`, `evidence_summaries.supporting_artifact_link_ids_json`, `evidence_summaries.gap_blocker_ids_json` |
| Final acceptance is required but missing | `user_judgments` rows with `judgment_kind=final_acceptance` and compatible `status` / `selected_option_json` |
| Residual risk is not visible or not accepted | `blockers` rows with residual-risk blocker kinds, plus `user_judgments` rows with `judgment_kind=residual_risk_acceptance` when acceptance is required |
| A blocker is still open | `blockers.status`, `blockers.blocker_kind`, `blockers.blocked_action`, `blockers.related_refs_json`, `blockers.required_next_action` |
| Readable status is stale | Current `tasks.state_version` compared with the source version returned by the read/card response; later `projection_jobs` only when Operations Profile is active |

The close response may expose a compact close-readiness summary, evidence
summary, and next action. Those are derived outputs over active records.
Persisting a `close_readiness`, status-card cache, projection cache, or full
report table is optional/later unless an owner profile promotes it.

For MVP-1, `evidence_summaries.status` uses exactly `not_required`,
`none`, `partial`, `sufficient`, `stale`, and `blocked`. If `coverage_items_json`
is present, each item's `coverage_state` uses exactly `supported`, `unsupported`,
`partial`, `not_applicable`, `stale`, and `blocked`. `status=sufficient`
is the only evidence state that can satisfy close when evidence is required.
Full Evidence Manifest rows, detached Eval rows, and Manual QA matrices are not
needed for this active storage slice unless their owner profiles are active.
If a close-required coverage item depends on a missing artifact, absent
`sha256` / `size_bytes` / `content_type` / `redaction_state` metadata,
unresolved relation owner, unavailable bytes, or an integrity failure such as
`hash_mismatch`, storage exposes that fact to Core and the evidence state stays
`stale` or `blocked` instead of `sufficient`.

Write Authorization rows are not close-readiness rows. A stale, missing,
expired, revoked, consumed, or incompatible authorization, or a blocked
`prepare_write` decision that created no authorization row, affects close only
through the current Run, scope, artifact, evidence summary, or blocker record
that depends on it. Storage must not turn an authorization lifecycle value or
blocked write-check response into a close result, and must not use an attempted
invalid authorization ref as evidence support.

## Later/Profile Storage

Later/profile storage is useful design inventory. It must not be read as an MVP
DDL bundle or first-implementation prerequisite.

| Later/profile table family | Why it may matter later | Active-slice replacement |
|---|---|---|
| Full Eval system, including `evals` and evaluator bundles | Detached verification and independence hardening | Runs, artifacts, artifact links, evidence summaries, and blockers; no detached assurance claim unless the owner profile is active |
| Full Manual QA matrix, including `manual_qa_records` | Human inspection workflows, findings, setup, and QA evidence refs | User judgment, blocker, and evidence summary visibility for active user-owned QA waiver/risk questions; no Manual QA pass, matrix, or close blocker unless the owner profile is active |
| Full Evidence Manifest report tables, including detailed `evidence_manifests` | Criteria-to-evidence matrices and rich reports | `evidence_summaries` or equivalent minimal evidence coverage plus artifact links |
| Shared Design/design-support records, including `shared_designs` and full design artifacts | Rich requirements/design history and later-profile design review | Active Task shaping fields, proposed or active Change Units, user-judgment candidates/records, blockers, and evidence summaries as needed |
| Projection job system, including `projection_jobs` and durable projection caches | Durable outbox for rendered Markdown or managed outputs | Read-time compact views and source-version freshness display |
| Export/recover tables, including `export_manifests`, `recover_items`, and release-handoff bundles | Operations, handoff, recovery, and export packages | Active artifacts and blockers only, unless Operations Profile is active |
| Broad validator run archive, including `validator_runs` | Persisted validator history and diagnostic trend analysis | Current blockers, API `ValidatorResult` response data, and owner-field validation |
| Long-term metrics tables | Trend analysis, latency, turnaround, and operational diagnostics | None in active storage; status is derived from current records |
| Connector ecosystem tables, including connector manifests, marketplace records, connector analytics, and remote-surface inventories | Broad connector operations and marketplace behavior | `surfaces` or equivalent reference-surface registration for the local active surface only |

Additional later/profile candidates stay outside the active slice until an owner
promotes them:

- committed `approvals` table for a richer Approval lifecycle
- `residual_risks` table for a rich residual-risk lifecycle separate from
  blockers and user judgments
- `baselines` for repository baseline capture beyond the active compatibility
  basis
- `reconcile_items` for routing human edits or projection drift
- `persistent_locks` for durable lock/recovery metadata beyond process/project
  locking
- Journey/spine continuity records such as `task_spine_entries` and
  `journey_cards`
- design/stewardship records such as `domain_terms`, `module_map_items`,
  `interface_contracts`, and `change_unit_dependencies`

## Event And Idempotency Semantics

`task_events` is an append-only audit trail and event-order support table when
the implementation retains it. It records what Core committed and in what order.
It is not the normal authority source for current state, and active state should
not be reconstructed by replaying events during ordinary operation.

Current state tables are authoritative:

- `project_state`, `surfaces`, `tasks`, `change_units`, `user_judgments`,
  `write_authorizations`, `runs`, `artifacts`, `artifact_links`,
  `evidence_summaries`, and `blockers` are active current records.
- `task_events` supports audit, debugging, idempotency explanation, projection
  freshness, and recovery history.
- `tool_invocations` supports exact committed replay of non-dry-run
  state-changing tool responses.

Required event emission applies only to committed state mutations. Malformed
requests, dry runs, pre-commit state conflicts, and invalid requests that do not
mutate state do not need `task_events` rows. If a blocked request creates or
updates a stored blocker, that blocker mutation is the event-worthy state
change. `dry_run=true` creates no current record, `task_events` row, artifact,
consumable Write Authorization, projection job, or `tool_invocations` replay
row.

For `harness.record_run`, pre-commit rejection creates no Run row, artifact row,
artifact link, evidence summary, authorization consumption, blocker/gate update,
`task_events` row, projection job, state-version advance, or `tool_invocations`
row. The only active-contract exception is an explicit committed
violation/audit path for observed after-the-fact behavior, which may store a
`runs.status=violation` row plus recovery/blocker/event state but must not
consume an invalid authorization or satisfy evidence, QA, verification, final
acceptance, residual-risk acceptance, or close readiness.

<a id="canonical-enum-hardening"></a>

## Storage Validation And Enum Hardening

SQLite can store malformed rows unless Core and migrations prevent them. A row
is authoritative only when it matches the owner schema, owner value set,
state-version basis, idempotency key, and artifact owner-link contract.

JSON `TEXT` columns are storage flexibility, not permission to store arbitrary
JSON. Before Core commits a JSON `TEXT` value, it must parse the value and
validate the parsed shape against the owner:

- API-shaped payloads validate against [MVP API](api/mvp-api.md) and
  [API Schema Core](api/schema-core.md).
- Storage-only JSON validates against this page or the owner document named by
  this page.
- SQLite defaults such as `'{}'` and `'[]'` are storage representation rules;
  they do not make public API fields optional.

Status-like `TEXT` columns are not open strings. Core validation owns allowed
values; database `CHECK` constraints or lookup tables are defense in depth.

Early hardening should cover:

| Field(s) | Owner/value source |
|---|---|
| `tasks.mode`, `tasks.lifecycle_phase`, `tasks.result` | [Core Model Reference](core-model.md) |
| `surfaces.status`, `surfaces.guarantee_level`, `surfaces.local_access_posture` | [Agent Integration Reference](agent-integration.md), [Security Reference](security.md), and storage registration rules on this page |
| `change_units.status` | Core Model / Change Unit owner rules |
| `user_judgments.status`, `judgment_kind`, `presentation` | user-judgment API/Core owners |
| `write_authorizations.status` | [Core Model `prepare_write`](core-model.md#prepare_write), [`harness.prepare_write`](api/mvp-api.md#harnessprepare_write), and [`harness.record_run`](api/mvp-api.md#harnessrecord_run) |
| `runs.kind`, `runs.status` | [`harness.record_run`](api/mvp-api.md#harnessrecord_run) and storage compatibility notes |
| `artifacts.kind`, `artifacts.redaction_state`, `artifacts.retention_class`, `artifacts.status` | `ArtifactRef`/artifact owners and storage compatibility notes |
| `artifact_links.owner_record_kind`, `artifact_links.relation` | API `StateRecordRef`, `ArtifactInput.relation`, and storage owner-link rules |
| `evidence_summaries.status` | Core evidence gate and API evidence summary owners |
| `blockers.status`, `blocked_action`, `blocker_kind` | Core Model and API blocker owners |
| `task_events.event_type` | Core stable event semantics |
| `tool_invocations.status` | storage idempotency replay semantics |
| Future `projection_jobs.status`, `projection_jobs.projection_kind` | Projection/API owners when Operations Profile is active |
| Future `validator_runs.status` | `ValidatorResult` owner when an assurance or conformance profile is active |
| Future `evidence_manifests.status` | Evidence profile owner when full Evidence Manifest profile is active |

Storage-owned compatibility values promoted here:

| Field | Durable values | Meaning |
|---|---|---|
| `runs.status` | `completed`, `interrupted`, `blocked`, `violation` | A committed Run row. Only `completed` can support evidence through normal owner refs. Other values are audit/recovery records and do not satisfy evidence, QA, verification, acceptance, or close readiness by themselves. |
| `change_units.status` | `planned`, `active`, `completed`, `deferred`, `superseded` | Scope lifecycle. Only the active compatible scope row scopes new writes. |
| `user_judgments.status` | `proposed`, `pending_user`, `resolved`, `deferred`, `rejected`, `blocked`, `superseded` | User judgment lifecycle. A resolved judgment affects only the judgment type and payload it records. |
| `write_authorizations.status` | `active`, `consumed`, `expired`, `stale`, `revoked` | Durable authorization lifecycle, matching the Core/API owner value set. Only `active` and compatible rows can be consumed by `record_run`. |
| `artifacts.status` | `available`, `missing`, `stale`, `blocked` | Artifact availability. It is a storage and integrity fact, not full evidence sufficiency. |
| `evidence_summaries.status` | `not_required`, `none`, `partial`, `sufficient`, `stale`, `blocked` | Minimal evidence coverage state used by MVP-1 status and close. `sufficient` is required when evidence is close-required. |
| `blockers.blocker_kind` | `task`, `open_run`, `scope`, `user_judgment`, `sensitive_approval`, `design_policy`, `write_compatibility`, `baseline`, `surface_capability`, `evidence`, `artifact_availability`, `final_acceptance`, `residual_risk_visibility`, `residual_risk_acceptance`, `cancellation`, `supersession`, `recovery` | Active blocker categories used by status, write compatibility, run recording, close, and recovery. Close responses expose the close-category subset owned by the MVP API. Verification, Manual QA, projection/report freshness, export, operations, full Approval, full Residual Risk, Evidence Manifest, Eval, and detached-verification blocker categories are later/profile-only. |
| `blockers.status` | `open`, `resolved`, `superseded` | Stored blocker lifecycle. Open blockers remain visible until Core resolves or supersedes them. |
| `tool_invocations.status` | `committed` | A row exists only for a committed replayable non-dry-run response. |

`prepare_write.decision` is separate from the durable authorization lifecycle
column. The canonical `prepare_write.decision` values are `allowed`, `blocked`,
`approval_required`, `decision_required`, and `state_conflict`. Only non-dry-run
`decision=allowed` creates a durable authorization row; exact idempotent replay
returns the original committed response.

The new row starts with `write_authorizations.status=active`. `blocked` is not a
persisted Write Authorization lifecycle status. Non-dry-run `blocked`,
`approval_required`, and `decision_required` decisions are represented by the
response decision, blockers, validator findings, errors, and committed
idempotency replay state as applicable. `state_conflict` returns conflict state
without merging a new replay row. A `dry_run=true` `decision=allowed` response
may describe `authorization_effect=would_create`, but it does not insert a
`write_authorizations` row and cannot be consumed by `record_run`.

Future table value sets should be used only when the table's owner profile is
active, a fixture explicitly seeds that optional table, or the owner document
explicitly promotes the values.

## Initial Schema Boundary

No migration runner exists in this repository, and no runtime data exists to
migrate. This page does not define a strategy for migrating existing runtime
data. Before runtime implementation, maintainers must accept the actual DDL,
migration mechanism, storage profile, and any tightening behavior separately.

Future implementation planning should still preserve these storage rules:

- Record schema/profile version in `project_state` and Runtime Home metadata or
  an equivalent chosen mechanism.
- Validate JSON and owner-bound status values before tightening constraints.
- Preserve `task_events.event_seq` order when `task_events` is retained.
- Preserve artifact hashes and owner links, or mark affected refs invalid for
  recovery.
- Stop on unknown owner-bound enum/status values instead of inventing fallback
  meanings.
- Treat status cards, compact views, projection/card/job freshness, close
  readiness, and full report text as derived output, not canonical state.

## Lock Policy

Runtime mutations should serialize through the Core transaction order owned by
[Runtime Architecture](runtime-architecture.md#state-transaction-flow). The
active slice can use ordinary SQLite transactions plus a process/project lock if
needed. `persistent_locks` is a later Operations candidate, not an active
storage prerequisite.

Locks protect concurrent writes; they do not provide OS sandboxing, evidence
integrity, or tamper-proof storage.
