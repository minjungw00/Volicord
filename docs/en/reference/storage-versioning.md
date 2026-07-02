# Storage Versioning

This document owns the baseline storage-versioning rules for current Volicord SQLite storage. It does not define public API behavior, Core authority meaning, security guarantees, or migration behavior outside the supported baseline.

## Storage Profile

The current baseline storage profile is `baseline_sqlite_v3`.

Registry storage and project-state storage record their own migration ledger rows. A database is current only when its schema version, migration names, database kind, and storage profile match the compiled baseline. Unknown newer versions, missing migration rows, partial ledgers, migration-name mismatch, and storage-profile mismatch are storage/runtime-unavailable conditions. Store code must not guess record meaning, silently rewrite data, or convert unsupported profiles.

Baseline registry storage includes Runtime Home identity, installation profile
records, repository-root-based project registrations, project aliases, Agent
Connection records, `connection_projects`, `guard_installations`, and
`local_web_consent_tokens`. Baseline project-state storage includes Core state
projection records, `authority_events`, replay rows, staged artifacts,
persistent artifacts, evidence, user judgments, runs, blockers, `write_checks`,
host-observation records, and session-watch records.

## Project State Version

`project_state.state_version` is the project-wide Core state clock for committed authority state changes.

It increments only when a complete owner-allowed state-changing transaction commits. It does not increment for rejected requests, dry-run responses, read-only results, startup checks, host verification, migration metadata, lock acquisition, status projection, rendered reports, or failed transactions.

Every newly committed authority mutation appends at least one durable `authority_events` row in the same transaction as the current projection updates. A normal committed mutation appends exactly one authority event. If an owner explicitly defines an event batch, all rows in that batch share the single resulting `project_state.state_version` for that committed state transition.

`tasks.state_version` is not a baseline authority field. A non-baseline `tasks.state_version` column is ignored metadata only and must not be used as a conflict, freshness, lock, or write-ticket basis.

Related fields:

- `write_checks.basis_state_version` stores the resulting `project_state.state_version` after the write-ticket issuance commit. Core uses it as the freshness basis for later write-ticket compatibility consumption.
- `tool_invocations.basis_state_version` stores the project-wide state version observed before the committed mutation.
- `authority_events.state_version` stores the resulting project-wide version after the committed authority event or event batch.

## Write Tickets

A write ticket is Volicord authority for authorized write intent for one proposed product-file write attempt. It is not OS permission, OS sandboxing, a filesystem ACL, network policy, secret isolation, global filesystem interception, or proof that a write occurred.

Write-ticket issuance and compatibility consumption follow normal state-version rules:

- issuance can commit only through an owner-defined method branch
- consumption can commit only when the stored write-ticket compatibility row is active, compatible, unexpired, unconsumed, and current for the project state basis
- stale `WriteCheck.basis_state_version` is rejected before consumption
- issuance or consumption never occurs on rejected, dry-run, or replay-only branches

## Idempotency And Replay

`tool_invocations` stores exact replay only for committed `dry_run=false` Core `MethodResult` responses whose method owner creates a replay row.

The storage unique key is exactly `(project_id, tool_name, idempotency_key)`. `request_hash` is the conflict discriminator for the public request payload. It does not absorb invocation context such as `actor_source`, `operation_category`, `connection_id`, or `verification_basis`.

New replay rows store complete non-null `actor_source` and `operation_category` from the verified invocation context. A current replay row requires complete matching `actor_source` and `operation_category`. Missing required replay identity is invalid stored state, not a compatibility projection.

Replay eligibility:

- a stored response must never be returned before the current invocation has a verified invocation context
- Core checks invocation-context compatibility before request-hash compatibility
- incompatible context returns `INVOCATION_CONTEXT_MISMATCH` and must not expose the stored response
- compatible context plus the same `idempotency_key` and same `request_hash` returns the stored original committed response exactly
- compatible context plus the same `idempotency_key` and a different `request_hash` returns `STATE_VERSION_CONFLICT`

Replay uses the stored response body. It does not recompute or reclassify `write_ticket_effect`, `write_check_effect`, `base.state_version`, `base.events`, or any other response field. Replay does not append events, promote or link artifacts, issue or consume write tickets, create another replay row, or change state again.

## Failure And Retry

Pre-commit failures have no storage effect. Transaction failures must leave no partial state-version increment, event, replay row, write-ticket change, artifact effect, evidence update, judgment effect, close effect, lifecycle effect, or staged-handle consumption.

Examples:

- stale `expected_state_version`
- stale `WriteCheck.basis_state_version`
- validation failure
- malformed request
- corrupt typed owner state
- idempotency request-hash conflict
- invocation-context mismatch

Retry follows the rejected reason: refresh state for stale version conflicts, fix invalid input for validation failures, use the User Channel for missing user judgments, or use the required write-ticket flow when write compatibility is still needed.

## Migration Boundary

Migration semantics describe how supported storage profile or schema-version changes preserve Core authority records. Supported migration execution exists only when [Scope](scope.md), [Storage Records](storage-records.md), [Storage DDL](storage-ddl.md), and this document define the version, storage profile, validation, preservation, repair, retry, and metadata-advance behavior.

Migration does not create a public `project_state.state_version` increment, Core event, replay record, or public method effect unless a focused owner explicitly defines that effect.

## Owner Links

- Record-family overview and storage-owned values: [Storage Records](storage-records.md)
- SQLite DDL, constraints, indexes, foreign keys, and migration table shape: [Storage DDL](storage-ddl.md)
- Method storage effects: [Storage Effects](storage-effects.md)
- Public conflict behavior: [API error precedence](api/error-precedence.md#state-conflict-behavior)
- Public invocation-context mismatch code: [API error codes](api/error-codes.md#errorcode-invocation-context-mismatch)
- Runtime Home separation: [Runtime Boundaries](runtime-boundaries.md)
