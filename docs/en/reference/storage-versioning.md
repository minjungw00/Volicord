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

The active current MVP has one public state clock: `project_state.state_version`. It is project-wide and is the only active authorization, conflict, freshness, and concurrency basis for public API mutations.

Task routing still matters for ownership, blockers, close state, evidence, and user judgments. It does not select a separate Task-local state clock.

Response-branch `state_version` values always use the project-wide version:

- the resulting version after a committed mutation
- the current project-wide version observed for read-only results, `ToolDryRunResponse` previews, and temporary staging responses

This summary table shows branch-level outcomes. Detail blocks keep conditions, results, and exceptions separate.

| Situation | Result | Details |
|---|---|---|
| Read-only status | no increment | [Read-only status](#state-version-read-only-status) |
| Rejected response | no increment | [Rejected response](#state-version-rejected-response) |
| Successful mutation | increments | [Successful mutation](#state-version-successful-mutation) |
| Committed blocked result | method-specific | [Committed blocked result](#state-version-committed-blocked-result) |

<a id="state-version-read-only-status"></a>
**Read-only status**

Condition:

- A read-only call such as `harness.status` observes current state.

Result:

- `project_state.state_version` does not increment.

Not allowed storage effects:

- current record creation or mutation
- event append
- replay row creation

<a id="state-version-rejected-response"></a>
**Rejected response**

Condition:

- `ToolRejectedResponse` returns before commit.

Result:

- The requested state change is not performed.
- `project_state.state_version` does not increment.

<a id="state-version-successful-mutation"></a>
**Successful mutation**

Condition:

- A `dry_run=false` state change commits.

Result:

- Project-wide state changes.
- `project_state.state_version` increments exactly once per commit.

<a id="state-version-committed-blocked-result"></a>
**Committed blocked result**

Condition:

- The method owner allows a blocked result to commit.

Result:

- Whether the blocked result has a state effect is defined by the method owner and [committed blocked result storage effects](storage-effects.md#committed-blocked-result).

Exception:

- A blocked result does not automatically increment `project_state.state_version`.

The active first schema should omit `tasks.state_version`. If an implementation encounters a legacy or prototype `tasks.state_version` column, that value is inactive metadata only. It must not be used as an authorization, `STATE_VERSION_CONFLICT`, stale-state, Write Authorization, idempotency, lock, or concurrency basis.

Related storage fields record the project-wide clock:

- `write_authorizations.basis_state_version` stores the `project_state.state_version` Core used when preparing the authorization.
- `tool_invocations.basis_state_version` stores the project-wide state version observed before the committed mutation.
- `task_events.state_version` stores the resulting project-wide version after the committed event.

## Incrementing cases

Condition: A new `dry_run=false` call commits an actual state change.

Result: `project_state.state_version` increments by exactly 1. If one public call updates Task lifecycle fields and project-level fields together, it is still one state change and one increment. For example, `harness.close_task intent=supersede` may update both `tasks.lifecycle_phase` and `project_state.active_task_id` in the same commit.

Exception: A committed blocked result does not automatically increment. It may increment only when the method owner allows blocker or other current-row mutation storage and [Storage Effects](storage-effects.md) allows a `state_version` effect for that branch.

Owner links: Method-specific persistence effects belong to [Storage Effects](storage-effects.md) and [MVP API](api/mvp-api.md).

## Non-incrementing cases

These branches do not increment `project_state.state_version`:

- `harness.status`
- `harness.close_task intent=check`
- `harness.close_task intent=check` with `dry_run=true`
- `ToolDryRunResponse` preview calls
- malformed requests
- pre-commit validation failures
- pre-commit state-version conflicts
- stale `WriteAuthorization.basis_state_version`
- idempotent replay
- no-effect rejected responses

Result: These branches must not create current records, `task_events`, replay rows, artifact promotion, evidence summaries, `Write Authorization` creation or consumption, `close_state` mutation, or `project_state.state_version` increment.

Exception: Idempotent replay may return an already committed original response. It still creates no new state change, new event, or new `state_version` increment.

Owner links: The detailed branch list and method-specific exceptions belong to [Storage Effects](storage-effects.md).

## `expected_state_version`

Condition: A new `dry_run=false` state-changing API call compares `ToolEnvelope.expected_state_version` with the current `project_state.state_version` before commit.

Result: If the values match, the call may continue to commit after other validation passes. If the values do not match, Core returns `STATE_VERSION_CONFLICT` only in `ToolRejectedResponse.errors`.

Rejected result: A stale-state conflict does not create or change:

- `CloseReadinessBlocker`
- current record
- `task_event` or `task_events` append
- artifact
- evidence summary
- `Write Authorization` creation or consumption
- `close_state` mutation
- replay row
- `project_state.state_version` increment

Non-claim: `expected_state_version` is a freshness condition for stale writes. It does not replace user-owned judgment, sensitive-action approval, final acceptance, residual-risk acceptance, or `Write Authorization`.

Public error boundary: `STATE_VERSION_CONFLICT` is the only active current MVP public `ErrorCode` for project-wide state-version mismatch. No active current MVP call requires or accepts more than one public `expected_state_version`.

Related storage field: Stale Write Authorization detection compares `write_authorizations.basis_state_version` with the current `project_state.state_version`. When that mismatch is surfaced through the public API, the public error is also `STATE_VERSION_CONFLICT`. The call is rejected before consumption and must not change the Write Authorization status unless another current contract explicitly says so.

## Event meaning

`task_events` records committed Core mutations in order. It is an audit and ordering trail, not the normal source used to reconstruct current state during ordinary operation. Current rows remain the current state, including:

- `tasks`
- `change_units`
- `user_judgments`
- `write_authorizations`
- `runs`
- `artifacts`
- `artifact_links`
- `evidence_summaries`
- `blockers`

`task_events` is append-only for ordinary active MVP operation. After an event is committed, Core must not update or delete that row to change history. Corrections or repairs are recorded by new events and current-row updates through the owner path.

Branches that do not append events:

- idempotent replay
- `dry_run`
- malformed requests
- pre-commit failures
- no-effect rejected responses

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

This section explains idempotency and replay meaning.

Condition: `tool_invocations` stores exact replay only for committed `dry_run=false` Core `MethodResult` responses whose API method state-effect row creates replay.

Storage key: The storage unique key is `(project_id, tool_name, idempotency_key)`. `request_hash` is the conflict discriminator stored in that row.

Stored response: `tool_invocations.response_json` stores only the exact committed `dry_run=false` Core `MethodResult` response for a replay-row-creating state effect.

Branches not stored:

- `ToolRejectedResponse`
- `ToolDryRunResponse`
- read-only result
- read-only `MethodResult`
- `StatusResult`
- successful `StageArtifactResult` staging result

Replay result: If the same `idempotency_key` and same `request_hash` are replayed, Core returns the original committed response. It does not append events, promote or link artifacts, consume Write Authorization, or change state again.

Conflict result: If the same `idempotency_key` is reused with a different `request_hash`, Core returns `STATE_VERSION_CONFLICT` as defined by [state version conflict](api/errors.md#state-conflict-behavior).

Non-claim: `request_hash` must not be added to a second uniqueness key that would allow the same idempotency key to fork into multiple committed responses.

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

Retry rules depend on the failure type. The summary table routes to detail blocks.

| Situation | Retry route |
|---|---|
| Stale `expected_state_version` | [Stale `expected_state_version`](#retry-stale-expected-state-version) |
| Transport uncertainty for the same request | [Transport uncertainty](#retry-transport-uncertainty) |
| Different request with the same `idempotency_key` | [Different request with same key](#retry-different-request-same-key) |
| Pre-commit validation failure | [Pre-commit validation failure](#retry-pre-commit-validation-failure) |

<a id="retry-stale-expected-state-version"></a>
**Stale `expected_state_version`**

Retry method:

- Read current state again.
- Send a new request with the latest `project_state.state_version`.

Note:

- This is a freshness check only; it does not replace user-owned judgment.

<a id="retry-transport-uncertainty"></a>
**Transport uncertainty**

Retry method:

- Retry with the same `idempotency_key` and same `request_hash`.

Note:

- If the original committed, the original response is returned as replay and the state change is not repeated.

<a id="retry-different-request-same-key"></a>
**Different request with same key**

Retry method:

- Do not retry with the reused key.
- Use a new idempotency key.

Note:

- The same key with a different `request_hash` is `STATE_VERSION_CONFLICT`.

<a id="retry-pre-commit-validation-failure"></a>
**Pre-commit validation failure**

Retry method:

- Fix the request.
- Send a new request.

Note:

- The failed request did not create a replay row.

Retry does not lower user-judgment boundaries. If a new acceptance, sensitive-action approval, residual-risk acceptance, or `Write Authorization` is needed after failure, the owning route must be used again.

## Related owners

- [API Errors](api/errors.md) for public conflict errors such as `STATE_VERSION_CONFLICT`.
- [Storage Effects](storage-effects.md) for branches that increment or do not increment state.
- [Storage Records](storage-records.md) for columns that store versioning or replay data.
- [Artifact Storage](storage-artifacts.md) for artifact lifecycle and retention boundaries.
- [Runtime Boundaries](runtime-boundaries.md) for Runtime Home separation.
