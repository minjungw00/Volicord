# Storage

This page is reference documentation for future Harness storage. It does not
mean this repository contains a Harness Server, Runtime Home, database,
artifact store, migration runner, generated projection, runtime state, or
implementation-complete DDL. Current repository status stays in
[MVP Plan](../build/mvp-plan.md#documentation-acceptance-status).

## 1. Owns / Does not own

This document owns the active current MVP persistence boundary:

- Runtime Home identity and project-local storage layout.
- Active persisted records and their table-level storage roles.
- Storage-owned JSON `TEXT` rules.
- Artifact persistence and artifact owner links.
- Event and idempotency storage meaning.
- State-version storage rules.
- Lock and migration boundaries.
- The line between active current MVP storage and later candidate storage.

This document does not own:

- Core lifecycle, gate, blocker, Write Authorization, `record_run`, or close
  semantics; see [Core Model Reference](core-model.md).
- Public MCP requests, responses, shared schemas, active enum values, errors, or
  replay behavior; see [MVP API](api/mvp-api.md),
  [API Schema Core](api/schema-core.md), and [API Errors](api/errors.md).
- Projection rendering, template bodies, report formats, dashboards, exports,
  reconcile behavior, operations entrypoints, conformance runners, or future
  fixture storage.
- OS permissions, sandboxing, tamper-proof files, pre-tool blocking, or
  security isolation claims; see [Security Reference](security.md).

Storage is the source of current Harness records only when rows are committed by
Core and validate against the owning Core/API/storage contracts. Chat, generated
Markdown, status cards, projections, connector output, operator output, and
report prose are not storage authority.

Storage defines where Harness records persist and how committed state
transitions are recorded. It does not claim tamper-proof storage, security
isolation, cryptographic evidence guarantee claims, or authority beyond the
Core/API/storage contracts.

## 2. Runtime Home identity

Harness uses one local Runtime Home and one project-local state database per
registered project. The default reference root is `~/.harness`; an
implementation may choose an equivalent configured root.

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

`registry.sqlite` stores the Runtime Home identity and minimal project
registration data. `project.yaml` stores static project configuration only.
`state.sqlite` stores project-local Core state. Artifact directories store
registered evidence bytes or safe metadata after Core applies the artifact
registration boundary.

`project.yaml` must not store current Task state, gates, Write Authorization
state, evidence sufficiency, final acceptance, residual-risk acceptance, or
close state.

Runtime Home identity must not depend only on a filesystem path. A copied or
moved Runtime Home may carry the same stored `runtime_home_id`; a newly created
Runtime Home gets a new id. The id helps detect suspicious copies, duplicate
registrations, or path drift. It does not make storage tamper-proof.

Runtime Home files are local operational control data and may contain sensitive
support data. Broad read access can expose secrets, PII, tokens, logs,
screenshots, diffs, and artifact content. Broad write access is a tampering and
evidence-poisoning risk. File permissions, owner checks, hashes, and diagnostics
are defensive checks; they do not create OS-level sandboxing, arbitrary-tool
control, tamper-proof storage, or pre-execution blocking.

## 3. Active persisted records

The active current MVP persists only the records needed by the active method set:
`harness.intake`, `harness.update_scope`, `harness.status`, `harness.prepare_write`,
`harness.record_run`, `harness.request_user_judgment`,
`harness.record_user_judgment`, and `harness.close_task`.

The active persisted records are:

- Runtime Home identity in `registry.sqlite`.
- Minimal project registration in `registry.sqlite`.
- Static project configuration in `project.yaml`.
- `project_state`.
- `surfaces`, limited to the registered local/reference surface facts needed by
  the active API envelope, capability display, and local-access posture.
- `tasks`.
- `change_units`.
- `user_judgments`.
- `write_authorizations`.
- `runs`.
- `artifacts`.
- `artifact_links`.
- `evidence_summaries`.
- `blockers`.
- `task_events`.
- `tool_invocations`.

No other persisted table family is active current MVP scope. Requirement
shaping persists through `tasks`, `change_units`, `user_judgments`,
`evidence_summaries`, and `blockers`; it is not a separate committed Discovery
Brief, Shared Design, Question Queue, Assumption Register, or First Safe Change
Unit Candidate table. Evidence persists through compact evidence summaries and
artifact refs, not through full Evidence Manifest storage.

The minimum active shaping information is stored through those existing records:
current goal summary, active scope summary, allowed paths or affected areas,
non-goals, acceptance criteria, Autonomy Boundary, required user-owned
judgments, one blocking question when necessary, one next safe action, evidence
expectation or evidence gap, and close blockers. Missing or unknown pieces stay
as `unknown`, pending `user_judgments`, evidence gaps, or `blockers`; storage
must not create extra active planning tables to make the request appear ready.

## 4. Tables

The table below names the active storage tables and the minimum storage role
they serve. It is not full DDL and does not duplicate API schemas.

| Table or file | Location | Active role | Essential stored fields |
|---|---|---|---|
| Runtime Home identity | `registry.sqlite` | Identify the local Runtime Home and schema/storage profile. | `runtime_home_id`, `schema_version`, `storage_profile`, `created_at`, `updated_at`. |
| Project registration | `registry.sqlite` | Map a registered project to its project-local storage. | `project_id`, `repo_root`, `project_home`, `display_name`, `status`, `created_at`, `updated_at`. |
| `project.yaml` | Project directory | Static project configuration. | `project_id`, `repo_root`, display/config defaults. |
| `project_state` | `state.sqlite` | Project-local state header, state clock, active Task pointer, and default surface pointer. | `project_id`, `schema_version`, `storage_profile`, `state_version`, `active_task_id`, `default_surface_id`, `created_at`, `updated_at`. |
| `surfaces` | `state.sqlite` | Registered local/reference surface facts for `surface_id`, capability profile, local access posture, and guarantee display. | `surface_id`, `project_id`, `surface_kind`, `capability_profile_json`, `local_access_posture`, `guarantee_level`, `status`, `created_at`, `updated_at`. |
| `tasks` | `state.sqlite` | User-value work unit, task-scoped state clock, current shaping summary, lifecycle, result, next-action, and close fields. | `task_id`, `project_id`, `title`, `user_request`, `current_goal_summary`, `mode`, `lifecycle_phase`, `close_reason`, `result`, `summary`, shaping JSON columns, `blocking_question`, `next_safe_action`, `active_change_unit_id`, `state_version`, `created_at`, `updated_at`, `closed_at`. |
| `change_units` | `state.sqlite` | Current or proposed scoped work boundary for write compatibility and close basis. | `change_unit_id`, `task_id`, `scope_summary`, scope JSON columns for allowed paths or affected areas, `baseline_ref`, `autonomy_boundary_json`, `status`, `created_at`, `updated_at`. |
| `user_judgments` | `state.sqlite` | User-owned judgment records for the active `UserJudgment.judgment_kind` values. | `user_judgment_id`, `task_id`, `change_unit_id`, `judgment_kind`, `presentation`, `status`, request/context JSON columns, `question`, `resolution_json`, `expires_at`, `resolved_at`, `created_at`, `updated_at`. |
| `write_authorizations` | `state.sqlite` | Durable single-use cooperative Write Authorization created only by non-dry-run `prepare_write` with `decision=allowed`. | `write_authorization_id`, `task_id`, `change_unit_id`, `surface_id`, `status`, `basis_state_version`, `attempt_scope_json`, `consumed_by_run_id`, `expires_at`, `created_at`, `updated_at`, `consumed_at`. |
| `runs` | `state.sqlite` | Committed execution or observation record, including compatible authorization consumption when a product write happened. | `run_id`, `task_id`, `change_unit_id`, `write_authorization_id`, `surface_id`, `kind`, `status`, `product_write`, `baseline_ref`, `summary`, observed/evidence JSON columns, `created_at`, `completed_at`. |
| `artifacts` | `state.sqlite` plus artifact store | Registered durable evidence bytes or safe metadata with integrity, redaction, producer, retention, and availability facts. | `artifact_id`, `project_id`, `task_id`, `run_id`, `kind`, `uri`, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `retention_class`, `produced_by`, `status`, `created_at`, `updated_at`. |
| `artifact_links` | `state.sqlite` | Owner relation from an artifact to the active Core/API record it supports. | `artifact_link_id`, `artifact_id`, `task_id`, `owner_record_kind`, `owner_record_id`, `relation`, `created_at`. |
| `evidence_summaries` | `state.sqlite` | Compact evidence coverage and gap record used by status, run/evidence summaries, blockers, and close. | `evidence_summary_id`, `task_id`, `change_unit_id`, `status`, `coverage_items_json`, `summary`, `supporting_run_ids_json`, `supporting_artifact_link_ids_json`, `gap_blocker_ids_json`, `updated_at`. |
| `blockers` | `state.sqlite` | Structured blocker for next action, write compatibility, evidence gaps, close readiness, or recovery. | `blocker_id`, `task_id`, `blocked_action`, `blocker_kind`, `status`, `message`, `owner_ref_json`, `related_refs_json`, `required_next_action`, `created_at`, `resolved_at`. |
| `task_events` | `state.sqlite` | Append-only audit and ordering trail for committed Core mutations. | `event_id`, `task_id`, `event_seq`, `event_type`, `state_version`, `actor_kind`, `surface_id`, `payload_json`, `created_at`. |
| `tool_invocations` | `state.sqlite` | Committed idempotency replay row for non-dry-run state-changing tool responses. | `invocation_id`, `project_id`, `tool_name`, `idempotency_key`, `request_hash`, `task_id`, `basis_state_version`, `response_json`, `status`, `created_at`. |

After intake, `harness.update_scope` owns committed updates to active Task scope
fields such as goal summary, scope boundary, non-goals, acceptance criteria,
Autonomy Boundary, baseline reference, and `tasks.active_change_unit_id`. It may
create or replace the active `change_units` row for the Task. A resolved
`scope_decision` in `user_judgments` may be linked as a related ref, but
`harness.record_user_judgment` does not directly update those active scope or
Change Unit fields.

At the storage contract level, `change_units.status` distinguishes only
`proposed`, `active`, `replaced`, and `closed`. `proposed` may hold an intake or
clarification candidate, `active` is the current write-compatibility and close
basis for the Task, `replaced` is no longer the active basis after
`harness.update_scope`, and `closed` is no longer active because the owning Task
closed, cancelled, or was superseded. This is a storage value boundary, not full
DDL or a migration recipe.

When `harness.update_scope` changes the active scope, active Change Unit,
baseline, or Autonomy Boundary, any active `write_authorizations` that no longer
match the current basis are marked `status=stale`. Stale authorizations remain
records but cannot be consumed by `harness.record_run`; the caller must get a
fresh compatible `harness.prepare_write` result for the exact operation.

`surfaces` is not a connector marketplace or broad connector ecosystem table.
It is the active local/reference surface registration needed to interpret
`surface_id`, capability, local access posture, and guarantee display.

`surfaces.local_access_posture` is a closed current MVP value set:

| Value | Storage meaning |
|---|---|
| `registered_local` | The stored surface registration can be used as the registered local posture for current API compatibility checks. |
| `unavailable` | Required MCP/Core or surface reachability cannot currently be established from this registration. |
| `mismatch` | The observed caller or transport does not match the stored registered local posture. |
| `revoked` | Local access for this registration was explicitly revoked and cannot be used. |

`surfaces.status` is a closed current MVP value set:

| Value | Storage meaning |
|---|---|
| `active` | The stored surface row may be used by current API access checks. |
| `disabled` | The row is retained but must not be used for current API access. |
| `stale` | The row requires refresh before current API access can rely on it. |
| `revoked` | The surface registration is no longer valid for current API access. |

Unknown `surfaces.local_access_posture` or `surfaces.status` values are invalid. State-changing API calls require `surfaces.status=active` and `surfaces.local_access_posture=registered_local` before commit. Read-only status paths may return display-safe diagnostics for unavailable, mismatched, stale, disabled, or revoked surfaces, but they must not turn those diagnostics into Core state or expose artifact content.

`display_label` is not an active storage identity column. Display labels are
derived from stable identifiers such as `judgment_kind` and locale.

`tasks.lifecycle_phase`, `tasks.close_reason`, and `tasks.result` store separate
Core concepts. `tasks.lifecycle_phase` must not store `intake`; terminal
lifecycle values are `completed`, `cancelled`, and `superseded`.
`tasks.result` must not store `failed`; failed Runs, projections, artifacts,
validators, evidence gaps, and close blockers remain in their owning records.
When committed supersession changes the active pointer, `project_state.active_task_id`
must follow the `harness.close_task` `superseding_task_id` rule and must not
continue pointing at the superseded Task.

## 5. JSON TEXT columns

SQLite `TEXT` columns that store JSON are a storage representation choice, not
permission to persist arbitrary JSON. Core must parse and validate JSON before
commit.

API-shaped stored JSON validates against [MVP API](api/mvp-api.md) and
[API Schema Core](api/schema-core.md). Storage-only JSON validates against this
page or the owner document named by this page. SQLite defaults such as `'{}'`
and `'[]'` are storage defaults only; they do not make API fields optional.

Active JSON `TEXT` columns are limited to compact owner-shaped data needed by
the active records, including:

- `surfaces.capability_profile_json`.
- Task and Change Unit shaping columns such as `success_criteria_json`,
  `acceptance_criteria_json`, `scope_boundary_json`, `non_goals_json`,
  `affected_areas_json`, `affected_path_candidates_json`, `constraints_json`,
  and `autonomy_boundary_json`.
- `user_judgments` request, context, option, affected-ref, artifact-ref, and
  `resolution_json` columns.
- `write_authorizations.attempt_scope_json`, which stores
  `AuthorizedAttemptScope`.
- `runs` observed-attempt and evidence-update JSON columns.
- `evidence_summaries.coverage_items_json` and supporting/gap ref arrays.
- `blockers.owner_ref_json` and `blockers.related_refs_json`.
- `task_events.payload_json`.
- `tool_invocations.response_json`.

Task and Change Unit shaping JSON stores compact summaries and bounded lists
only. It must not store a standalone Discovery Brief, Question Queue,
Assumption Register, full design artifact, generated projection body, evidence
manifest body, QA record, acceptance record, residual-risk record, or close
record under another name.

Status-like `TEXT` values are closed owner value sets, not open strings. Active
values are owned by Core/API owners and by the storage notes here. Defensive
`CHECK` constraints or lookup tables may be used, but Core validation remains
required.

## 6. Artifact references

`ArtifactRef` is the public API shape for registered durable evidence bytes or
safe metadata. Storage implements it through `artifacts` plus `artifact_links`;
see [API Schema Core: ArtifactRef](api/schema-core.md#artifactref).

Artifact registration accepts only the owner-documented `ArtifactInput`
sources: `staged_file`, `capture_adapter`, or `existing_artifact`. A staged or
captured handle must be resolved by the owner path before storage commits the
artifact row. An `existing_artifact` input must name an already registered
`ArtifactRef` that belongs to the same project and has a compatible owner
relation. Caller-supplied raw filesystem paths are not registration authority.

An artifact is evidence-eligible only when storage has:

- registered bytes or a safe metadata notice under the artifact store,
- integrity facts such as `sha256`, `size_bytes`, and `content_type`,
- a `redaction_state`,
- producer and retention facts,
- an availability `status`,
- and an owner link to an active record such as `task`, `change_unit`, `run`,
  `user_judgment`, `evidence_summary`, or `blocker`.

`sha256`, `size_bytes`, and `content_type` are artifact integrity facts for
comparison and availability handling. They do not make artifact storage
tamper-proof or create a cryptographic evidence guarantee claim.

`uri` resolves through Harness storage, normally as
`harness-artifact://{project_id}/{artifact_id}`. It is not a caller-supplied
arbitrary filesystem path. Raw secrets, tokens, and full sensitive logs must not
be stored as evidence bytes. Store redacted bytes, `secret_omitted` or `blocked`
notices, safe handles, or other owner-approved safe representations instead.

Raw artifact path reads are not granted by default. Artifact metadata or content
reads require a registered `ArtifactRef`, the matching same-project `task_id`,
the required `artifact_links` owner relation, and the redaction/availability
state needed by the caller's access class. A local path under the artifact store,
an artifact `uri`, or a copied file is not enough by itself to read or rely on
artifact bytes.

An artifact link does not create the owner record, satisfy a gate by itself,
prove evidence sufficiency, perform QA, create final acceptance, accept
residual risk, or close a Task.

## 7. Idempotency and event meaning

`task_events` records committed Core mutations in order. It is an audit and
ordering trail, not the normal source used to reconstruct current state during
ordinary operation. Current rows such as `tasks`, `change_units`,
`user_judgments`, `write_authorizations`, `runs`, `artifacts`,
`artifact_links`, `evidence_summaries`, and `blockers` remain the current state.

`tool_invocations` stores exact replay for committed non-dry-run state-changing
responses. Keys are scoped as described by [API Errors: Idempotency](api/errors.md#idempotency).
If the same key and request hash are replayed, Core returns the original
committed response without appending events, registering artifacts, consuming
authorization, or changing state again. If the key is reused with a different
request hash, Core returns the API-owned state conflict behavior.

Dry runs, malformed requests, pre-commit validation failures, pre-commit state
conflicts, and rejected `record_run` attempts that create no mutation do not
create current rows, `task_events`, artifacts, evidence summaries, Write
Authorizations, close state, or `tool_invocations` replay rows.

A blocked response may persist only the blocker or other mutation the method
owner allows. It must not create the authority the blocker says is missing. For
example, blocked `prepare_write` responses do not create consumable
`write_authorizations`.

## 8. State versioning

State clocks are scoped. Task-scoped mutations increment
`tasks.state_version`. Project-scoped mutations with no Core-selected primary
Task increment `project_state.state_version`.

State-changing API calls compare `ToolEnvelope.expected_state_version` against
the affected scope before committing. The response `ToolResponseBase.state_version`
is the resulting affected-scope version for a committed mutation, or the current
readable/would-be affected version for read-only and dry-run responses.

`write_authorizations.basis_state_version` stores the state version used when
Core allowed the attempt. `write_authorizations.attempt_scope_json` stores the
authorized attempt boundary that `record_run` later compares against observed
facts. The top-level `task_id`, `change_unit_id`, `surface_id`, and
`basis_state_version` columns are query fields; the stored attempt scope remains
the compatibility boundary.

`tool_invocations.basis_state_version` stores the affected-scope version used as
the compatibility basis before the committed mutation. `task_events.state_version`
stores the resulting affected-scope version for the committed event.

## 9. Lock policy

Runtime mutations serialize through Core-owned state-changing paths, with
ordinary SQLite transactions and a process/project lock if needed. The
authority placement is owned by [Runtime Boundaries Reference](runtime-boundaries.md).

The active current MVP does not require a `persistent_locks` table. Durable
lock/recovery metadata is later operations material until an owner
promotes it.

Locks protect concurrent state writes. They do not provide OS sandboxing,
artifact-integrity enforcement, tamper-proof storage, permission isolation, or
pre-tool blocking.

## 10. Migration boundary

No migration runner exists in this repository, and no runtime data exists to
migrate. This page does not define migration steps for existing runtime data.
Before runtime implementation, maintainers must separately accept the actual
DDL, migration mechanism, storage profile, and tightening behavior.

The active migration boundary is:

- Store schema/profile version in Runtime Home metadata and `project_state`, or
  an equivalent maintainer-accepted mechanism.
- Validate owner-shaped JSON before commit and before tightening constraints.
- Treat unknown owner-bound status or enum values as invalid until an owner
  defines them.
- Preserve `task_events.event_seq` ordering when `task_events` is retained.
- Preserve artifact hashes and owner links, or mark affected refs invalid for
  recovery.
- Keep status cards, compact views, projection freshness, close readiness, and
  report prose derived from current records. They are not migration authority.

This page intentionally excludes inactive DDL bundles, migration catalogs, and
profile-specific migration details.

## 11. Later storage excluded from active current MVP

Profile-gated later storage is outside the active current MVP unless an owner
document promotes a narrow behavior with scope, fallback behavior, and
proof-path expectations for future promotion. Reference-schema presence alone
does not make storage active.

The active current MVP excludes storage for:

- projection jobs, durable projection caches, managed-output outboxes, and
  projection dashboards;
- validator-run records, conformance-runner state, fixture execution history,
  and generated conformance artifacts;
- operations-profile storage for doctor suites, recover, export, release
  handoff, artifact dashboards, reconcile queues, or operational reports;
- full Evidence Manifest tables, detailed evidence catalogs, detached Eval,
  detached verification, full Manual QA matrices, and rich QA/waiver machinery;
- rich Approval tables and rich residual-risk lifecycle tables separate from
  `user_judgments` and `blockers`;
- dashboard, metrics, analytics, team workflow, hosted connector registry,
  connector marketplace, connector analytics, and cross-surface orchestration
  storage;
- Shared Design, Journey/Spine, Domain Language, Module Map, Interface Contract,
  stewardship, and long-term design-support storage.

Active status, close readiness, run/evidence summaries, next actions, readable
cards, and guarantee display are derived from the active persisted records above.
They may be stale, absent, failed, or recomputed without changing storage
authority.
