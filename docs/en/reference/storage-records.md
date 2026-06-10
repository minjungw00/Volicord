# Storage records

This document owns persistent storage record layout for the current MVP source design. It is documentation source material only and does not create a runtime database, generated records, migration files, or implementation-complete DDL in this repository.

Persistent records are local records committed by Core for later reads. They are not tamper-proof storage, anti-forgery proof, or external audit guarantees. Security non-claims and guarantee levels belong to [Security](security.md).

## Owns / Does not own

This document owns:

- Runtime Home identity and project-local storage layout assumptions.
- Active persisted record categories and table-level storage roles.
- Record-column meaning for future storage design.
- Storage-owned JSON `TEXT` placement and validation expectations.
- Record-level active/later exclusions.

This document does not own:

- method-to-storage effects; see [Storage Effects](storage-effects.md)
- artifact staging, promotion, linking, body reads, retention, or integrity lifecycle; see [Artifact Storage](storage-artifacts.md)
- `project_state.state_version`, idempotency, event meaning, locks, or migrations; see [Storage Versioning](storage-versioning.md)
- API request or response schemas; see [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Artifact Schemas](api/schema-artifacts.md), and the other API schema owners
- API method behavior; see [MVP API](api/mvp-api.md)
- runtime/repository/server boundaries; see [Runtime Boundaries](runtime-boundaries.md)

## Runtime home layout

Harness uses one local Runtime Home and one project-local state database per registered project. The default reference root is `~/.harness`; an implementation may choose an equivalent configured root.

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
- `artifacts/` is the project artifact store. `artifacts/tmp/` is temporary staging space, not evidence authority.

`project.yaml` must not store current Task state, gates, Write Authorization state, evidence sufficiency, final acceptance, residual-risk acceptance, or close state.

Runtime Home identity must not depend only on a filesystem path. A copied or moved Runtime Home may carry the same stored `runtime_home_id`; a newly created Runtime Home gets a new id. The id helps detect suspicious copies, duplicate registrations, or path drift, but it is not a security guarantee.

Runtime Home files are local operational control data and may contain sensitive support data. Security non-claims and guarantee levels belong to [Security](security.md); location boundaries belong to [Runtime Boundaries](runtime-boundaries.md).

## Persisted record categories

The active current MVP persists only the Core records needed by the active state-changing method set: `harness.intake`, `harness.update_scope`, `harness.prepare_write`, `harness.record_run`, `harness.request_user_judgment`, `harness.record_user_judgment`, and state-changing `harness.close_task` intents. `harness.status` and `harness.close_task intent=check` are read-only.

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

The active temporary storage boundary is `artifact_staging` or an equivalent storage-owned staging manifest, together with safe temporary bytes or notices under `artifacts/tmp/`. This is storage placement only; staging lifecycle, provenance, consumption, and promotion belong to [Artifact Storage](storage-artifacts.md).

No other persisted table family or temporary handle family is active current MVP scope. Requirement shaping persists through `tasks`, `change_units`, `user_judgments`, `evidence_summaries`, and `blockers`; it is not a separate committed Discovery Brief, Shared Design, Question Queue, Assumption Register, or First Safe Change Unit Candidate table. Evidence persists through compact evidence summaries, `CompletionPolicy` on the Task or Change Unit, required coverage items, and artifact refs, not through full Evidence Manifest storage.

Projection has no active persisted table family. `status-card`, `judgment-request`, `run-evidence-summary`, `close-result`, and `agent-context-packet` are read-time views over active records, not stored state or storage mutation paths.

The minimum active shaping information is stored through existing records: current goal summary, active scope summary, allowed paths or affected areas, non-goals, acceptance criteria, Autonomy Boundary, required user-owned judgments, one blocking question when necessary, one next safe action, `CompletionPolicy`, required evidence expectation or evidence gap, and close readiness. Missing or unknown pieces stay as `unknown`, pending `user_judgments`, evidence gaps, or `blockers`; storage must not create extra active planning tables to make the request appear ready.

## Table overview

This table names active storage tables and the minimum storage role they serve. It is not full DDL and does not duplicate API schemas.

| Table or file | Location | Active role | Essential stored fields |
|---|---|---|---|
| Runtime Home identity | `registry.sqlite` | Identify the local Runtime Home and schema/storage profile. | `runtime_home_id`, `schema_version`, `storage_profile`, `created_at`, `updated_at`. |
| Project registration | `registry.sqlite` | Map a registered project to its project-local storage. | `project_id`, `repo_root`, `project_home`, `display_name`, `status`, `created_at`, `updated_at`. |
| `project.yaml` | Project directory | Static project configuration. | `project_id`, `repo_root`, display/config defaults. |
| `project_state` | `state.sqlite` | Project-local state header, single public project-wide state clock, active Task pointer, and default surface pointer. | `project_id`, `schema_version`, `storage_profile`, `state_version`, `active_task_id`, `default_surface_id`, `created_at`, `updated_at`. |
| `surfaces` | `state.sqlite` | Stored `LocalSurfaceRegistration` facts used to verify a local surface context for API access. The row is registration data, not live proof that the current caller is trusted. | `project_id`, `surface_id`, `surface_instance_id`, `transport_kind`, `transport_binding_fingerprint`, `access_secret_hash`, `capability_profile_hash`, `capability_profile_json`, `status`, `local_access_posture`, `registered_at`, `last_verified_at`, `updated_at`. |
| `tasks` | `state.sqlite` | User-value work unit, shaping summary, lifecycle, result, next action, active Task-level `CompletionPolicy`, and close fields. | `task_id`, `project_id`, `title`, `user_request`, `current_goal_summary`, `mode`, `lifecycle_phase`, `close_reason`, `result`, `summary`, shaping JSON columns, `completion_policy_json`, `blocking_question`, `next_safe_action`, `active_change_unit_id`, `created_at`, `updated_at`, `closed_at`. |
| `change_units` | `state.sqlite` | Current or proposed scoped work boundary for write compatibility, Change Unit-level `CompletionPolicy`, and close basis. | `change_unit_id`, `task_id`, `scope_summary`, scope JSON columns for allowed paths or affected areas, `baseline_ref`, `autonomy_boundary_json`, `completion_policy_json`, `status`, `created_at`, `updated_at`. |
| `user_judgments` | `state.sqlite` | User-owned judgment records, including separate sensitive-action approval scope when relevant. | `user_judgment_id`, `task_id`, `change_unit_id`, `judgment_kind`, `presentation`, `status`, request/context JSON columns, `question`, `sensitive_action_scope_json`, `resolution_json`, `expires_at`, `resolved_at`, `created_at`, `updated_at`. |
| `write_authorizations` | `state.sqlite` | Durable single-use cooperative Write Authorization created only by non-dry-run `prepare_write` with `decision=allowed`. | `write_authorization_id`, `task_id`, `change_unit_id`, `surface_id`, `status`, `basis_state_version`, `attempt_scope_json`, `consumed_by_run_id`, `expires_at`, `created_at`, `updated_at`, `consumed_at`. |
| `runs` | `state.sqlite` | Committed execution or observation record, including compatible Write Authorization consumption when a product write happened. | `run_id`, `task_id`, `change_unit_id`, `write_authorization_id`, `surface_id`, `kind`, `status`, `product_write`, `baseline_ref`, `summary`, observed/evidence JSON columns, `created_at`, `completed_at`. |
| `artifact_staging` | `state.sqlite` plus `artifacts/tmp/` | Temporary staged safe bytes or safe notices created by `harness.stage_artifact` for later single-use `harness.record_run` consumption. | `handle_id`, `project_id`, `task_id`, `created_by_surface_id`, `created_by_surface_instance_id`, `display_name`, `relation_hint`, `tmp_uri`, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `status`, `consumed_by_run_id`, `promoted_artifact_id`, `expires_at`, `created_at`, `consumed_at`. |
| `artifacts` | `state.sqlite` plus artifact store | Registered durable evidence bytes or safe metadata with integrity, redaction, producer, retention, and availability facts. | `artifact_id`, `project_id`, `task_id`, `run_id`, `kind`, `uri`, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `retention_class`, `produced_by`, `status`, `created_at`, `updated_at`. |
| `artifact_links` | `state.sqlite` | Owner relation from an artifact to the active Core/API record it supports. | `artifact_link_id`, `artifact_id`, `task_id`, `owner_record_kind`, `owner_record_id`, `relation`, `created_at`. |
| `evidence_summaries` | `state.sqlite` | Compact evidence coverage and gap record used by status, run/evidence summaries, blockers, and close. | `evidence_summary_id`, `task_id`, `change_unit_id`, `status`, `coverage_items_json`, `summary`, `supporting_run_ids_json`, `supporting_artifact_link_ids_json`, `gap_blocker_ids_json`, `updated_at`. |
| `blockers` | `state.sqlite` | Structured blocker state for next action, write compatibility, evidence gaps, close readiness, or recovery. `CloseReadinessBlocker` is the API data shape and is not itself a row or persistence signal. | `blocker_id`, `task_id`, `blocked_action`, `blocker_kind`, `status`, `message`, `owner_ref_json`, `related_refs_json`, `required_next_action`, `created_at`, `resolved_at`. |
| `task_events` | `state.sqlite` | Append-only audit and ordering trail for committed Core mutations. | `event_id`, `project_id`, `task_id`, `event_seq`, `event_type`, `state_version`, `actor_kind`, `surface_id`, `payload_json`, `created_at`. |
| `tool_invocations` | `state.sqlite` | Replay row only for committed non-dry-run Core `MethodResult` responses whose method state-effect row creates replay. | `invocation_id`, `project_id`, `tool_name`, `idempotency_key`, `request_hash`, `task_id`, `basis_state_version`, `response_json`, `status`, `created_at`. |

## First schema integrity contract

This section is a first-implementation storage contract for future SQLite schema design. It is not full DDL, a migration file, or proof that runtime implementation has started.

For a first SQLite schema, committed rows must preserve these identities, value sets, relations, and transaction boundaries. Implementations may choose `CHECK` constraints, lookup tables, generated columns, triggers, or Core-side validation, but Core/API/storage owner validation remains required.

Required identity and uniqueness constraints:

- Active tables use opaque stable ids as primary keys or equivalent unique keys: `project_id`, `surface_id`, `surface_instance_id`, `task_id`, `change_unit_id`, `user_judgment_id`, `write_authorization_id`, `run_id`, `handle_id`, `artifact_id`, `artifact_link_id`, `evidence_summary_id`, `blocker_id`, `event_id`, and `invocation_id`.
- Runtime Home identity stores one `runtime_home_id` for the Runtime Home.
- Project registration requires unique `project_id`, unique `project_home`, and one active registration for a `repo_root` unless a future owner defines multi-registration behavior.
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

Ordinary active MVP Core operations do not hard-delete authority rows. They move rows through status or lifecycle fields, append events, and keep replay and artifact metadata available for audit and recovery. Foreign keys should default to `RESTRICT` or equivalent owner validation for authority rows. Closing, cancelling, or superseding a Task must not cascade-delete `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers`, `task_events`, or `tool_invocations`.

Unconsumed or expired `artifact_staging` rows and `artifacts/tmp/` staging bytes or notices may be marked `expired` or `discarded`, and temporary bytes may be cleaned before registration, because they are not evidence authority. Once an `artifacts` row is committed, retention purge, project teardown, or destructive cleanup is outside ordinary active MVP mutation behavior and needs an owner-defined path.

## Storage-owned values and JSON

Closed current MVP storage value sets are table-level persistence constraints. Rows that mirror API schema values must match the API schema owner exactly; rows marked storage-owned define storage behavior that is not a public API schema body. Unknown values fail before commit.

| Field | Current MVP values | Storage rule |
|---|---|---|
| Project registration `status` | `active` | Only registered active projects are in the baseline current MVP. Disable/unregister behavior is later until promoted. |
| `surfaces.transport_kind` | `local_mcp_stdio`, `local_http` | Stored local transport category for registration matching. It is not a socket or protocol setup specification. |
| `surfaces.local_access_posture` | `registered_local`, `unavailable`, `mismatch`, `revoked` | Stored registration posture for API compatibility checks; meanings mirror the API schema owner. |
| `surfaces.status` | `active`, `disabled`, `stale`, `revoked` | Stored surface registration usability; meanings mirror the API schema owner. |
| `tasks.lifecycle_phase` | `shaping`, `ready`, `executing`, `waiting_user`, `blocked`, `completed`, `cancelled`, `superseded` | Persisted Task lifecycle. `intake` is not a stored value; `superseded` is terminal. |
| `tasks.close_reason` | `none`, `completed_self_checked`, `completed_with_risk_accepted`, `cancelled`, `superseded` | Persisted close detail, separate from lifecycle and result. |
| `tasks.result` | `none`, `advice_only`, `completed`, `cancelled`, `superseded` | Persisted coarse outcome. Failed Runs, violations, blocked closes, and evidence gaps stay in their owning records. |
| `change_units.status` | `proposed`, `active`, `replaced`, `closed` | Storage-owned active Change Unit lifecycle for write compatibility and close basis. |
| `write_authorizations.status` | `active`, `consumed`, `expired`, `stale`, `revoked` | Durable authorization lifecycle. Storage owns persistence and transition rules. |
| `artifact_staging.status` | `staged`, `consumed`, `expired`, `discarded` | Storage-owned temporary handle lifecycle. Only `staged` is consumable by `harness.record_run`; terminal values cannot return to `staged`. |
| `artifacts.status` | `available`, `missing`, `integrity_failed`, `unavailable` | Storage-owned artifact availability state. Redaction and blocked-payload handling stay in `redaction_state`. |
| `artifact_links.owner_record_kind` | `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` | Persisted owner relation discriminator; storage owns same-project/same-Task owner lookup and relation validation. |
| `blockers.status` | `active`, `resolved`, `superseded` | Storage-owned blocker row state. Public close blocker shapes remain API-owned. |
| `tool_invocations.status` | `committed` | A replay row exists only for a committed non-dry-run Core `MethodResult` response whose method state-effect row creates replay. |

Other persisted status-like API fields, including `tasks.mode`, `runs.kind`, `runs.status`, `user_judgments.status`, and `evidence_summaries.status`, validate against [API Value Sets](api/schema-value-sets.md) and the Core/API method owners. Storage may index and constrain them, but this document does not redefine their public schema values.

SQLite `TEXT` columns that store JSON are a storage representation choice, not permission to persist arbitrary JSON. Core must parse and validate JSON before commit. API-shaped stored JSON validates against the API schema owners. Storage-only JSON validates against this document or the owner document named by this document. SQLite defaults such as `'{}'` and `'[]'` are storage defaults only; they do not make API fields optional.

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

Task and Change Unit shaping JSON stores compact summaries and bounded lists only. It must not store a standalone Discovery Brief, Question Queue, Assumption Register, full design artifact, generated projection body, evidence manifest body, QA record, acceptance record, residual-risk record, or close record under another name.

## Active / later boundary

Profile-gated later storage is outside the active current MVP unless an owner document promotes a narrow behavior with scope, fallback behavior, and proof-path expectations. Reference-schema presence alone does not make storage active.

The active current MVP excludes storage for projection jobs, durable projection caches, managed-output outboxes, conformance-runner state, fixture execution history, operations-profile storage, `captured_artifact` handles, native capture storage, capture-adapter output tables, full Evidence Manifest tables, detailed evidence catalogs, detached verification, full Manual QA matrices, rich QA/waiver machinery, rich approval or residual-risk lifecycle tables separate from `user_judgments` and `blockers`, dashboards, analytics, hosted connector registries, cross-surface orchestration storage, and long-term design-support storage.

Active status, close readiness, run/evidence summaries, next actions, readable cards, `agent-context-packet`, and guarantee display are read-time derived views over active persisted records. They may be stale, absent, failed, or recomputed without changing storage authority.

## Related owners

- [Storage Effects](storage-effects.md) for which methods create, update, observe, or leave records untouched.
- [Artifact Storage](storage-artifacts.md) for artifact-specific storage lifecycle.
- [Storage Versioning](storage-versioning.md) for clocks, idempotency, locks, and migration semantics.
- [MVP API](api/mvp-api.md) for public method behavior that uses records.
