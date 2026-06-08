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
  reconcile behavior, persistent projection jobs, reconcile queues, managed
  block drift repair, operations entrypoints, conformance runners, or future
  fixture storage.
- OS permissions, sandboxing, tamper-proof files, pre-tool blocking, or
  security isolation claims; see [Security Reference](security.md).

Storage is the source of current Harness records only when rows are committed by
Core and validate against the owning Core/API/storage contracts. Chat, generated
Markdown, status cards, projections, connector output, operator output, and
report prose are not storage authority.

Read-time status/projection output is derived from committed records and
registered artifact refs. Editing a rendered projection, Markdown status card,
or generated document does not update storage or mutate Core state. The active
current MVP has no `projection_jobs` table, durable projection cache, reconcile
queue, managed-output outbox, or managed block drift-repair storage.

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

The path meanings are part of the storage contract:

- The Runtime Home root, shown as `~/.harness`, is Harness operational data. It
  is not the Product Repository and not a grant of filesystem permission.
- `registry.sqlite` stores Runtime Home identity and minimal project
  registration. It is the Runtime Home registry, not project-local Task state.
- A project directory under `projects/{project_id}/` is the Harness project
  home for one registered project. It is not the same thing as `repo_root`.
- `project.yaml` stores static project configuration only.
- `state.sqlite` stores project-local Core state for the registered project.
- `artifacts/` is the project artifact store. Paths below it store registered
  evidence bytes or safe metadata only after Core applies the artifact
  registration boundary. `artifacts/tmp/` is staging space, not evidence
  authority.

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

The active current MVP persists only the Core records needed by the active
state-changing method set: `harness.intake`, `harness.update_scope`,
`harness.prepare_write`, `harness.record_run`,
`harness.request_user_judgment`, `harness.record_user_judgment`, and
`harness.close_task`. `harness.status` and `harness.close_task intent=check`
are read-only. `harness.stage_artifact` is an active local artifact utility,
but it creates only temporary storage-owned staging bytes or notices plus an
`artifact_staging` row or equivalent staging manifest behind a
`StagedArtifactHandle`; it does not create a current Core authority record or
increment the project state clock.

The active Core persisted records are:

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

The active temporary storage boundary is `artifact_staging` or an equivalent
storage-owned staging manifest, together with safe temporary bytes or notices
under `artifacts/tmp/`. It exists only to let `harness.stage_artifact` produce a
scoped, expiring, single-consumption handle with server-recorded creation
surface provenance for later `harness.record_run` promotion. It is not a
persistent `ArtifactRef`, evidence authority, a Core mutation record, an event
source, a replay row, or a state-version source.

No other persisted table family or temporary handle family is active current MVP
scope. Requirement shaping persists through `tasks`, `change_units`,
`user_judgments`, `evidence_summaries`, and `blockers`; it is not a separate
committed Discovery Brief, Shared Design, Question Queue, Assumption Register,
or First Safe Change Unit Candidate table. Evidence persists through compact evidence summaries,
`CompletionPolicy` on the Task or Change Unit, required coverage items, and
artifact refs, not through full Evidence Manifest storage.

Projection has no active persisted table family. `status-card`,
`judgment-request`, `run-evidence-summary`, `close-result`, and
`agent-context-packet` are read-time views over the active records, not stored
state or storage mutation paths.

The minimum active shaping information is stored through those existing records:
current goal summary, active scope summary, allowed paths or affected areas,
non-goals, acceptance criteria, Autonomy Boundary, required user-owned
judgments, one blocking question when necessary, one next safe action,
`CompletionPolicy`, required evidence expectation or evidence gap, and close
blockers. Missing or unknown pieces stay
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
| `project_state` | `state.sqlite` | Project-local state header, single public project-wide state clock, active Task pointer, and default surface pointer. | `project_id`, `schema_version`, `storage_profile`, `state_version`, `active_task_id`, `default_surface_id`, `created_at`, `updated_at`. |
| `surfaces` | `state.sqlite` | Stored `LocalSurfaceRegistration` facts used to verify a local surface context for API access. The row is registration data, not live proof that the current caller is trusted. | `project_id`, `surface_id`, `surface_instance_id`, `transport_kind`, `transport_binding_fingerprint`, `access_secret_hash`, `capability_profile_hash`, `capability_profile_json`, `status`, `local_access_posture`, `registered_at`, `last_verified_at`, `updated_at`. |
| `tasks` | `state.sqlite` | User-value work unit, current shaping summary, lifecycle, result, next-action, active Task-level `CompletionPolicy`, and close fields. | `task_id`, `project_id`, `title`, `user_request`, `current_goal_summary`, `mode`, `lifecycle_phase`, `close_reason`, `result`, `summary`, shaping JSON columns, `completion_policy_json`, `blocking_question`, `next_safe_action`, `active_change_unit_id`, `created_at`, `updated_at`, `closed_at`. |
| `change_units` | `state.sqlite` | Current or proposed scoped work boundary for write compatibility, Change Unit-level `CompletionPolicy`, and close basis. | `change_unit_id`, `task_id`, `scope_summary`, scope JSON columns for allowed paths or affected areas, `baseline_ref`, `autonomy_boundary_json`, `completion_policy_json`, `status`, `created_at`, `updated_at`. |
| `user_judgments` | `state.sqlite` | User-owned judgment records for the active `UserJudgment.judgment_kind` values, including separate sensitive-action approval scope when relevant. | `user_judgment_id`, `task_id`, `change_unit_id`, `judgment_kind`, `presentation`, `status`, request/context JSON columns, `question`, `sensitive_action_scope_json`, `resolution_json`, `expires_at`, `resolved_at`, `created_at`, `updated_at`. |
| `write_authorizations` | `state.sqlite` | Durable single-use cooperative Write Authorization created only by non-dry-run `prepare_write` with `decision=allowed`. | `write_authorization_id`, `task_id`, `change_unit_id`, `surface_id`, `status`, `basis_state_version`, `attempt_scope_json`, `consumed_by_run_id`, `expires_at`, `created_at`, `updated_at`, `consumed_at`. |
| `runs` | `state.sqlite` | Committed execution or observation record, including compatible authorization consumption when a product write happened. | `run_id`, `task_id`, `change_unit_id`, `write_authorization_id`, `surface_id`, `kind`, `status`, `product_write`, `baseline_ref`, `summary`, observed/evidence JSON columns, `created_at`, `completed_at`. |
| `artifact_staging` | `state.sqlite` plus `artifacts/tmp/` | Temporary staged safe bytes or safe notices created by `harness.stage_artifact` for later single-use `harness.record_run` consumption. It is not a persistent `ArtifactRef` and not evidence authority. | `handle_id`, `project_id`, `task_id`, `created_by_surface_id`, `created_by_surface_instance_id`, `display_name`, `relation_hint`, `tmp_uri`, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `status`, `consumed_by_run_id`, `promoted_artifact_id`, `expires_at`, `created_at`, `consumed_at`. |
| `artifacts` | `state.sqlite` plus artifact store | Registered durable evidence bytes or safe metadata with integrity, redaction, producer, retention, and availability facts. | `artifact_id`, `project_id`, `task_id`, `run_id`, `kind`, `uri`, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `retention_class`, `produced_by`, `status`, `created_at`, `updated_at`. |
| `artifact_links` | `state.sqlite` | Owner relation from an artifact to the active Core/API record it supports. | `artifact_link_id`, `artifact_id`, `task_id`, `owner_record_kind`, `owner_record_id`, `relation`, `created_at`. |
| `evidence_summaries` | `state.sqlite` | Compact evidence coverage and gap record used by status, run/evidence summaries, blockers, and close. It is evaluated against the Task or Change Unit `CompletionPolicy`; it does not own that policy. | `evidence_summary_id`, `task_id`, `change_unit_id`, `status`, `coverage_items_json`, `summary`, `supporting_run_ids_json`, `supporting_artifact_link_ids_json`, `gap_blocker_ids_json`, `updated_at`. |
| `blockers` | `state.sqlite` | Structured blocker for next action, write compatibility, evidence gaps, close readiness, or recovery. | `blocker_id`, `task_id`, `blocked_action`, `blocker_kind`, `status`, `message`, `owner_ref_json`, `related_refs_json`, `required_next_action`, `created_at`, `resolved_at`. |
| `task_events` | `state.sqlite` | Append-only audit and ordering trail for committed Core mutations. | `event_id`, `project_id`, `task_id`, `event_seq`, `event_type`, `state_version`, `actor_kind`, `surface_id`, `payload_json`, `created_at`. |
| `tool_invocations` | `state.sqlite` | Committed replay row for non-dry-run state-changing tool responses. | `invocation_id`, `project_id`, `tool_name`, `idempotency_key`, `request_hash`, `task_id`, `basis_state_version`, `response_json`, `status`, `created_at`. |

### First schema integrity contract

This section is a first-implementation storage contract for future SQLite
schema design. It is not full DDL, a migration file, or proof that runtime
implementation has started.

For a first SQLite schema, the subsections below are minimum persistence
constraints. Implementations may choose `CHECK` constraints, lookup tables,
generated columns, triggers, or Core-side validation, but committed rows must
preserve these identities, value sets, relations, and transaction boundaries.
If this page and an API/Core owner disagree about a public schema value or
method effect, the owner documents must be corrected before DDL is accepted.

Required identity and uniqueness constraints:

- Active tables use opaque stable ids as primary keys or equivalent unique
  keys: `project_id`, `surface_id`, `surface_instance_id`, `task_id`,
  `change_unit_id`, `user_judgment_id`, `write_authorization_id`, `run_id`,
  `handle_id`, `artifact_id`, `artifact_link_id`, `evidence_summary_id`,
  `blocker_id`, `event_id`, and `invocation_id`.
- Runtime Home identity stores one `runtime_home_id` for the Runtime Home.
- Project registration requires unique `project_id`, unique `project_home`, and
  one active registration for a `repo_root` unless a future owner defines
  multi-registration behavior.
- `project_state.project_id` is one row per registered project.
- `surfaces` requires a unique `(project_id, surface_id)`. The stored
  `surface_instance_id` identifies the registered local instance selected by
  that surface row; it must match the server-derived verified context before a
  request may rely on the surface.
- `tasks` requires a unique `(project_id, task_id)`.
- `change_units` requires unique `(task_id, change_unit_id)` and at most one
  `status=active` Change Unit per Task.
- `write_authorizations.consumed_by_run_id`, when non-null, is unique.
  `runs.write_authorization_id`, when non-null, is unique. Together these
  preserve the single-use consumption relation.
- `artifact_staging` requires a unique `(project_id, handle_id)` and a unique
  `tmp_uri` or equivalent staging-object identity within the project while the
  handle exists. `consumed_by_run_id` and `promoted_artifact_id`, when non-null,
  are unique so one staged handle cannot be consumed twice or promoted into two
  durable artifacts.
- `artifact_links` requires a uniqueness rule equivalent to
  `(artifact_id, owner_record_kind, owner_record_id, relation)` so the same
  owner relation is not duplicated.
- `artifacts.uri`, when stored rather than derived, must be unique within the
  project and must resolve to the same `artifact_id`.
- `task_events` requires unique `event_id` and monotonic unique `event_seq`
  within the affected scope: `(project_id, task_id, event_seq)` for Task-scoped
  events, and `(project_id, event_seq)` when `task_id` is null for
  project-scoped events.
- `tool_invocations` requires a unique replay key on
  `(project_id, tool_name, idempotency_key)`. `request_hash` is stored in the
  row and must not be added to a second uniqueness key that would allow the same
  idempotency key to fork into multiple committed responses.

Main foreign key relationships:

- `project_state.project_id`, `surfaces.project_id`, `tasks.project_id`,
  `artifact_staging.project_id`, `artifacts.project_id`, `task_events.project_id`, and
  `tool_invocations.project_id` belong to project registration.
- `project_state.active_task_id`, when present, points to an open same-project
  `tasks` row. `project_state.default_surface_id`, when present, points to a
  same-project `surfaces` row.
- `tasks.active_change_unit_id` points to a `change_units` row for the same
  Task, and may be null while the Task is still shaping or not write-capable.
- `change_units.task_id`, `user_judgments.task_id`,
  `write_authorizations.task_id`, `runs.task_id`,
  `evidence_summaries.task_id`, `blockers.task_id`, and Task-scoped
  `task_events.task_id` point to `tasks`.
- `user_judgments.change_unit_id`, `write_authorizations.change_unit_id`,
  `runs.change_unit_id`, and `evidence_summaries.change_unit_id`, when present,
  point to a `change_units` row for the same Task.
- `write_authorizations.surface_id` and `runs.surface_id` point to a same-project
  `surfaces` row.
- `runs.write_authorization_id`, when present, points to the consumed
  `write_authorizations` row and must match the same Task, Change Unit, surface,
  and compatible attempt scope.
- `artifact_staging.task_id` points to the owning Task.
  `artifact_staging.consumed_by_run_id` and
  `artifact_staging.promoted_artifact_id`, when present, point to same-project
  same-Task `runs` and `artifacts` rows created by the consuming
  `harness.record_run` transaction.
- `artifacts.task_id` points to the owning Task. `artifacts.run_id`, when
  present, points to a same-Task `runs` row.
- `artifact_links.artifact_id` points to `artifacts`, and
  `artifact_links.task_id` points to the same Task as the artifact and owner
  relation.
- `tool_invocations.task_id`, when present, points to a same-project `tasks`
  row for the affected Task.
- JSON ref arrays such as `supporting_run_ids_json`,
  `supporting_artifact_link_ids_json`, `gap_blocker_ids_json`,
  `related_refs_json`, and `response_json` cannot be raw unvalidated text refs.
  They must be parsed and checked against the same project/task and owner
  relation before commit, even where SQLite cannot express the relation as a
  direct foreign key.

Cascade delete policy:

- Ordinary active MVP Core operations do not hard-delete authority rows. They
  move rows through status or lifecycle fields, append events, and keep replay
  and artifact metadata available for audit and recovery.
- Foreign keys should default to `RESTRICT` or equivalent owner validation for
  authority rows. Closing, cancelling, or superseding a Task must not cascade
  delete `tasks`, `change_units`, `user_judgments`, `write_authorizations`,
  `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers`,
  `task_events`, or `tool_invocations`.
- Unconsumed or expired `artifact_staging` rows and `artifacts/tmp/` staging
  bytes or notices may be marked `expired` or `discarded`, and temporary bytes
  may be cleaned before registration, because they are not evidence authority.
  Once an `artifacts` row is committed, retention purge, project teardown, or
  destructive cleanup is outside ordinary active MVP mutation behavior and needs
  an owner-defined path.
- A future retention or migration path must preserve artifact hashes, owner
  links, events, and replay rows, or mark affected refs invalid for recovery. It
  must not silently cascade-delete evidence support that current records still
  name.

Closed current MVP storage value sets are table-level persistence constraints.
Rows that mirror Schema Core values must match Schema Core exactly; rows marked
storage-owned below define storage behavior that is not a public API schema
body. Unknown values fail before commit.

| Field | Current MVP values | Storage rule |
|---|---|---|
| Project registration `status` | `active` | Only registered active projects are in the baseline current MVP. Disable/unregister behavior is later until promoted. |
| `surfaces.transport_kind` | `local_mcp_stdio`, `local_http` | Stored local transport category for registration matching. It is not a socket or protocol setup specification. |
| `surfaces.local_access_posture` | `registered_local`, `unavailable`, `mismatch`, `revoked` | Stored registration posture for API compatibility checks; meanings are below and mirror `LocalSurfaceRegistration.local_access_posture` in Schema Core. |
| `surfaces.status` | `active`, `disabled`, `stale`, `revoked` | Stored surface registration usability; meanings are below and mirror `LocalSurfaceRegistration.status` in Schema Core. |
| `tasks.lifecycle_phase` | `shaping`, `ready`, `executing`, `waiting_user`, `blocked`, `completed`, `cancelled`, `superseded` | Persisted Task lifecycle. `intake` is not a stored value; `superseded` is terminal. |
| `tasks.close_reason` | `none`, `completed_self_checked`, `completed_with_risk_accepted`, `cancelled`, `superseded` | Persisted close detail, separate from lifecycle and result. |
| `tasks.result` | `none`, `advice_only`, `completed`, `cancelled`, `superseded` | Persisted coarse outcome. Failed Runs, violations, blocked closes, and evidence gaps stay in their owning records. |
| `change_units.status` | `proposed`, `active`, `replaced`, `closed` | Storage-owned active Change Unit lifecycle for write compatibility and close basis. |
| `write_authorizations.status` | `active`, `consumed`, `expired`, `stale`, `revoked` | Durable authorization lifecycle. Schema Core exposes the same public summary values; storage owns persistence and transition rules. |
| `artifact_staging.status` | `staged`, `consumed`, `expired`, `discarded` | Storage-owned temporary handle lifecycle. Only `staged` is consumable by `harness.record_run`; terminal values cannot return to `staged`. |
| `artifact_staging.redaction_state` | `none`, `redacted`, `secret_omitted`, `blocked` | Temporary staging uses the same safe-byte or safe-notice redaction vocabulary as `ArtifactRef`; it does not describe a hidden original. |
| `artifacts.status` | `available`, `missing`, `integrity_failed`, `unavailable` | Storage-owned artifact availability state. Redaction and blocked-payload handling stay in `redaction_state`. |
| `artifacts.redaction_state` | `none`, `redacted`, `secret_omitted`, `blocked` | Persisted `ArtifactRef.redaction_state` values from Schema Core. Hash and size describe the committed safe bytes or safe notice, not a hidden original. |
| `artifact_links.owner_record_kind` | `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` | Persisted owner relation discriminator. Values mirror `ArtifactRelationOwner.record_kind`; storage owns same-project/same-Task owner lookup and relation validation. |
| `blockers.status` | `active`, `resolved`, `superseded` | Storage-owned blocker row state. Public close blocker shapes remain API-owned. |
| `tool_invocations.status` | `committed` | A replay row exists only for a committed non-dry-run state-changing response. Dry-run and pre-commit failures have no replay row. |

Other persisted status-like API fields, including `tasks.mode`, `runs.kind`,
`runs.status`, `user_judgments.status`, and `evidence_summaries.status`, validate
against [API Schema Core](api/schema-core.md#current-mvp-value-sets) and the
Core/API method owners. Storage may index and constrain them, but this page does
not redefine their public schema values.

After intake, `harness.update_scope` owns committed updates to active Task scope
fields such as goal summary, scope boundary, non-goals, acceptance criteria,
Autonomy Boundary, baseline reference, and `tasks.active_change_unit_id`. It may
create or replace the active `change_units` row for the Task. A resolved
`scope_decision` in `user_judgments` may be linked as a related ref, but
`harness.record_user_judgment` does not directly update those active scope or
Change Unit fields.

`change_units.status` transitions are closed:

- New write-capable intake or clarification candidates may start as `proposed`.
- `proposed` may become `active` only through the owner path that makes it the
  Task's current write-compatibility basis.
- `proposed` may become `replaced` if a different candidate supersedes it before
  activation.
- `active` becomes `replaced` when `harness.update_scope` selects a different
  active Change Unit for the Task.
- Any non-`closed` Change Unit may become `closed` when the owning Task is
  completed, cancelled, or superseded.
- `closed` is terminal for active MVP storage.

`write_authorizations.status` transitions are closed:

- A non-dry-run `harness.prepare_write` with `decision=allowed` creates exactly
  one `status=active` row. `active` is the durable open state; there is no stored
  `open` value.
- `active` becomes `consumed` only when a compatible `harness.record_run`
  consumes it. Storage then sets `consumed_by_run_id` and `consumed_at`.
- `active` becomes `stale` when `harness.update_scope` or another owner path
  changes the active Task, Change Unit, baseline, scope boundary, acceptance
  basis, Autonomy Boundary, or state version so the authorization no longer
  matches the current basis.
- `active` becomes `revoked` only through an explicit owner path that invalidates
  the authorization without consuming it.
- `active` becomes `expired` when the stored `expires_at` boundary has passed or
  the owner path marks the time-bound authorization expired. `expired` is active
  current MVP because Schema Core exposes it, but it is a terminal state for a
  row that was previously active; storage must not create an already-expired row
  as a consumable authorization.
- `consumed`, `stale`, `revoked`, and `expired` cannot transition back to
  `active`. A caller must obtain a fresh compatible `harness.prepare_write`
  result for the exact operation.
- Blocked, dry-run, malformed, or pre-commit failed `prepare_write` attempts
  create no consumable `write_authorizations` row.

`SensitiveActionScope` storage is distinct from `write_authorizations`.
For `judgment_kind=sensitive_approval`, `user_judgments` stores the approved or
pending sensitive-action scope in `sensitive_action_scope_json` or an equivalent
validated judgment payload field. That scope describes the named sensitive
action the user is judging. It is not `AuthorizedAttemptScope`, is not stored in
`write_authorizations.attempt_scope_json`, and does not create or consume a
Write Authorization. A product-file write may need both a compatible
`SensitiveActionScope` judgment and a separate compatible Write Authorization,
but one record cannot substitute for the other.

`surfaces` is not a connector marketplace or broad connector ecosystem table.
It stores the active `LocalSurfaceRegistration` facts needed to interpret
`surface_id`, surface instance identity, local transport binding, capability
profile hash, local access posture, and registration status.

The `surfaces` row is necessary but not sufficient for API trust. It records the
registration basis; the server must still derive a per-request
`VerifiedSurfaceContext` from the local transport/session/binding before a
mutating API call commits or an artifact body is read. `ToolEnvelope.surface_id`
selects the row to compare against. It does not prove authority by itself.
`VerifiedSurfaceContext` is derived request context, not user-provided data, not
a stored registration row, and not a durable authority record.

Product Repository files, projections, generated Markdown, chat text, and agent
memory cannot create, modify, or refresh a `surfaces` row. A registration
refresh changes stored registration facts only through the owner path that
verifies the local surface context and writes current registration state.

`surfaces.local_access_posture` is a closed current MVP value set:

| Value | Storage meaning |
|---|---|
| `registered_local` | A successful local registration or verification established that this stored surface registration can be considered registered-local for current API compatibility checks. |
| `unavailable` | Required MCP/Core or surface reachability cannot currently be established from this registration. |
| `mismatch` | The observed caller or transport binding does not match the stored registered local surface registration. |
| `revoked` | Local access for this registration was explicitly revoked and cannot be used. |

`surfaces.status` is a closed current MVP value set:

| Value | Storage meaning |
|---|---|
| `active` | The stored surface row may be used by current API access checks. |
| `disabled` | The row is retained but must not be used for current API access. |
| `stale` | The row requires refresh before current API access can rely on it. |
| `revoked` | The surface registration is no longer valid for current API access. |

Unknown `surfaces.transport_kind`, `surfaces.local_access_posture`, or `surfaces.status` values are invalid. State-changing API calls require a same-project `surfaces` row with `status=active` and a server-derived `VerifiedSurfaceContext.verified=true` for the requested access class before commit. Artifact body reads require the same verified context for `access_class=artifact_read`. Read-only status paths may return display-safe diagnostics for unavailable, mismatched, stale, disabled, revoked, or insufficient-capability surfaces, but they must not turn those diagnostics into Core state or expose artifact content.

`display_label` is not an active storage identity column. Display labels are
derived from stable identifiers such as `judgment_kind` and locale.

`tasks.lifecycle_phase`, `tasks.close_reason`, and `tasks.result` store separate
Core concepts. `CloseTaskResponse.close_state` is response-level close status,
not a persisted `tasks` column. `tasks.lifecycle_phase` must not store `intake`;
terminal lifecycle values are `completed`, `cancelled`, and `superseded`.
`tasks.result` must not store `passed` or `failed`; failed Runs, projections,
artifacts, validators, evidence gaps, blocked closes, and close blockers remain
in their owning records or current Task state.
`tasks.close_reason` must be compatible with the committed `close_task` intent:
`completed_self_checked` and `completed_with_risk_accepted` are successful
completion reasons that require sufficient required evidence and required final
acceptance, while `cancelled` and `superseded` are terminal non-completion
reasons. `cancelled` and `superseded` must not be stored as if they satisfied
`CompletionPolicy` evidence, final acceptance, or residual-risk acceptance
requirements.
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
Malformed JSON, unknown owner-bound fields, unknown enum values, wrong scalar
types, unbounded arrays, and JSON that names records outside the compatible
project/task scope must fail before commit. SQLite `json_valid`, `CHECK`
constraints, generated columns, or lookup tables may harden the representation,
but they do not replace Core/API/storage owner validation.

Active JSON `TEXT` columns are limited to compact owner-shaped data needed by
the active records, including:

- `surfaces.capability_profile_json`.
- Task and Change Unit shaping columns such as `success_criteria_json`,
  `acceptance_criteria_json`, `scope_boundary_json`, `non_goals_json`,
  `affected_areas_json`, `affected_path_candidates_json`, `constraints_json`,
  `autonomy_boundary_json`, and `completion_policy_json`.
- `user_judgments` request, context, option, affected-ref, artifact-ref, and
  `sensitive_action_scope_json` or `resolution_json` columns.
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

`tasks.completion_policy_json` or `change_units.completion_policy_json` stores
the compact `CompletionPolicy` for `close_task intent=complete` on the Task or
Change Unit. It is not a cancellation or supersession policy. An
`evidence_summaries` row is tied to that owner policy through `task_id` and,
when present, `change_unit_id`; response shapes that include
`EvidenceSummary.completion_policy` read the policy from the owning Task or
Change Unit instead of treating evidence coverage as the policy owner.

`evidence_summaries.coverage_items_json` stores bounded `EvidenceCoverageItem`
entries with explicit `claim`, `required_for_close`, `coverage_state`, and
validated supporting or gap refs. Storage must reject coverage JSON that omits
the required/optional distinction, uses unknown coverage states, or names refs
outside the compatible project/task scope. When the owning
`completion_policy_json.evidence_required=true`, the stored summary must leave
enough information for `harness.close_task` to derive an evidence blocker
whenever a required coverage item is unsupported, partial, stale, blocked, or
missing.

Status-like `TEXT` values are closed owner value sets, not open strings. Active
values are owned by Core/API owners and by the storage notes here. Defensive
`CHECK` constraints or lookup tables may be used, but Core validation remains
required.

## 6. Artifact references

`ArtifactRef` is the public API shape for registered durable evidence bytes or
safe metadata. Storage implements it through `artifacts` plus `artifact_links`;
see [API Schema Core: ArtifactRef](api/schema-core.md#artifactref).

Artifact registration and linking accept only the active owner-documented
`ArtifactInput` sources: `staged_artifact` or `existing_artifact`. A
`staged_artifact` input must carry a `StagedArtifactHandle` from the active
`harness.stage_artifact` utility and must be resolved by the owner path before
storage commits the artifact row. An `existing_artifact` input must name an
already registered persistent `ArtifactRef` that belongs to the same project and
allowed Task scope. `existing_artifact` is not a path to new artifact bytes, does
not register a new artifact body, and may only add a compatible owner link to a
previously persisted artifact. Caller-supplied raw filesystem paths, arbitrary
local path strings, raw logs as authority claims, `captured_artifact` handles,
raw capture-adapter outputs, and native capture claims are not registration
authority in the active MVP.

For `harness.record_run`, storage effects must follow the API-owned validation
order: request-level `VerifiedSurfaceContext.access_class=run_recording`,
project-wide `ToolEnvelope.expected_state_version`, referenced Task and Change
Unit, compatible Write Authorization when product file writes are recorded,
staged-handle validation, staged-handle field checks, staged promotion, staged
consumption, existing-artifact link validation, and no artifact body read.
Storage must not commit any staged promotion, consumed handle, existing-artifact
link, Run, evidence update, event, replay row, authorization consumption, or
state-version increment when any validation in this sequence fails. Artifact body reads
are separate reads that require `access_class=artifact_read`.

Temporary staging is not artifact authority. `artifact_staging` or an
equivalent storage-owned staging manifest must track at least `handle_id`,
`project_id`, `task_id`, `created_by_surface_id`,
`created_by_surface_instance_id`, `sha256`, `size_bytes`, `content_type`,
`redaction_state`, `status`, `expires_at`, and consumption facts such as
`consumed_by_run_id`, `promoted_artifact_id`, and `consumed_at`. The
`created_by_surface_*` fields are recorded by the server from the successful
`harness.stage_artifact` request's `VerifiedSurfaceContext`; they are not
caller-provided authority claims and must be checked against the staging row,
not trusted merely because a submitted handle has the right shape.
`harness.stage_artifact` may write safe bytes or a safe notice under
`artifacts/tmp/` and create the temporary staging row, but it creates no
`artifacts` row, no `artifact_links` row, no `evidence_summaries` row, no
`task_events` row, no `tool_invocations` replay row, and no
`project_state.state_version` increment.

Only a compatible `harness.record_run` may consume an unexpired same-project
same-Task handle with `artifact_staging.status=staged` and promote it to a
persistent `ArtifactRef`, and only when the current verified
`surface_instance_id` matches `created_by_surface_instance_id`. The active MVP
does not support cross-surface staged artifact handoff, and
`StagedArtifactHandle` is not a bearer token that any local caller may use. The
consuming transaction must validate stored `project_id`, `task_id`,
`created_by_surface_instance_id`, expiration, consumed status, `sha256`,
`size_bytes`, and `redaction_state`; promote only validated staged handles; mark
promoted handles `consumed`; set the consuming Run and promoted artifact ids;
commit the durable `artifacts` row and required `artifact_links`; and update
evidence coverage only as allowed by the method owner. Missing, expired,
mismatched, already-consumed, discarded, cross-surface,
wrong-`created_by_surface_instance_id`, wrong-`sha256`, wrong-`size_bytes`,
wrong-`redaction_state`, integrity-incompatible, or cross-task staging handles
must be rejected before mutation with the API-owned `VALIDATION_FAILED`
`artifact_input_error` detail and must not be hidden as evidence sufficiency,
local access mismatch, or capability insufficiency. Projection files, generated
Markdown, chat text, Product Repository files, and agent memory cannot create
staged-handle provenance.

Using an `existing_artifact` reuses the registered artifact row only when its
availability, integrity facts, redaction state, same-project identity, and
allowed Task scope remain compatible with the new use. It may add a new
`artifact_links` row for the new owner relation, subject to the uniqueness and
same-project/same-Task rules; it must not clone bytes, register a new artifact
body, skip integrity checks, or use a raw artifact path as authority.

An artifact is evidence-eligible only when storage has:

- registered bytes or a safe metadata notice under the artifact store,
- integrity facts such as `sha256`, `size_bytes`, and `content_type`,
- a `redaction_state`,
- producer and retention facts,
- an availability `status`,
- and an owner link to an active record such as `task`, `change_unit`, `run`,
  `user_judgment`, `evidence_summary`, or `blocker`.

Evidence eligibility, artifact availability, and evidence sufficiency remain
separate. An `artifacts.status=available` row with a valid owner link can
support a coverage item, but it does not make `EvidenceSummary.status=sufficient`
unless the required coverage item links that artifact to the claim and the item
is `supported` or `not_applicable`. Missing, unavailable, integrity-failed, or
otherwise unusable artifacts stay artifact-availability problems and can also
keep required evidence coverage from being sufficient.

`artifacts.status` is an availability state:

| Value | Storage meaning |
|---|---|
| `available` | The registered safe bytes or safe metadata notice is present and matches stored integrity metadata. |
| `missing` | The artifact row remains, but the registered bytes or safe metadata notice cannot be found. |
| `integrity_failed` | The available bytes or metadata do not match stored integrity facts such as `sha256` or `size_bytes`. |
| `unavailable` | The artifact store or required retrieval path cannot currently provide the registered bytes or safe metadata notice. |

`artifacts.redaction_state` uses the active `ArtifactRef.redaction_state`
values from [API Schema Core](api/schema-core.md#artifactref). `blocked` is a
redaction/omission state, not an artifact availability status. A `blocked`,
`secret_omitted`, or `redacted` artifact may still have `artifacts.status=available`
when the committed safe notice or redacted bytes are present and integrity-aware.

`sha256`, `size_bytes`, and `content_type` are artifact integrity facts for
comparison and availability handling. They do not make artifact storage
tamper-proof or create a cryptographic evidence guarantee claim.

Artifact owner relation integrity is required even though `artifact_links` is a
polymorphic owner table. Storage must validate that `owner_record_kind` is one
of `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, or
`blocker`; that `owner_record_id` exists in the matching active table; that the
owner belongs to the same `project_id` and `task_id`; and that the relation is
compatible with the way the artifact is used. A raw `artifact_id` without a
valid owner link is not evidence support.

When `owner_record_kind=run`, the owner run must be same-Task and compatible
with `artifacts.run_id` when that column is present. When
`owner_record_kind=change_unit`, `user_judgment`, `evidence_summary`, or
`blocker`, the owner row must belong to the same Task and the linked artifact
must not outlive the owner relation as evidence support if that owner is later
superseded or resolved.

`uri` resolves through Harness storage, normally as
`harness-artifact://{project_id}/{artifact_id}`. It is not a caller-supplied
arbitrary filesystem path. Raw secrets, tokens, and full sensitive logs must not
be stored as evidence bytes. Store redacted bytes, `secret_omitted` or `blocked`
notices, safe handles, or other owner-approved safe representations instead.

Raw artifact path reads are not granted by default. Artifact metadata or content
reads require a registered `ArtifactRef`, the matching same-project `task_id`,
the required `artifact_links` owner relation, and the redaction/availability
state needed by the caller's access class. A local path under the artifact store,
an artifact `uri`, a staged path, or a copied file is not enough by itself to
read or rely on artifact bytes.

An artifact link does not create the owner record, satisfy a gate by itself,
prove evidence sufficiency, perform QA, create final acceptance, accept
residual risk, or close a Task.

## 7. Idempotency and event meaning

`task_events` records committed Core mutations in order. It is an audit and
ordering trail, not the normal source used to reconstruct current state during
ordinary operation. Current rows such as `tasks`, `change_units`,
`user_judgments`, `write_authorizations`, `runs`, `artifacts`,
`artifact_links`, `evidence_summaries`, and `blockers` remain the current state.

`task_events` is append-only for ordinary active MVP operation: after an event is
committed, Core must not update or delete that row to change history. Corrections
or repairs are recorded by new events and current-row updates through the owner
path. Idempotent replay, dry-run, malformed requests, and pre-commit failures do
not append events.

For a new committed non-dry-run mutation, current-row writes, the
`task_events` append, the project-wide state-version increment, and the
`tool_invocations` replay-row insert must commit atomically. For
`harness.record_run`, staged-handle consumption in `artifact_staging` is part of
that same transaction. If any part fails, the transaction must leave no partial
authority row, staging consumption, event, artifact registration, authorization
consumption, evidence update, close effect, or replay row.

`tool_invocations` stores exact replay for committed non-dry-run state-changing
responses. Keys are scoped as described by [API Errors: Idempotency](api/errors.md#idempotency).
If the same key and request hash are replayed, Core returns the original
committed response without appending events, registering artifacts, consuming
authorization, or changing state again. If the key is reused with a different
request hash, Core returns `STATE_VERSION_CONFLICT` as defined by
[API Errors](api/errors.md#state-conflict-behavior). The storage unique key is
`(project_id, tool_name, idempotency_key)`; `request_hash` is the conflict
discriminator stored in that row.

Dry runs, malformed requests, pre-commit validation failures, pre-commit state
version conflicts, read-only calls such as `harness.status` and
`harness.close_task intent=check`, and rejected `record_run` attempts that
create no mutation do not create current rows, consume or update
`artifact_staging` handles, append `task_events`, register artifacts, update
evidence summaries, create Write Authorizations, change close state, create
`tool_invocations` replay rows, or increment state versions.

A blocked response may persist only the blocker or other mutation the API
method-state-effect matrix allows. It must not create the authority the blocker
says is missing. For example, blocked `prepare_write` responses do not create consumable
`write_authorizations`. When the API owner allows a committed blocked response
to persist a blocker or other current-row mutation, that response is a committed
non-dry-run mutation for event, replay-row, and state-version purposes.

## 8. State versioning

The active current MVP has one public state clock:
`project_state.state_version`. It is project-wide and is the only active
authorization, conflict, freshness, and concurrency basis for public API
mutations. Task routing still matters for ownership, blockers, close state,
evidence, and user judgments, but it does not select a separate state clock.

Fresh non-dry-run state-changing API calls compare
`ToolEnvelope.expected_state_version` with the current
`project_state.state_version` before commit. A mismatch returns
`STATE_VERSION_CONFLICT` and creates no current records, events, artifacts,
evidence summaries, Write Authorizations, close state, replay rows, or
state-version increments. `STATE_VERSION_CONFLICT` is the only active current
MVP public `ErrorCode` for project-wide state-version mismatch; `STATE_CONFLICT`
is not an active public code, alias, deprecated spelling, or storage-layer
public error name. No active current MVP call requires or accepts more than one
public `expected_state_version`.

Every committed non-dry-run mutation increments
`project_state.state_version` by exactly 1. This includes committed blocked
responses when the method owner allows Core to persist a blocker or another
current-row mutation. A single public call may update Task lifecycle fields and
project-level fields together, such as `harness.close_task intent=supersede`
updating both `tasks.lifecycle_phase` and `project_state.active_task_id`, but it
is still one mutation and creates exactly one project-wide version increment.

`harness.status`, `harness.close_task intent=check`, dry-run calls, malformed
requests, pre-commit validation failures, pre-commit state-version conflicts, and
idempotent replay do not increment `project_state.state_version`.
`ToolResponseBase.state_version` always returns the project-wide version: the
resulting version after a committed mutation, or the current project-wide
version observed for read-only and dry-run responses.

The active first schema should omit `tasks.state_version`. If an implementation
encounters a legacy or prototype `tasks.state_version` column, that value is
inactive metadata only. It must not be used as an authorization,
`STATE_VERSION_CONFLICT`, stale-state, Write Authorization, idempotency, lock, or
concurrency basis.

`write_authorizations.basis_state_version` stores the project-wide
`project_state.state_version` used when Core prepared the authorization.
Stale Write Authorization detection compares that stored value with the current
project-wide state version, not with any Task-local clock. When that mismatch
is surfaced through the public API, the public error is `STATE_VERSION_CONFLICT`.
Storage examples may describe stale state or version mismatch, but storage does
not define a different public error name or expose internal database exception
names.
`write_authorizations.attempt_scope_json` stores the authorized attempt boundary
that `record_run` later compares against observed facts. The top-level
`task_id`, `change_unit_id`, `surface_id`, and `basis_state_version` columns are
query fields; the stored attempt scope remains the compatibility boundary.

`tool_invocations.basis_state_version` stores the project-wide state version
observed by the call before the committed mutation. `task_events.state_version`
stores the resulting project-wide version after the committed event.

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
- Each future migration must declare a source version, target version, storage
  profile, owner, and rollback or repair expectation before it is accepted.
- Run future migrations transactionally for `registry.sqlite` or one
  `state.sqlite` at a time, with a clear interrupted-state recovery rule before
  runtime implementation.
- Validate owner-shaped JSON before commit and before tightening constraints.
- Treat unknown owner-bound status or enum values as invalid until an owner
  defines them.
- Tighten nullable fields, foreign keys, enum checks, and JSON validation only
  after existing rows have been validated or routed to an owner-defined repair
  state.
- Preserve `task_events.event_seq` ordering when `task_events` is retained.
- Preserve artifact hashes and owner links, or mark affected refs invalid for
  recovery.
- Preserve committed `tool_invocations` replay rows so idempotency does not fork
  after migration.
- Keep status cards, compact views, projection freshness, close readiness, and
  report prose derived from current records at read time. They are not migration
  authority, repair input, or storage mutation paths.

This page intentionally excludes inactive DDL bundles, migration catalogs, and
profile-specific migration details.

## 11. Later storage excluded from active current MVP

Profile-gated later storage is outside the active current MVP unless an owner
document promotes a narrow behavior with scope, fallback behavior, and
proof-path expectations for future promotion. Reference-schema presence alone
does not make storage active.

The active current MVP excludes storage for:

- projection jobs, durable projection caches, managed-output outboxes, managed
  block hash/drift-repair records, and projection dashboards;
- validator-run records, conformance-runner state, fixture execution history,
  and generated conformance artifacts;
- operations-profile storage for doctor suites, recover, export, release
  handoff, artifact dashboards, reconcile queues, or operational reports;
- `captured_artifact` handles, native capture storage, capture-adapter output
  tables, raw capture queues, or surface-native artifact capture records;
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
cards, `agent-context-packet`, and guarantee display are read-time derived views
over the active persisted records above. They may be stale, absent, failed, or
recomputed without changing storage authority.
