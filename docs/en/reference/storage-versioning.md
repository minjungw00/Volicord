# Storage versioning

This document owns state versioning, idempotency, event meaning, locks, and migration semantics for current MVP storage source design. It is documentation source material only and does not run migrations, create runtime locks, or create runtime state.

## Owns / Does not own

This document owns:

- the public project-wide `project_state.state_version` conflict basis
- state-version increment rules at the storage-semantics level
- idempotency and request-hash replay semantics
- event meaning for `task_events`
- lock policy
- migration semantics and active/later migration boundaries
- failure and retry interpretation for state versions and idempotency keys

This document does not own:

- record layout or DDL; see [Storage Records](storage-records.md)
- which method branch produces an effect; see [Storage Effects](storage-effects.md) and [MVP API](api/mvp-api.md)
- public error codes and precedence; see [API Errors](api/errors.md)
- artifact lifecycle; see [Artifact Storage](storage-artifacts.md)
- security guarantee wording; see [Security](security.md)
- runtime deployment or operational commands

## Project-wide state clock

The active current MVP has one public state clock: `project_state.state_version`. It is project-wide and is the only active authorization, conflict, freshness, and concurrency basis for public API mutations. Task routing still matters for ownership, blockers, close state, evidence, and user judgments, but it does not select a separate state clock.

Fresh non-dry-run state-changing API calls compare `ToolEnvelope.expected_state_version` with the current `project_state.state_version` before commit. A mismatch returns `STATE_VERSION_CONFLICT` only in `ToolRejectedResponse.errors` and creates no `CloseReadinessBlocker`, current record, `task_event` or `task_events` append, artifact, evidence summary, Write Authorization creation or consumption, `close_state` mutation, replay row, or `project_state.state_version` increment.

`STATE_VERSION_CONFLICT` is the only active current MVP public `ErrorCode` for project-wide state-version mismatch. No alternate public code, alias, deprecated spelling, or storage-layer public error name is exposed for that mismatch. No active current MVP call requires or accepts more than one public `expected_state_version`.

Every committed non-dry-run mutation increments `project_state.state_version` by exactly 1. This includes committed blocked responses when the method owner allows Core to persist a blocker or another current-row mutation. A single public call may update Task lifecycle fields and project-level fields together, such as `harness.close_task intent=supersede` updating both `tasks.lifecycle_phase` and `project_state.active_task_id`, but it is still one mutation and creates exactly one project-wide version increment.

`harness.status`, `harness.close_task intent=check`, the same check with `dry_run=true`, `ToolDryRunResponse` preview calls, malformed requests, pre-commit validation failures, pre-commit state-version conflicts, and idempotent replay do not increment `project_state.state_version`.

Response-branch `state_version` values always use the project-wide version: the resulting version after a committed mutation, or the current project-wide version observed for read-only results, `ToolDryRunResponse` previews, and temporary staging responses.

The active first schema should omit `tasks.state_version`. If an implementation encounters a legacy or prototype `tasks.state_version` column, that value is inactive metadata only. It must not be used as an authorization, `STATE_VERSION_CONFLICT`, stale-state, Write Authorization, idempotency, lock, or concurrency basis.

`write_authorizations.basis_state_version` stores the project-wide `project_state.state_version` used when Core prepared the authorization. Stale Write Authorization detection compares that stored value with the current project-wide state version, not with any Task-local clock. When that mismatch is surfaced through the public API, the public error is `STATE_VERSION_CONFLICT`. The call is rejected before consumption and must not change the Write Authorization status unless another current contract explicitly says so.

`tool_invocations.basis_state_version` stores the project-wide state version observed by the call before the committed mutation. `task_events.state_version` stores the resulting project-wide version after the committed event.

## Event meaning

`task_events` records committed Core mutations in order. It is an audit and ordering trail, not the normal source used to reconstruct current state during ordinary operation. Current rows such as `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, and `blockers` remain the current state.

`task_events` is append-only for ordinary active MVP operation. After an event is committed, Core must not update or delete that row to change history. Corrections or repairs are recorded by new events and current-row updates through the owner path. Idempotent replay, dry-run, malformed requests, and pre-commit failures do not append events.

For a new committed non-dry-run mutation, these effects must commit atomically:

- current-row writes
- `task_events` append
- project-wide state-version increment
- `tool_invocations` replay-row insert

For `harness.record_run`, the same transaction also includes:

- staged-handle consumption in `artifact_staging`
- artifact promotion/linking
- evidence update
- Write Authorization consumption
- event append
- replay-row insert
- exactly one `project_state.state_version` increment

If any part fails, the transaction must leave no partial:

- authority row
- staging consumption
- persistent artifact promotion/linking
- Write Authorization consumption
- evidence update
- event
- close effect
- replay row
- state-version increment

## Idempotency and replay

`tool_invocations` stores exact replay only for committed non-dry-run Core `MethodResult` responses whose method state-effect row creates replay. It does not store `ToolRejectedResponse`, `ToolDryRunResponse`, read-only results, or successful `StageArtifactResult` staging results, and those branches never create or reserve replay rows.

The storage unique key is `(project_id, tool_name, idempotency_key)`. `request_hash` is the conflict discriminator stored in that row. `request_hash` must not be added to a second uniqueness key that would allow the same idempotency key to fork into multiple committed responses.

If the same key and request hash are replayed, Core returns the original committed response without appending events, promoting or linking artifacts, consuming Write Authorization, or changing state again. If the key is reused with a different request hash, Core returns `STATE_VERSION_CONFLICT` as defined by [state version conflict](api/errors.md#state-conflict-behavior).

`tool_invocations.response_json` stores only the exact committed non-dry-run Core `MethodResult` response for a replay-row-creating state effect. It does not store `StatusResult`, `ToolRejectedResponse`, `ToolDryRunResponse`, read-only `MethodResult` results, or successful `StageArtifactResult` staging results.

## Lock policy

Runtime mutations serialize through Core-owned state-changing paths, with ordinary SQLite transactions and a process/project lock if needed. Authority placement is owned by [Runtime Boundaries](runtime-boundaries.md).

The active current MVP does not require a `persistent_locks` table. Durable lock/recovery metadata is later operations material until an owner promotes it.

Locks protect concurrent state writes. Security guarantee wording and non-claims belong to [Security](security.md).

## Migration boundary

No migration runner exists in this repository, and no runtime data exists to migrate. This document does not define migration steps for existing runtime data. Before runtime implementation, maintainers must separately accept the actual DDL, migration mechanism, storage profile, and tightening behavior.

The active migration boundary is:

- Store schema/profile version in Runtime Home metadata and `project_state`, or an equivalent maintainer-accepted mechanism.
- Each future migration must declare a source version, target version, storage profile, owner, and rollback or repair expectation before it is accepted.
- Run future migrations transactionally for `registry.sqlite` or one `state.sqlite` at a time, with a clear interrupted-state recovery rule before runtime implementation.
- Validate owner-shaped JSON before commit and before tightening constraints.
- Treat unknown owner-bound status or enum values as invalid until an owner defines them.
- Tighten nullable fields, foreign keys, enum checks, and JSON validation only after existing rows have been validated or routed to an owner-defined repair state.
- Preserve `task_events.event_seq` ordering when `task_events` is retained.
- Preserve artifact hashes and owner links, or mark affected refs invalid for recovery.
- Preserve committed `tool_invocations` replay rows so idempotency does not fork after migration.
- Keep status cards, compact views, projection freshness, close readiness, and report prose derived from current records at read time. They are not migration authority, repair input, or storage mutation paths.

This document intentionally excludes inactive DDL bundles, migration catalogs, and profile-specific migration details.

## Failures and retry

Pre-commit failures have no storage effect. Stale `expected_state_version`, stale `WriteAuthorization.basis_state_version`, validation failure, malformed request, and idempotency request-hash conflict end in `ToolRejectedResponse` before commit and do not increment `state_version`.

Transaction failures must leave no partial result. If any part of a new committed `dry_run=false` state change fails, storage must not partially leave current-row writes, events, replay rows, artifact effects, Write Authorization consumption, evidence updates, close effects, or a `state_version` increment.

Retry rules depend on the failure type:

| Situation | Retry method | Note |
|---|---|---|
| Stale `expected_state_version` | Read current state again and send a new request with the latest `project_state.state_version`. | This is a freshness check only; it does not replace user-owned judgment. |
| Transport uncertainty for the same request | Retry with the same `idempotency_key` and same `request_hash`. | If the original committed, the original response is returned as replay and the state change is not repeated. |
| Different request with the same `idempotency_key` | Do not retry; use a new idempotency key. | Same key with a different `request_hash` is `STATE_VERSION_CONFLICT`. |
| Pre-commit validation failure | Fix the request and send a new request. | The failed request did not create a replay row. |

Retry does not lower user-judgment boundaries. If a new acceptance, sensitive-action approval, residual-risk acceptance, or `Write Authorization` is needed after failure, the owning route must be used again.

## Related owners

- [API Errors](api/errors.md) for public conflict errors such as `STATE_VERSION_CONFLICT`.
- [Storage Effects](storage-effects.md) for branches that increment or do not increment state.
- [Storage Records](storage-records.md) for columns that store versioning or replay data.
- [Artifact Storage](storage-artifacts.md) for artifact lifecycle and retention boundaries.
- [Runtime Boundaries](runtime-boundaries.md) for Runtime Home separation.
