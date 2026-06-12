# Storage versioning

This document owns state versioning, idempotency, event meaning, locks, and migration semantics for baseline scope storage source design.

## Owns / Does not own

This document owns:

- the public project-wide `project_state.state_version` conflict basis
- state-version increment rules at the storage-semantics level
- idempotency and request-hash replay semantics
- event meaning for `task_events`
- lock policy
- migration semantics and active/out-of-scope migration boundaries
- failure and retry interpretation for state versions and idempotency keys

This document does not own:

- record layout or DDL; see [Storage Records](storage-records.md)
- which method branch produces an effect; see [Storage Effects](storage-effects.md), the [API Methods](api/methods.md), and method owner documents
- public error codes and precedence; see [API Errors](api/errors.md)
- artifact lifecycle; see [Artifact Storage](storage-artifacts.md)
- security guarantee wording; see [Security](security.md)
- runtime deployment or operational commands

## Project-wide state clock

Meaning:

- The baseline has one public state clock: `project_state.state_version`.
- `project_state.state_version` is project-wide and is the only active authorization, conflict, freshness, and concurrency basis for public API mutations.
- Task routing still matters for ownership, blockers, close state, evidence, and user judgments.
- Task routing does not select a separate Task-local state clock.
- A committed mutation response reports the resulting project-wide version.
- Read-only results, `ToolDryRunResponse` previews, and transient staging responses report the current project-wide version they observed.

Increments when:

- A `dry_run=false` state-changing call commits through an owner-allowed branch.

Does not increment when:

- A response only observes state, previews a dry-run effect, stages transient data, or rejects before commit.

Retry behavior:

- Stale writes compare `ToolEnvelope.expected_state_version` with the current `project_state.state_version` before commit.
- Transport uncertainty for a committed state-changing request is handled by idempotency replay, not by adding another state-version increment.

Owner links:

- Branch-level persistence effects belong to [Storage Effects](storage-effects.md) and the method owner documents routed from the [API Methods](api/methods.md).

This summary table shows branch-level outcomes. Detail blocks keep conditions, results, and exceptions separate.

| Situation | Result | Details |
|---|---|---|
| Read-only status | no increment | [Read-only status](#state-version-read-only-status) |
| Rejected response | no increment | [Rejected response](#state-version-rejected-response) |
| Successful mutation | increments | [Successful mutation](#state-version-successful-mutation) |
| Committed blocked result | method-specific | [Committed blocked result](#state-version-committed-blocked-result) |

<a id="state-version-read-only-status"></a>
**Read-only status**

Meaning:

- A read-only call such as `harness.status` observes current state.

Increments when:

- None. A read-only call does not increment by itself.

Does not increment when:

- The call only observes current state.
- The call must not create or mutate current records, append events, or create replay rows.

Retry behavior:

- A repeated read-only call observes the then-current project-wide version; it is not idempotency replay.

Owner links:

- Method-specific no-effect branch details belong to [Storage Effects](storage-effects.md).

<a id="state-version-rejected-response"></a>
**Rejected response**

Meaning:

- `ToolRejectedResponse` returns before commit.

Increments when:

- None. A pre-commit rejection does not increment by itself.

Does not increment when:

- The requested state change is not performed.
- `project_state.state_version` does not increment.

Retry behavior:

- Retry follows the rejected reason: refresh state for stale version conflicts, fix invalid input for validation failures, or use the owner-required route for any judgment or authorization that is still needed.

Owner links:

- Public error-code routing belongs to [API Errors](api/errors.md).
- Branch storage effects belong to [Storage Effects](storage-effects.md).

<a id="state-version-successful-mutation"></a>
**Successful mutation**

Meaning:

- A `dry_run=false` state change commits.

Increments when:

- Project-wide state changes.
- `project_state.state_version` increments exactly once per commit.

Does not increment when:

- The request reaches only a preview, rejection, replay, read-only result, or other no-effect branch.

Retry behavior:

- Retrying the same committed request through idempotency replay returns the original response and does not repeat the state change.

Owner links:

- Method-specific storage effects belong to [Storage Effects](storage-effects.md) and method owner documents.

<a id="state-version-committed-blocked-result"></a>
**Committed blocked result**

Meaning:

- The method owner allows a blocked result to commit.
- Whether the blocked result has a state effect is defined by the method owner and [committed blocked result storage effects](storage-effects.md#committed-blocked-result).

Increments when:

- The method owner allows blocker or other current-row mutation storage and [Storage Effects](storage-effects.md) allows a `state_version` effect for that branch.

Does not increment when:

- A blocked result has no owner-defined state effect.
- A blocked result merely exists; it does not automatically increment `project_state.state_version`.

Retry behavior:

- Follow the method owner and the failure/retry rules for the branch that produced the blocked result.

Owner links:

- Blocked-result storage effects belong to [Storage Effects](storage-effects.md#committed-blocked-result) and the affected method owner.

The active first schema should omit `tasks.state_version`.

If an implementation encounters a legacy or prototype `tasks.state_version` column, that value is ignored metadata only.

`tasks.state_version` must not be used as:

- authorization
- `STATE_VERSION_CONFLICT`
- stale-state basis
- `Write Authorization` basis
- idempotency basis
- lock basis
- concurrency basis

Related storage fields record the project-wide clock:

- `write_authorizations.basis_state_version` stores the `project_state.state_version` Core used when preparing the authorization.
- `tool_invocations.basis_state_version` stores the project-wide state version observed before the committed mutation.
- `task_events.state_version` stores the resulting project-wide version after the committed event.

## Incrementing cases

Meaning:

- An increment means one committed project-wide state change.
- One public call can update Task lifecycle fields and project-level fields together. That is still one state change and one increment.

Increments when:

- A new `dry_run=false` call commits an actual state change.
- `project_state.state_version` increments by exactly 1.
- Example: `harness.close_task intent=supersede` may update both `tasks.lifecycle_phase` and `project_state.active_task_id` in the same commit.

Does not increment when:

- A committed blocked result has no owner-defined state effect.
- A committed blocked result merely exists; it does not automatically increment `project_state.state_version`.

Retry behavior:

- A replay of the already committed response does not create another increment.

Owner links:

- Method-specific persistence effects belong to [Storage Effects](storage-effects.md) and the method owner documents routed from the [API Methods](api/methods.md).

## Non-incrementing cases

Meaning:

- No-effect branches may report an observed `state_version`, but they do not create a new one.

Increments when:

- None. The branches listed in this section do not increment.

Does not increment when:

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

These branches must not create:

- current records
- `task_events`
- replay rows
- artifact promotion
- evidence summaries
- `Write Authorization` creation or consumption
- `close_state` mutation
- `project_state.state_version` increment

Retry behavior:

- Idempotent replay may return an already committed original response.
- Replay still creates no new state change, new event, or new `state_version` increment.

Owner links:

- The detailed branch list and method-specific exceptions belong to [Storage Effects](storage-effects.md).

## `expected_state_version`

Meaning:

- `expected_state_version` is a freshness condition for stale writes.
- A new `dry_run=false` state-changing API call compares `ToolEnvelope.expected_state_version` with the current `project_state.state_version` before commit.
- `expected_state_version` does not replace user-owned judgment, sensitive-action approval, final acceptance, residual-risk acceptance, or `Write Authorization`.

Increments when:

- The values match, other validation passes, and the call subsequently commits an owner-allowed state change.

Does not increment when:

- The values do not match.
- Core returns `STATE_VERSION_CONFLICT` only in `ToolRejectedResponse.errors`.

A stale-state conflict does not create or change:

- `CloseReadinessBlocker`
- current record
- `task_event` or `task_events` append
- artifact
- evidence summary
- `Write Authorization` creation or consumption
- `close_state` mutation
- replay row
- `project_state.state_version` increment

Retry behavior:

- Read current state again.
- Send a new request with the latest `project_state.state_version`.

Public API boundary:

- `STATE_VERSION_CONFLICT` is the only baseline public `ErrorCode` for project-wide state-version mismatch.
- No baseline call requires or accepts more than one public `expected_state_version`.
- When that mismatch is surfaced through the public API, the public error is also `STATE_VERSION_CONFLICT`.

Related storage field:

- Stale `Write Authorization` detection compares `write_authorizations.basis_state_version` with the current `project_state.state_version`.

Owner links:

- Public error-code routing belongs to [API Errors](api/errors.md).

Not allowed:

- The call must be rejected before consumption.
- The call must not change the `Write Authorization` status unless another current contract explicitly says so.

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

`task_events` is append-only for ordinary baseline operation. After an event is committed, Core must not update or delete that row to change history. Corrections or repairs are recorded by new events and current-row updates through the owner path.

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

Artifact lifecycle effects such as staged-handle consumption, artifact promotion, and artifact linking join the same committed transaction only when [Artifact Storage](storage-artifacts.md), [Storage Effects](storage-effects.md), and the method owner allow them.

If any part fails, the transaction must leave no partial:

- authority row
- staging consumption
- persistent artifact promotion/linking
- `Write Authorization` consumption
- evidence update
- event
- close effect
- replay row
- state-version increment

## Idempotency and replay

Meaning:

- `tool_invocations` stores exact replay only for committed `dry_run=false` Core `MethodResult` responses whose API method state-effect row creates replay.
- The storage unique key is `(project_id, tool_name, idempotency_key)`.
- `request_hash` is the conflict discriminator stored in that row.
- `tool_invocations.response_json` stores only the exact committed `dry_run=false` Core `MethodResult` response for a replay-row-creating state effect.

Increments when:

- Only the original committed state-changing request can create a `state_version` increment.
- The replay row is stored with that original committed response.

Does not increment when:

- The same `idempotency_key` and same `request_hash` are replayed.
- Core returns the original committed response.
- Core rejects reuse of the same `idempotency_key` with a different `request_hash`.

Branches not stored:

- `ToolRejectedResponse`
- `ToolDryRunResponse`
- read-only result
- read-only `MethodResult`
- `StatusResult`
- successful `StageArtifactResult` staging result

Retry behavior:

- If the same `idempotency_key` and same `request_hash` are replayed, Core returns the original committed response.
- Replay does not append events, promote or link artifacts, consume `Write Authorization`, or change state again.
- If the same `idempotency_key` is reused with a different `request_hash`, Core returns `STATE_VERSION_CONFLICT` as defined by [state version conflict](api/errors.md#state-conflict-behavior).

Owner links:

- Public conflict behavior belongs to [API Errors](api/errors.md#state-conflict-behavior).
- Branch storage effects belong to [Storage Effects](storage-effects.md).

Non-claim: `request_hash` must not be added to a second uniqueness key that would allow the same idempotency key to fork into multiple committed responses.

## Lock policy

Meaning:

- Runtime mutations serialize through Core-owned state-changing paths.
- Core uses ordinary SQLite transactions and a process/project lock if needed.
- Locks protect concurrent state writes.

Increments when:

- The protected operation commits an owner-allowed state change under the normal `state_version` rules.

Does not increment when:

- Lock acquisition or release does not itself define a public state change.
- The baseline does not require a `persistent_locks` table.
- Durable lock/recovery metadata is reserved operations material until an owner promotes it.

Retry behavior:

- Retrying after transport uncertainty still follows idempotency and state-version rules.
- A lock does not override stale `expected_state_version`, user-owned judgment, or authorization boundaries.

Owner links:

- Authority placement belongs to [Runtime Boundaries](runtime-boundaries.md).
- Security guarantee wording and non-claims belong to [Security](security.md).

## Migration boundary

Meaning:

- Migration semantics describe how accepted storage profile or schema-version changes preserve Core authority records.
- Supported migration execution exists only when [Scope](scope.md) and the affected storage owners define an active path.
- Migration detail must state the version, storage profile, validation, repair, and tightening behavior it owns.

Increments when:

- No public API `state_version` increment is defined for migration unless the migration owner explicitly defines one.
- An accepted migration states its version and storage-profile behavior in its owning documentation.

Does not increment when:

- Status cards, compact views, projection freshness, close readiness, and report prose are derived from current records at read time.
- Derived read-time material is not migration authority, repair input, or a storage mutation path.

Retry behavior:

- Migration repair and retry follow the owner-defined migration path.

Owner links:

- Record layout and DDL belong to [Storage Records](storage-records.md).
- Runtime Home separation belongs to [Runtime Boundaries](runtime-boundaries.md).

The active migration boundary is:

- Store schema/profile version in Runtime Home metadata and `project_state`, or an equivalent maintainer-accepted mechanism.
- Validate owner-shaped JSON before commit and before tightening constraints.
- Treat unknown owner-bound status or enum values as invalid until an owner defines them.
- Tighten nullable fields, foreign keys, enum checks, and JSON validation only after existing rows have been validated or routed to an owner-defined repair state.
- Preserve `task_events.event_seq` ordering when `task_events` is retained.
- Preserve artifact hashes and owner links, or mark affected refs invalid for recovery.
- Preserve committed `tool_invocations` replay rows so idempotency does not fork after migration.

This document intentionally excludes DDL bundles, migration catalogs, and profile-specific migration details outside the supported baseline.

## Failures and retry

Meaning:

- Pre-commit failures have no storage effect.
- Transaction failures must leave no partial result.

Examples:
- stale `expected_state_version`
- stale `WriteAuthorization.basis_state_version`
- validation failure
- malformed request
- idempotency request-hash conflict

Increments when:

- Only a complete committed state-changing transaction increments `state_version`.

Does not increment when:

- These failures end in `ToolRejectedResponse` before commit.
- Any part of a new committed `dry_run=false` state change fails.

If any part of a new committed `dry_run=false` state change fails, storage must not partially leave:

- current-row writes
- events
- replay rows
- artifact effects
- `Write Authorization` consumption
- evidence updates
- close effects
- `state_version` increment

Retry behavior:

- Retry rules depend on the failure type.
- The summary table routes to detail blocks.

| Situation | Retry route |
|---|---|
| Stale `expected_state_version` | [Stale `expected_state_version`](#retry-stale-expected-state-version) |
| Transport uncertainty for the same request | [Transport uncertainty](#retry-transport-uncertainty) |
| Different request with the same `idempotency_key` | [Different request with same key](#retry-different-request-same-key) |
| Pre-commit validation failure | [Pre-commit validation failure](#retry-pre-commit-validation-failure) |

<a id="retry-stale-expected-state-version"></a>
**Stale `expected_state_version`**

Retry behavior:

- Read current state again.
- Send a new request with the latest `project_state.state_version`.

Note:

- This is a freshness check only; it does not replace user-owned judgment.

<a id="retry-transport-uncertainty"></a>
**Transport uncertainty**

Retry behavior:

- Retry with the same `idempotency_key` and same `request_hash`.

Note:

- If the original committed, the original response is returned as replay and the state change is not repeated.

<a id="retry-different-request-same-key"></a>
**Different request with same key**

Retry behavior:

- Do not retry with the reused key.
- Use a new idempotency key.

Note:

- The same key with a different `request_hash` is `STATE_VERSION_CONFLICT`.

<a id="retry-pre-commit-validation-failure"></a>
**Pre-commit validation failure**

Retry behavior:

- Fix the request.
- Send a new request.

Note:

- The failed request did not create a replay row.

Retry does not lower user-judgment boundaries. If a new acceptance, sensitive-action approval, residual-risk acceptance, or `Write Authorization` is needed after failure, the owning route must be used again.

Owner links:

- Public conflict errors belong to [API Errors](api/errors.md).
- Branch storage effects belong to [Storage Effects](storage-effects.md).

## Related owners

- [API Errors](api/errors.md) for public conflict errors such as `STATE_VERSION_CONFLICT`.
- [Storage Effects](storage-effects.md) for branches that increment or do not increment state.
- [Storage Records](storage-records.md) for columns that store versioning or replay data.
- [Artifact Storage](storage-artifacts.md) for artifact lifecycle and retention boundaries.
- [Runtime Boundaries](runtime-boundaries.md) for Runtime Home separation.
