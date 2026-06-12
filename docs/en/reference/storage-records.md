# Storage records

Rule:

- This document owns persistent storage record layout for the baseline scope source design.
- Persistent records are local records committed by Core for subsequent reads.

Not allowed:

- Persistent records are not tamper-proof storage, anti-forgery proof, or external audit guarantees.

Owner links:

- Security non-claims and guarantee levels belong to [Security](security.md).

## Owns / Does not own

This document owns:

- Runtime Home identity and project-local storage layout assumptions.
- Active persisted record categories and table-level storage roles.
- Record-column meaning for baseline storage design.
- Storage-owned record values and status fields.
- Storage-owned JSON `TEXT` placement and validation expectations.
- Record-level active/out-of-scope exclusions.

This document does not own:

- method-to-storage effects; see [Storage Effects](storage-effects.md)
- artifact staging, promotion, linking, body reads, retention, or integrity lifecycle; see [Artifact Storage](storage-artifacts.md)
- `project_state.state_version`, idempotency, event meaning, locks, or migrations; see [Storage Versioning](storage-versioning.md)
- API request or response schemas; see [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Artifact Schemas](api/schema-artifacts.md), and the other API schema owners
- API method behavior; see the [API Methods](api/methods.md) and method owner documents
- Runtime Home and Product Repository boundaries; see [Runtime Boundaries](runtime-boundaries.md)

## Records versus API schemas

Storage records and API schemas have different owner boundaries.

- API schema files define request and response data shape, public API values, public errors, and response branches. See [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Artifact Schemas](api/schema-artifacts.md), [API Judgment Schemas](api/schema-judgment.md), and [API Value Sets](api/schema-value-sets.md).
- Storage records define persistent record categories, file/table placement, storage-owned JSON `TEXT` placement, stored relationships, and validation expectations.
- A similar name in both places is not the same authority. `CloseReadinessBlocker` is an API data shape; `blockers` is a storage row. `ArtifactRef` is an API schema; `artifacts` and `artifact_links` are storage records.
- Storage layout does not own rendered template prose. User-visible status cards, judgment requests, run/evidence summaries, close results, and agent context packets belong to [Template Bodies](template-bodies.md) and [Projection Authority Reference](projection-and-templates.md).

## Runtime Home layout

Rule:

- Harness uses one local Runtime Home and one project-local state database per registered project.
- The default reference root is `~/.harness`.

Allowed:

- An implementation may choose an equivalent configured root.

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

The path meanings are storage assumptions:

- The Runtime Home root, shown as `~/.harness`, is Harness operational data. It is not the Product Repository and not a grant of filesystem permission.
- `registry.sqlite` stores Runtime Home identity and minimal project registration. It is the Runtime Home registry, not project-local Task state.
- `projects/{project_id}/` is the Harness project home for one registered project. It is not the same thing as `repo_root`.
- `project.yaml` stores static project configuration only.
- `state.sqlite` stores project-local Core state for the registered project.
- `artifacts/` is the project artifact store. `artifacts/tmp/` is transient staging space, not evidence authority.

Rule:

- Runtime Home identity must not depend only on a filesystem path.
- A copied or moved Runtime Home may carry the same stored `runtime_home_id`.
- A newly created Runtime Home gets a new id.
- Runtime Home files are local operational control data and may contain sensitive support data.

Allowed:

- The id helps detect suspicious copies, duplicate registrations, or path drift.

Not allowed:

- The id is not a security guarantee.

Owner links:

- Security non-claims and guarantee levels belong to [Security](security.md).
- Location boundaries belong to [Runtime Boundaries](runtime-boundaries.md).

## Runtime Home exclusions

Runtime Home is Harness operational data space. It is not automatically the Product Repository, and it does not replace product-repository write authority or safety judgment.

- Product source files, the Product Repository working tree, and build outputs are not Runtime Home records.
- `repo_root` is the registered Product Repository path. `projects/{project_id}/` is the Harness project home. Do not treat them as the same location or authority.
- `project.yaml` must not store current Task state, gates, Write Authorization state, evidence sufficiency, final acceptance, residual-risk acceptance, or close state.
- Generated projection bodies, rendered template prose, expanded evidence-package bodies, QA records, acceptance records, residual-risk records, and close records are not part of this storage record layout.

## Persisted record categories

### Conditions

Rule:

- The baseline persists only the Core records needed by the active state-changing method set.
- `harness.status` and `harness.close_task intent=check` are read-only.

State-changing method set:

- `harness.intake`.
- `harness.update_scope`.
- `harness.prepare_write`.
- `harness.record_run`.
- `harness.request_user_judgment`.
- `harness.record_user_judgment`.
- State-changing `harness.close_task` intents.

### Stored records

The active Core persisted records are:

- Runtime Home identity in `registry.sqlite`.
- Minimal project registration in `registry.sqlite`.
- Static project configuration in `project.yaml`.
- `project_state`.
- `surfaces`, limited to the registered local/reference surface facts needed by the active API envelope, capability display, and local-access posture.
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

### Transient storage boundary

- `artifact_staging` or an equivalent storage-owned staging manifest.
- Safe transient bytes or notices under `artifacts/tmp/`.

Owner links:

- Staging lifecycle, provenance, consumption, and promotion belong to [Artifact Storage](storage-artifacts.md).

### Not stored

Not allowed:

- No other persisted table family or transient handle family is baseline scope.
- Requirement shaping is not a separate committed Discovery Brief, Shared Design, Question Queue, Assumption Register, or First Safe Change Unit Candidate table.
- Evidence is stored only through the active compact evidence-summary and artifact-ref paths.

### Projection non-claim

Not allowed:

- Projection has no active persisted table family.
- `status-card`, `judgment-request`, `run-evidence-summary`, `close-result`, and `agent-context-packet` are not stored state or storage mutation paths.

Allowed:

- `status-card`, `judgment-request`, `run-evidence-summary`, `close-result`, and `agent-context-packet` are read-time views over active records.

### Shaping storage

Allowed:

- Requirement shaping persists through `tasks`, `change_units`, `user_judgments`, `evidence_summaries`, and `blockers`.
- Evidence persists through compact evidence summaries, `CompletionPolicy` on the Task or Change Unit, required coverage items, and artifact refs.

Minimum active shaping information:

- current goal summary
- active scope summary
- allowed paths or affected areas
- non-goals
- acceptance criteria
- Autonomy Boundary
- required user-owned judgments
- one blocking question when necessary
- one next safe action
- `CompletionPolicy`
- required evidence expectation or evidence gap
- close readiness

Exceptions:

- Missing or unknown pieces stay as `unknown`, pending `user_judgments`, evidence gaps, or `blockers`.

Not allowed:

- Storage must not create extra active planning tables to make the request appear ready.

## Table overview

This table names active storage record categories and links to category details. It is not full DDL and does not duplicate API schemas or rendered template bodies.

| Record category | Purpose | Details |
|---|---|---|
| Runtime Home identity | local Runtime Home identity and storage profile | See [Runtime Home identity](#runtime-home-identity) |
| Project registration | registered project to project-local storage mapping | See [Project registration](#project-registration) |
| `project.yaml` | static project configuration | See [`project.yaml`](#projectyaml) |
| `project_state` | current project state, version, and active pointers | See [`project_state`](#project_state) |
| `surfaces` | registered local surface facts for API access checks | See [`surfaces`](#surfaces) |
| `tasks` | work unit, shaping, lifecycle, and close state | See [`tasks`](#tasks) |
| `change_units` | scoped write and close boundary | See [`change_units`](#change_units) |
| `user_judgments` | user-owned judgment and sensitive-action approval records | See [`user_judgments`](#user_judgments) |
| `write_authorizations` | single-use cooperative Write Authorization records | See [`write_authorizations`](#write_authorizations) |
| `runs` | committed execution or observation records | See [`runs`](#runs) |
| `artifact_staging` | transient staged artifact handles | See [`artifact_staging`](#artifact_staging) |
| `artifacts` | registered durable artifact metadata or bytes | See [`artifacts`](#artifacts) |
| `artifact_links` | owner relations between artifacts and supported Core/API records | See [`artifact_links`](#artifact_links) |
| `evidence_summaries` | compact evidence coverage and gap records | See [`evidence_summaries`](#evidence_summaries) |
| `blockers` | structured blocker state | See [`blockers`](#blockers) |
| `task_events` | append-only audit and ordering trail for committed Core mutations | See [`task_events`](#task_events) |
| `tool_invocations` | committed Core method replay rows | See [`tool_invocations`](#tool_invocations) |

## Record category details

<a id="runtime-home-identity"></a>
### Runtime Home identity

Purpose:
- Identifies the local Runtime Home and the schema/storage profile.

Stored in:
- `registry.sqlite`.

Contains:
- `runtime_home_id`.
- `schema_version` and `storage_profile`.
- `created_at` and `updated_at`.

Does not contain:
- project-local Task state.
- Product Repository content or permissions.
- tamper-proof proof material.

Owner links:
- [Runtime Boundaries](runtime-boundaries.md).
- [Security](security.md).

<a id="project-registration"></a>
### Project registration

Purpose:
- Maps a registered project to its project-local storage.

Stored in:
- `registry.sqlite`.

Contains:
- `project_id`.
- `repo_root` and `project_home`.
- `display_name` and `status`.
- `created_at` and `updated_at`.

Does not contain:
- current Task lifecycle state.
- Product Repository file contents.
- multi-registration behavior beyond the baseline scope.

Owner links:
- [Runtime Boundaries](runtime-boundaries.md).
- [Storage Versioning](storage-versioning.md).

<a id="projectyaml"></a>
### `project.yaml`

Purpose:
- Stores static project configuration for one registered project.

Stored in:
- the project directory under the Runtime Home.

Contains:
- `project_id`.
- `repo_root`.
- display and configuration defaults.

Does not contain:
- current Task state, gates, or Write Authorization state.
- evidence sufficiency, final acceptance, residual-risk acceptance, or close state.
- rendered template text.

Owner links:
- [Runtime Boundaries](runtime-boundaries.md).
- [Storage Versioning](storage-versioning.md).

<a id="project_state"></a>
### `project_state`

Purpose:
- Stores the project-local state header, public project-wide state clock, active Task pointer, and default surface pointer.

Stored in:
- `state.sqlite`.

Contains:
- `project_id`.
- `schema_version`, `storage_profile`, and `state_version`.
- `active_task_id` and `default_surface_id`.
- `created_at` and `updated_at`.

Does not contain:
- artifact bytes.
- API request or response bodies.
- rendered template text.
- tamper-proof proof material.

Owner links:
- [Storage Versioning](storage-versioning.md).
- [API State Schemas](api/schema-state.md).

<a id="surfaces"></a>
### `surfaces`

Purpose:
- Stores `LocalSurfaceRegistration` facts used to verify a local surface context for API access.

Stored in:
- `state.sqlite`.

Contains:
- `project_id`, `surface_id`, and `surface_instance_id`.
- `transport_kind`, `transport_binding_fingerprint`, and `access_secret_hash`.
- `capability_profile_hash` and `capability_profile_json`.
- `status`, `local_access_posture`, `registered_at`, `last_verified_at`, and `updated_at`.

Does not contain:
- live proof that the current caller is trusted.
- caller-provided authority claims.
- hosted connector registry state.

Owner links:
- [Agent Integration](agent-integration.md).
- [API Methods](api/methods.md).
- [Security](security.md).

<a id="tasks"></a>
### `tasks`

Purpose:
- Stores the user-value work unit, shaping summary, lifecycle, result, next action, active Task-level `CompletionPolicy`, and close fields.

Stored in:
- `state.sqlite`.

Contains:
- `task_id`, `project_id`, `title`, `user_request`, and `current_goal_summary`.
- `mode`, `lifecycle_phase`, `close_reason`, `result`, and `summary`.
- shaping JSON columns and `completion_policy_json`.
- `blocking_question`, `next_safe_action`, and `active_change_unit_id`.
- `created_at`, `updated_at`, and `closed_at`.

Does not contain:
- a separate committed Discovery Brief, Question Queue, Assumption Register, or First Safe Change Unit Candidate.
- expanded evidence-package storage.
- rendered `status-card` or `close-result` bodies.

Owner links:
- [Core Model](core-model.md).
- [API State Schemas](api/schema-state.md).
- [Template Bodies](template-bodies.md).

<a id="change_units"></a>
### `change_units`

Purpose:
- Stores the current or proposed scoped work boundary for write compatibility, Change Unit-level `CompletionPolicy`, and close basis.

Stored in:
- `state.sqlite`.

Contains:
- `change_unit_id`, `task_id`, and `scope_summary`.
- scope JSON columns for allowed paths or affected areas.
- `baseline_ref`, `autonomy_boundary_json`, and `completion_policy_json`.
- `status`, `created_at`, and `updated_at`.

Does not contain:
- a separate Shared Design or First Safe Change Unit Candidate table.
- Product Repository diff bytes.
- Write Authorization records.

Owner links:
- [Core Model](core-model.md).
- [Storage Effects](storage-effects.md).
- [API Methods](api/methods.md).

<a id="user_judgments"></a>
### `user_judgments`

Purpose:
- Stores user-owned judgment records, including separate sensitive-action approval scope when relevant.

Stored in:
- `state.sqlite`.

Contains:
- `user_judgment_id`, `task_id`, and `change_unit_id`.
- `judgment_kind`, `presentation`, and `status`.
- request/context JSON columns, `question`, and `sensitive_action_scope_json`.
- `resolution_json`, `expires_at`, `resolved_at`, `created_at`, and `updated_at`.

Does not contain:
- Core-owned state or artifact authority.
- artifact bytes.
- blanket approval beyond the recorded judgment scope.

Owner links:
- [API Judgment Schemas](api/schema-judgment.md).
- [Core Model](core-model.md).
- [API Methods](api/methods.md).

<a id="write_authorizations"></a>
### `write_authorizations`

Purpose:
- Stores durable single-use cooperative Write Authorization created only by non-dry-run `prepare_write` with `decision=allowed`.

Stored in:
- `state.sqlite`.

Contains:
- `write_authorization_id`, `task_id`, `change_unit_id`, and `surface_id`.
- `status`, `basis_state_version`, and `attempt_scope_json`.
- `consumed_by_run_id`, `expires_at`, `created_at`, `updated_at`, and `consumed_at`.

Does not contain:
- Product Repository writes themselves.
- evidence sufficiency or final acceptance.
- reusable permission or a stronger security guarantee.

Owner links:
- [Storage Effects](storage-effects.md).
- [API Methods](api/methods.md).
- [Security](security.md).

<a id="runs"></a>
### `runs`

Purpose:
- Stores committed execution or observation records, including compatible Write Authorization consumption when a product write happened.

Stored in:
- `state.sqlite`.

Contains:
- `run_id`, `task_id`, `change_unit_id`, `write_authorization_id`, and `surface_id`.
- `kind`, `status`, `product_write`, `baseline_ref`, and `summary`.
- observed/evidence JSON columns.
- `created_at` and `completed_at`.

Does not contain:
- artifact bytes.
- rendered run/evidence summary text.
- final acceptance or residual-risk acceptance by itself.

Owner links:
- [Storage Effects](storage-effects.md).
- [API State Schemas](api/schema-state.md).
- [Core Model](core-model.md).

<a id="artifact_staging"></a>
### `artifact_staging`

Purpose:
- Stores transient staged safe bytes or safe notices created by `harness.stage_artifact` for single-use consumption by a subsequent `harness.record_run`.

Stored in:
- `state.sqlite` plus safe transient bytes or notices under `artifacts/tmp/`.

Contains:
- `handle_id`, `project_id`, `task_id`, `created_by_surface_id`, and `created_by_surface_instance_id`.
- `display_name`, `relation_hint`, `tmp_uri`, `sha256`, `size_bytes`, `content_type`, and `redaction_state`.
- `status`, `consumed_by_run_id`, `promoted_artifact_id`, `expires_at`, `created_at`, and `consumed_at`.

Does not contain:
- persistent `ArtifactRef` authority.
- evidence sufficiency or close readiness.
- cross-surface staged artifact transfer.

Owner links:
- [Artifact Storage](storage-artifacts.md).
- [API Artifact Schemas](api/schema-artifacts.md).
- [API Methods](api/methods.md).

<a id="artifacts"></a>
### `artifacts`

Purpose:
- Stores registered durable evidence bytes or safe metadata with integrity, redaction, producer, retention, and availability facts.

Stored in:
- `state.sqlite` plus the project artifact store.

Contains:
- `artifact_id`, `project_id`, `task_id`, and `run_id`.
- `kind`, `uri`, `sha256`, `size_bytes`, `content_type`, and `redaction_state`.
- `retention_class`, `produced_by`, `status`, `created_at`, and `updated_at`.

Does not contain:
- evidence sufficiency by itself.
- rendered evidence summary bodies.
- unrestricted body-read permission.

Owner links:
- [Artifact Storage](storage-artifacts.md).
- [API Artifact Schemas](api/schema-artifacts.md).
- [Security](security.md).

<a id="artifact_links"></a>
### `artifact_links`

Purpose:
- Stores the owner relation from an artifact to the active Core/API record it supports.

Stored in:
- `state.sqlite`.

Contains:
- `artifact_link_id`, `artifact_id`, and `task_id`.
- `owner_record_kind`, `owner_record_id`, and `relation`.
- `created_at`.

Does not contain:
- artifact bytes.
- embedded owner records or API schema objects.
- proof that the artifact is sufficient evidence.

Owner links:
- [Artifact Storage](storage-artifacts.md).
- [API Value Sets](api/schema-value-sets.md).
- [Core Model](core-model.md).

<a id="evidence_summaries"></a>
### `evidence_summaries`

Purpose:
- Stores compact evidence coverage and gap records used by status, run/evidence summaries, blockers, and close.

Stored in:
- `state.sqlite`.

Contains:
- `evidence_summary_id`, `task_id`, and `change_unit_id`.
- `status`, `coverage_items_json`, and `summary`.
- `supporting_run_ids_json`, `supporting_artifact_link_ids_json`, and `gap_blocker_ids_json`.
- `updated_at`.

Does not contain:
- expanded evidence-package storage.
- separate QA workflow storage.
- final acceptance.
- rendered `run-evidence-summary` bodies.

Owner links:
- [Core Model](core-model.md).
- [API State Schemas](api/schema-state.md).
- [Template Bodies](template-bodies.md).

<a id="blockers"></a>
### `blockers`

Purpose:
- Stores structured blocker state for next action, write compatibility, evidence gaps, close readiness, or recovery.

Stored in:
- `state.sqlite`.

Contains:
- `blocker_id`, `task_id`, `blocked_action`, `blocker_kind`, and `status`.
- `message`, `owner_ref_json`, `related_refs_json`, and `required_next_action`.
- `created_at` and `resolved_at`.

Does not contain:
- `CloseReadinessBlocker` as a stored row or persistence signal.
- the whole close-readiness concept.
- rendered template text.

Owner links:
- [Core Model](core-model.md).
- [API State Schemas](api/schema-state.md).
- [API Errors](api/errors.md).

<a id="task_events"></a>
### `task_events`

Purpose:
- Stores the append-only audit and ordering trail for committed Core mutations.

Stored in:
- `state.sqlite`.

Contains:
- `event_id`, `project_id`, `task_id`, and `event_seq`.
- `event_type`, `state_version`, `actor_kind`, and `surface_id`.
- `payload_json` and `created_at`.

Does not contain:
- Product Repository diff bytes.
- rendered template bodies.
- external audit or tamper-proof guarantees.

Owner links:
- [Storage Versioning](storage-versioning.md).
- [Storage Effects](storage-effects.md).
- [Security](security.md).

<a id="tool_invocations"></a>
### `tool_invocations`

Purpose:
- Stores replay rows only for committed non-dry-run Core `MethodResult` responses whose method state-effect row creates replay.

Stored in:
- `state.sqlite`.

Contains:
- `invocation_id`, `project_id`, `tool_name`, and `idempotency_key`.
- `request_hash`, `task_id`, and `basis_state_version`.
- `response_json`, `status`, and `created_at`.

Does not contain:
- dry-run or read-only responses when no replay effect is created.
- API schema definitions.
- permission for one idempotency key to fork into multiple committed responses.

Owner links:
- [Storage Versioning](storage-versioning.md).
- [API Methods](api/methods.md).
- [API Schema Core](api/schema-core.md).

## First schema integrity contract

Rule:

- This section is a first-implementation storage contract for SQLite schema design.
- For a first SQLite schema, committed rows must preserve these identities, value sets, relations, and transaction boundaries.

Allowed:

- Implementations may choose `CHECK` constraints, lookup tables, generated columns, triggers, or Core-side validation.

Not allowed:

- This section is not full DDL or a migration file.
- Implementation choices do not replace Core/API/storage owner validation.

Required identity and uniqueness constraints:

- Active tables use opaque stable ids as primary keys or equivalent unique keys.

  Identity fields include:
  - `project_id`
  - `surface_id`
  - `surface_instance_id`
  - `task_id`
  - `change_unit_id`
  - `user_judgment_id`
  - `write_authorization_id`
  - `run_id`
  - `handle_id`
  - `artifact_id`
  - `artifact_link_id`
  - `evidence_summary_id`
  - `blocker_id`
  - `event_id`
  - `invocation_id`
- Runtime Home identity stores one `runtime_home_id` for the Runtime Home.
- Project registration requires unique `project_id`, unique `project_home`, and one active registration for a `repo_root` unless an owner defines multi-registration behavior.
- `project_state.project_id` is one row per registered project.
- `surfaces` requires a unique `(project_id, surface_id)`. The stored `surface_instance_id` identifies the registered local instance selected by that surface row.
- `tasks` requires a unique `(project_id, task_id)`.
- `change_units` requires unique `(task_id, change_unit_id)` and at most one `status=active` Change Unit per Task.
- `write_authorizations.consumed_by_run_id`, `runs.write_authorization_id`, `artifact_staging.consumed_by_run_id`, and `artifact_staging.promoted_artifact_id` are unique when non-null.
- `artifact_staging` requires a unique `(project_id, handle_id)` and a unique `tmp_uri` or equivalent staging-object identity within the project while the handle exists.
- `artifact_links` requires a uniqueness rule equivalent to `(artifact_id, owner_record_kind, owner_record_id, relation)`.
- `artifacts.uri`, when stored rather than derived, must be unique within the project and must resolve to the same `artifact_id`.
- `task_events` requires unique `event_id` and monotonic unique `event_seq` within the affected project/Task scope.
- `tool_invocations` requires a unique replay key on `(project_id, tool_name, idempotency_key)`. `request_hash` is the conflict discriminator stored in that row, not a second uniqueness key that permits forking.

Main relationship constraints:

- Project-scoped rows belong to project registration.
- Active Task pointers and default surface pointers must point to same-project rows.
- `tasks.active_change_unit_id` points to a `change_units` row for the same Task.
- Task-scoped rows such as `change_units`, `user_judgments`, `write_authorizations`, `runs`, `evidence_summaries`, `blockers`, `task_events`, and `tool_invocations` point to the same project/Task scope.
- `runs.write_authorization_id`, when present, points to the consumed `write_authorizations` row and must match the same Task, Change Unit, surface, and compatible attempt scope.
- `artifact_staging.consumed_by_run_id` and `artifact_staging.promoted_artifact_id`, when present, point to same-project same-Task `runs` and `artifacts` rows created by the consuming `harness.record_run` transaction.
- `artifact_links.artifact_id` points to `artifacts`, and `artifact_links.task_id` points to the same Task as the artifact and owner relation.
- JSON ref arrays such as `supporting_run_ids_json`, `supporting_artifact_link_ids_json`, `gap_blocker_ids_json`, `related_refs_json`, and `response_json` must be parsed and checked against the same project/task and owner relation before commit, even where SQLite cannot express the relation as a direct foreign key.

### Authority-row deletion non-claim

Rule:

- Ordinary baseline Core operations do not hard-delete authority rows.
- Rows move through status or lifecycle fields.
- Core appends events.
- Replay and artifact metadata remain available for audit and recovery.
- Foreign keys should default to `RESTRICT` or equivalent owner validation for authority rows.

Not allowed:

- Closing, cancelling, or superseding a Task must not cascade-delete these rows:
  - `tasks`
  - `change_units`
  - `user_judgments`
  - `write_authorizations`
  - `runs`
  - `artifacts`
  - `artifact_links`
  - `evidence_summaries`
  - `blockers`
  - `task_events`
  - `tool_invocations`

### Transient staging exception

Exceptions:

- Unconsumed or expired `artifact_staging` rows and `artifacts/tmp/` staging bytes or notices may be marked `expired` or `discarded`.
- transient bytes may be cleaned before registration.
- These exceptions are allowed because unconsumed staging rows and transient bytes are not evidence authority.

Not allowed:

- Once an `artifacts` row is committed, retention purge, project teardown, or destructive cleanup is outside ordinary baseline mutation behavior and needs an owner-defined path.

## Storage-owned value summary

Rule:

- Closed baseline scope storage value sets are table-level persistence constraints.
- Rows that mirror API schema values must match the API schema owner exactly.
- Rows marked storage-owned define storage behavior that is not a public API schema body.

Not allowed:

- Unknown values must not commit.

| Field | Purpose | Details |
|---|---|---|
| Project registration `status` | active project registration baseline | See [Project registration `status`](#project-registration-status) |
| `surfaces.transport_kind` | stored local transport category | See [`surfaces.transport_kind`](#surfacestransport_kind) |
| `surfaces.local_access_posture` | stored registration posture | See [`surfaces.local_access_posture`](#surfaceslocal_access_posture) |
| `surfaces.status` | stored surface registration usability | See [`surfaces.status`](#surfacesstatus) |
| `tasks.lifecycle_phase` | persisted Task lifecycle | See [`tasks.lifecycle_phase`](#taskslifecycle_phase) |
| `tasks.close_reason` | persisted close detail | See [`tasks.close_reason`](#tasksclose_reason) |
| `tasks.result` | persisted coarse Task outcome | See [`tasks.result`](#tasksresult) |
| `change_units.status` | active Change Unit lifecycle | See [`change_units.status`](#change_unitsstatus) |
| `write_authorizations.status` | durable authorization lifecycle | See [`write_authorizations.status`](#write_authorizationsstatus) |
| `artifact_staging.status` | transient handle lifecycle | See [`artifact_staging.status`](#artifact_stagingstatus) |
| `artifacts.status` | artifact availability state | See [`artifacts.status`](#artifactsstatus) |
| `artifact_links.owner_record_kind` | owner relation discriminator | See [`artifact_links.owner_record_kind`](#artifact_linksowner_record_kind) |
| `blockers.status` | blocker row state | See [`blockers.status`](#blockersstatus) |
| `tool_invocations.status` | committed replay row state | See [`tool_invocations.status`](#tool_invocationsstatus) |

<a id="project-registration-status"></a>
### Project registration `status`

Values:
- `active`

Storage rule:
- Only registered active projects are in the baseline scope.
- Disable/unregister behavior is outside the baseline scope until promoted.

Owner links:
- [Runtime Boundaries](runtime-boundaries.md).
- [Storage Versioning](storage-versioning.md).

<a id="surfacestransport_kind"></a>
### `surfaces.transport_kind`

Values:
- `local_mcp_stdio`
- `local_http`

Storage rule:
- Stored local transport category for registration matching.
- It is not a socket or protocol setup specification.

Owner links:
- [Agent Integration](agent-integration.md).
- [API Methods](api/methods.md).
- [Security](security.md).

<a id="surfaceslocal_access_posture"></a>
### `surfaces.local_access_posture`

Values:
- `registered_local`
- `unavailable`
- `mismatch`
- `revoked`

Storage rule:
- Stored registration posture for API compatibility checks.
- Meanings mirror the API schema owner.

Owner links:
- [API Schema Core](api/schema-core.md).
- [Agent Integration](agent-integration.md).
- [Security](security.md).

<a id="surfacesstatus"></a>
### `surfaces.status`

Values:
- `active`
- `disabled`
- `stale`
- `revoked`

Storage rule:
- Stored surface registration usability.
- Meanings mirror the API schema owner.

Owner links:
- [API Schema Core](api/schema-core.md).
- [Agent Integration](agent-integration.md).
- [Security](security.md).

<a id="taskslifecycle_phase"></a>
### `tasks.lifecycle_phase`

Values:
- `shaping`
- `ready`
- `executing`
- `waiting_user`
- `blocked`
- `completed`
- `cancelled`
- `superseded`

Storage rule:
- Persisted Task lifecycle.
- `intake` is not a stored lifecycle value.
- `superseded` is terminal.

Owner links:
- [Task lifecycle values](api/schema-value-sets.md#task-lifecycle-values).
- [API State Schemas](api/schema-state.md).
- [Core Model](core-model.md).

<a id="tasksclose_reason"></a>
### `tasks.close_reason`

Values:
- `none`
- `completed_self_checked`
- `completed_with_risk_accepted`
- `cancelled`
- `superseded`

Storage rule:
- Persisted close detail.
- It is separate from lifecycle and result.

Owner links:
- [Task lifecycle values](api/schema-value-sets.md#task-lifecycle-values).
- [API State Schemas](api/schema-state.md).
- [Core Model](core-model.md).

<a id="tasksresult"></a>
### `tasks.result`

Values:
- `none`
- `advice_only`
- `completed`
- `cancelled`
- `superseded`

Storage rule:
- Persisted coarse outcome.
- Failed Runs, violations, blocked closes, and evidence gaps stay in their owning records.

Owner links:
- [Task lifecycle values](api/schema-value-sets.md#task-lifecycle-values).
- [API State Schemas](api/schema-state.md).
- [Core Model](core-model.md).

<a id="change_unitsstatus"></a>
### `change_units.status`

Values:
- `proposed`
- `active`
- `replaced`
- `closed`

Storage rule:
- Storage-owned active Change Unit lifecycle.
- The lifecycle supports write compatibility and close basis.

Owner links:
- [Core Model](core-model.md).
- [Storage Effects](storage-effects.md).
- [API Methods](api/methods.md).

<a id="write_authorizationsstatus"></a>
### `write_authorizations.status`

Values:
- `active`
- `consumed`
- `expired`
- `stale`
- `revoked`

Storage rule:
- Durable authorization lifecycle.
- Storage owns persistence and transition rules.

Owner links:
- [Storage Effects](storage-effects.md).
- [API Methods](api/methods.md).
- [Security](security.md).

<a id="artifact_stagingstatus"></a>
### `artifact_staging.status`

Values:
- `staged`
- `consumed`
- `expired`
- `discarded`

Storage rule:
- Storage-owned transient handle lifecycle.
- Only `staged` is consumable by `harness.record_run`.
- Terminal values cannot return to `staged`.

Owner links:
- [Artifact Storage](storage-artifacts.md).
- [API Artifact Schemas](api/schema-artifacts.md).
- [API Methods](api/methods.md).

<a id="artifactsstatus"></a>
### `artifacts.status`

Values:
- `available`
- `missing`
- `integrity_failed`
- `unavailable`

Storage rule:
- Storage-owned artifact availability state.
- Redaction and blocked-payload handling stay in `redaction_state`.

Owner links:
- [Artifact Storage](storage-artifacts.md).
- [API Artifact Schemas](api/schema-artifacts.md).
- [Security](security.md).

<a id="artifact_linksowner_record_kind"></a>
### `artifact_links.owner_record_kind`

Values:
- `task`
- `change_unit`
- `run`
- `user_judgment`
- `evidence_summary`
- `blocker`

Storage rule:
- Persisted owner relation discriminator.
- Storage owns same-project/same-Task owner lookup and relation validation.

Owner links:
- [Artifact Storage](storage-artifacts.md).
- [Record and reference values](api/schema-value-sets.md#record-and-reference-values).
- [Core Model](core-model.md).

<a id="blockersstatus"></a>
### `blockers.status`

Values:
- `active`
- `resolved`
- `superseded`

Storage rule:
- Storage-owned blocker row state.
- Public close blocker shapes remain API-owned.

Owner links:
- [Core Model](core-model.md).
- [API State Schemas](api/schema-state.md).
- [API Errors](api/errors.md).

<a id="tool_invocationsstatus"></a>
### `tool_invocations.status`

Values:
- `committed`

Storage rule:
- A replay row exists only for a committed non-dry-run Core `MethodResult` response whose method state-effect row creates replay.

Owner links:
- [Storage Versioning](storage-versioning.md).
- [API Methods](api/methods.md).
- [API Schema Core](api/schema-core.md).

### Values not owned here

Rule:

- Other persisted status-like API fields validate against [API Value Sets](api/schema-value-sets.md) and the Core/API method owners.

Examples:

- `tasks.mode`
- `runs.kind`
- `runs.status`
- `user_judgments.status`
- `evidence_summaries.status`

Allowed:

- Storage may index and constrain them.

Not allowed:

- This document does not redefine their public schema values.

## Storage-owned JSON

### JSON storage conditions

SQLite `TEXT` columns that store JSON are a storage representation choice, not permission to persist arbitrary JSON.

- Core must parse and validate JSON before commit.
- API-shaped stored JSON validates against the API schema owners.
- Storage-only JSON validates against this document or the owner document named by this document.
- SQLite defaults such as `'{}'` and `'[]'` are storage defaults only; they do not make API fields optional.

### Stored JSON

Active JSON `TEXT` columns are limited to compact owner-shaped data needed by the active records, including:

- `surfaces.capability_profile_json`.
- Task and Change Unit shaping columns such as `success_criteria_json`, `acceptance_criteria_json`, `scope_boundary_json`, `non_goals_json`, `affected_areas_json`, `affected_path_candidates_json`, `constraints_json`, `autonomy_boundary_json`, and `completion_policy_json`.
- `user_judgments` request, context, option, affected-ref, artifact-ref, `sensitive_action_scope_json`, and `resolution_json` columns.
- `write_authorizations.attempt_scope_json`.
- `runs` observed-attempt and evidence-update JSON columns.
- `evidence_summaries.coverage_items_json` and supporting/gap ref arrays.
- `blockers.owner_ref_json` and `blockers.related_refs_json`.
- `task_events.payload_json`.
- `tool_invocations.response_json`.

### JSON not stored

Rule:

- Task and Change Unit shaping JSON stores compact summaries and bounded lists only.

Not allowed:

- It must not store any of these under another name:
  - standalone Discovery Brief
  - Question Queue
  - Assumption Register
  - full design artifact
  - generated projection body
  - expanded evidence-package body
  - QA record
  - acceptance record
  - residual-risk record
  - close record

## Baseline / Out-of-Scope Boundary

### Conditions

Rule:

- Profile-gated storage is outside the baseline.

Exceptions:

- An owner document may promote a narrow behavior with scope, fallback behavior, and proof-path expectations.

Not allowed:

- Reference-schema presence alone does not make storage active.

### Not stored

Unless [Scope](scope.md) and the storage owners define a supported contract, the baseline excludes storage for capability families outside the supported scope, generated operational outputs, expanded evidence packages, hosted services, cross-surface orchestration, and long-term design-support records.

### Non-claim

Allowed:

- Active status, close readiness, run/evidence summaries, next actions, readable cards, `agent-context-packet`, and guarantee display are read-time derived views over active persisted records.

Exceptions:

- These views may be stale, absent, failed, or recomputed without changing storage authority.

Not allowed:

- These views do not change storage authority.

## Related owners

- [Storage Effects](storage-effects.md) for which methods create, update, observe, or leave records untouched.
- [Artifact Storage](storage-artifacts.md) for artifact-specific storage lifecycle.
- [Storage Versioning](storage-versioning.md) for clocks, idempotency, locks, and migration semantics.
- [API Methods](api/methods.md) and method owner documents for public method behavior that uses records.
- [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Artifact Schemas](api/schema-artifacts.md), [API Judgment Schemas](api/schema-judgment.md), and [API Value Sets](api/schema-value-sets.md) for request/response shape and public API values.
- [Template Bodies](template-bodies.md) for user-visible status cards, judgment requests, run/evidence summaries, close results, and agent context packets.
- [Projection Authority Reference](projection-and-templates.md) for read-only projection authority, source records, and freshness boundaries.
- [Runtime Boundaries](runtime-boundaries.md) for Runtime Home and Product Repository boundaries.
- [Security](security.md) for security non-claims, guarantee levels, and security meanings this document does not assert.
