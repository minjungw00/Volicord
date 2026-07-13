# Storage Versioning

This document owns baseline storage-versioning rules for current Volicord SQLite storage. It does not define public API behavior, Core authority meaning, security guarantees, schema conversion chains, or compatibility conversion for old Runtime Homes.

## Storage Profile

The current baseline storage profile is `baseline_sqlite_v4`.

Baseline storage uses the canonical SQL sources [`registry.sql`](../../../crates/volicord-store/src/schema/registry.sql) and [`project.sql`](../../../crates/volicord-store/src/schema/project.sql). Runtime Home initialization applies those sources to empty SQLite databases. Baseline storage does not create `schema_migrations`, `schema_version`, `migration_version`, `storage_version`, or equivalent storage-version fields.

A database is usable only when its table shape, columns, indexes, foreign keys, constraints, and stored `storage_profile` match the current baseline. These conditions make storage or runtime unavailable:

- an unknown table that represents an old schema ledger
- a missing required table
- a forbidden storage-version column
- a storage-profile mismatch
- a malformed required record

Store code must not guess record meaning, silently rewrite data, or convert unsupported storage. Existing Runtime Homes with incompatible storage fail clearly and require Runtime Home recreation.

Baseline registry storage includes Runtime Home identity, installation profile records, repository-root-based project registrations, project aliases, Agent Connection records, `connection_projects`, and `guard_installations`. Baseline project-state storage includes Core state projection records, `authority_events`, replay rows, staged artifacts, persistent artifacts, evidence, evidence-capture intents, receipts, exclusive source claims, immutable evidence producers, user judgments, `local_web_consent_tokens`, runs, blockers, `write_tickets`, host-observation records, and session-watch records.

`baseline_sqlite_v4` adds the canonical evidence-capture intent, durable
source-fact receipt, exclusive source-claim, and immutable producer record families. The baseline provides no
in-place conversion from `baseline_sqlite_v3`. A v3 Runtime Home is an
incompatible shape and must be recreated; Store must not relabel or
guess-convert it. `project_state.state_version` remains a Core state clock and
is not the storage-profile version.

## Project State Version

`project_state.state_version` is the project-wide Core state clock for committed authority state changes. It is not a schema version, migration version, storage version, or compatibility marker.

It increments only when a complete owner-allowed state-changing transaction commits. It does not increment for rejected requests, dry-run responses, read-only results, startup checks, host verification, schema initialization, storage-profile validation, lock acquisition, status projection, rendered reports, or failed transactions.

Every newly committed authority mutation appends at least one durable `authority_events` row in the same transaction as the current projection updates. A normal committed mutation appends exactly one authority event. If an owner explicitly defines an event batch, all rows in that batch share the single resulting `project_state.state_version` for that committed state transition.

`tasks.state_version` is not a baseline authority field. A non-baseline `tasks.state_version` column is invalid storage shape and must not be used as a conflict, freshness, lock, or write-ticket basis.

Related fields:

- `write_tickets.basis_state_version` stores the resulting `project_state.state_version` after the write-ticket issuance commit. Core uses it as the freshness basis for later write-ticket consumption.
- `tool_invocations.basis_state_version` stores the project-wide state version observed before the committed mutation.
- `authority_events.state_version` stores the resulting project-wide version after the committed authority event or event batch.

## Write Tickets

A write ticket is Volicord authority for authorized write intent for one proposed product-file write attempt. It is not OS permission, OS sandboxing, a filesystem ACL, network policy, secret isolation, global filesystem interception, or proof that a write occurred.

Write-ticket issuance and compatibility consumption follow normal state-version rules:

- issuance can commit only through an owner-defined method branch
- consumption can commit only when the stored physical `write_tickets` row for the write ticket is active, compatible, unexpired, unconsumed, and current for the project state basis
- stale `WriteTicket.basis_state_version` is rejected before consumption
- issuance or consumption never occurs on rejected, dry-run, or replay-only branches

## Idempotency And Replay

`tool_invocations` stores exact replay only for committed `dry_run=false` Core `MethodResult` responses whose method owner creates a replay row.

The storage unique key is exactly `(project_id, tool_name, idempotency_key)`. `request_hash` is the conflict discriminator for the public request payload. It does not absorb invocation context such as `actor_source`, `operation_category`, `connection_id`, or `verification_basis`.

New replay rows store complete non-null `actor_source` and `operation_category` from the verified invocation context. When the verified invocation carries a Git workspace context, the row also stores its canonical JSON as `git_workspace_context_json`; absence is stored as `null`. A current replay row requires complete matching `actor_source` and `operation_category` plus an exact match between the stored and current optional Git workspace contexts. Missing required replay identity is invalid stored state, not a compatibility projection.

Replay eligibility:

- a stored response must never be returned before the current invocation has a verified invocation context
- Core checks invocation-context compatibility before request-hash compatibility
- incompatible context, including a changed or newly absent/present Git workspace context, returns `INVOCATION_CONTEXT_MISMATCH` and must not expose the stored response
- compatible context plus the same `idempotency_key` and same `request_hash` returns the stored original committed response exactly
- compatible context plus the same `idempotency_key` and a different `request_hash` returns `STATE_VERSION_CONFLICT`

Replay uses the stored response body. It does not recompute or reclassify `write_ticket_effect`, `base.state_version`, `base.events`, or any other response field. Replay does not append events, promote or link artifacts, issue or consume write tickets, create another replay row, or change state again.

<a id="exact-operation-result-retrieval"></a>
### Exact operation-result retrieval

Every eligible `operation_category=agent_workflow` Core commit and exact replay
exposes an `OperationResultRef` for the immutable
`tool_invocations.response_json` already stored by the original commit.
`volicord.get_operation_result` reads that value in contiguous UTF-8-safe pages.
Concatenating the pages in cursor order must
reproduce the stored response byte-for-byte; retrieval must not recompute,
normalize, reserialize, or reclassify any field.

Before returning any page, Store loads the exact stored bytes and computes their
byte length and SHA-256; Core compares those facts with the reference. The
current verified actor and project access are checked separately under the
security and method owners; the reference is not a bearer credential.
`operation_category=user_only` rows, including
`volicord.record_user_judgment`, are not eligible for Agent Connection
retrieval. The retrieved response is historical and does not replace a current
`volicord.status` read.

This path reuses the existing replay row and immutable `response_json`. It adds
no table, column, replay form, schema ledger, storage profile, or migration.
Retrieval itself is read-only and creates no replay row, event, lock-visible
state transition, or `project_state.state_version` increment.

`volicord.stage_artifact` remains outside the Core replay transaction: it creates
no replay row and exposes no `OperationResultRef`. Its complete serialized result
must satisfy the supported prospective size bound before any staging side effect
occurs.

## Failure And Retry

Pre-commit failures have no storage effect. Transaction failures must leave no partial state-version increment, event, replay row, write-ticket change, artifact effect, evidence update, judgment effect, close effect, lifecycle effect, or staged-handle consumption.

Examples:

- stale `expected_state_version`
- stale `WriteTicket.basis_state_version`
- validation failure
- malformed request
- corrupt typed owner state
- idempotency request-hash conflict
- invocation-context mismatch
- incompatible existing storage shape

Retry follows the rejected reason: refresh state for stale version conflicts, fix invalid input for validation failures, use the User Channel for missing user judgments, use the required write-ticket flow when write compatibility is still needed, or recreate the Runtime Home when storage is incompatible.

## Owner Links

- Record-family overview and storage-owned values: [Storage Records](storage-records.md)
- SQLite DDL, constraints, indexes, and foreign keys: [Storage DDL](storage-ddl.md)
- Method storage effects: [Storage Effects](storage-effects.md)
- Public conflict behavior: [API error precedence](api/error-precedence.md#state-conflict-behavior)
- Public invocation-context mismatch code: [API error codes](api/error-codes.md#errorcode-invocation-context-mismatch)
- Runtime Home separation: [Runtime Boundaries](runtime-boundaries.md)
