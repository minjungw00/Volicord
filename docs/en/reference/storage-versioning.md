# Storage Versioning

This document owns baseline storage-versioning rules for current Volicord SQLite storage. It does not define public API behavior, Core authority meaning, security guarantees, schema conversion chains, or compatibility conversion for old Runtime Homes.

## Storage Profile

The current baseline storage profile is `baseline_sqlite_v5`.

Baseline storage uses the canonical SQL sources [`registry.sql`](../../../crates/volicord-store/src/schema/registry.sql) and [`project.sql`](../../../crates/volicord-store/src/schema/project.sql). Runtime Home initialization applies those sources to empty SQLite databases. Baseline storage does not create `schema_migrations`, `schema_version`, `migration_version`, `storage_version`, or equivalent storage-version fields.

A database is usable only when its table shape, columns, indexes, foreign keys, constraints, and stored `storage_profile` match the current baseline. These conditions make storage or runtime unavailable:

- an unknown table that represents an old schema ledger
- a missing required table
- a forbidden storage-version column
- a storage-profile mismatch
- a malformed required record

Store code must not guess record meaning, silently rewrite data, or convert unsupported storage. Existing Runtime Homes with incompatible storage fail clearly and require Runtime Home recreation.

Baseline registry storage includes Runtime Home identity, installation profile records, repository-root-based project registrations, project aliases, Agent Connection records, `connection_projects`, and `guard_installations`. Baseline project-state storage includes Core state projection records, `authority_events`, replay rows, staged artifacts, persistent artifacts, evidence, evidence-capture intents, receipts, exclusive source claims, immutable evidence producers, user-action requests, immutable user-action resolutions, request-bound local channel tokens, runs, blockers, `write_tickets`, host-observation records, and session-watch records.

`baseline_sqlite_v5` replaces the v4 judgment and direct user-observation
families with `user_action_requests`, immutable one-to-one
`user_action_resolutions` carrying closed tagged observation-resolution detail,
and request-bound local channel tokens. The baseline provides no in-place
conversion from `baseline_sqlite_v4`. A v4 Runtime Home is an incompatible
shape and must be recreated; Store must not relabel or guess-convert it.
`project_state.state_version` remains a Core state clock and is not the
storage-profile version.

<a id="canonical-core-utc-clock"></a>
## Canonical Core UTC Clock

The canonical Core UTC clock is the project-scoped, non-decreasing UTC clock
used for temporal-authority decisions. `project_state.updated_at` is the
persisted floor for that clock. Despite its physical column name, this value is
not merely display metadata on the `project_state` row.

A current project-time sample must be no earlier than all of these values:

- the configured live-time candidate, which is SQLite current UTC for
  `SystemClock`
- the persisted `project_state.updated_at` floor
- any later project-time sample already accepted by the current Store handle

The default `SystemClock` uses SQLite current UTC as its live-time source. An
injected or custom Clock may replace that live source for controlled execution
or tests, but it cannot replace the persisted floor or the same-handle accepted
sample. The `CoreService` clock boundary composes every such candidate with
those lower bounds by taking their maximum before exposing canonical project
time. This composition never rewrites a stored row timestamp to current time. A
future-valued row fails closed only where its timestamp owner defines that
value as invalid; the clock does not normalize it or add a new rejection rule
for other owners.

The persisted floor and every timestamp compared with it use the canonical UTC
timestamp form owned by [Storage Records](storage-records.md). A malformed
persisted floor is corrupt owner state. Store must fail closed rather than
repairing, replacing, or rewinding it.

After common preflight, each prepared public Core operation takes exactly one
`operation_now` sample. Planning must reuse that sample for every current-time
decision and public operation timestamp in the operation. This includes expiry
and effective-status checks, derived-expiry calculation, and UserAction
`created_at`, `requested_at`, or `resolved_at` values. Planning must not take
another clock sample that could change the result within the same operation.

Every owner-defined TTL or derived expiry uses checked timestamp addition and
must remain representable in the canonical RFC 3339 UTC form. Arithmetic
overflow or an unrepresentable result is a controlled validation rejection
before commit and has no storage effect. Store revalidates canonical timestamp
columns before writing them; a typed timestamp supplied by Core or an adapter
does not bypass that storage boundary.

A new normal Core commit chooses one `committed_at` under its immediate write
transaction. Its candidate set follows the configured Clock:

- With the production `SystemClock`, `committed_at` is the maximum of
  `operation_now`, SQLite current UTC sampled inside that transaction, the
  persisted project floor, and any later project-time sample already accepted
  by the current Store handle.
- With an injected or custom Clock, `committed_at` is the maximum of
  `operation_now`, that Clock's injected live-time candidate, the persisted
  project floor, and any later same-handle accepted sample. The injected
  candidate replaces the transaction's SQLite live-time candidate; SQLite
  current UTC is not added as a second live candidate.

Neither branch can bypass the persisted or same-handle floor. The transaction
writes that exact same `committed_at` value to:

- `project_state.updated_at`
- every `authority_events.created_at` row in the committed event or event batch
- `tool_invocations.created_at` when the commit creates a replay row
- every Store transaction-metadata timestamp that mutation application itself
  generates for that commit, including applicable `created_at`, `updated_at`,
  `retired_at`, and `promoted_at` values

`committed_at` can be later than `operation_now`; public method timestamps that
the method owner derives from the prepared sample remain `operation_now`.
Multiple state versions may share a timestamp when the selected UTC value is
equal. The clock is non-decreasing, not required to increase strictly for every
commit. UTC values express temporal boundaries and owner-defined times; they
must not be used as a surrogate for authority commit order or latest-record
selection.

Semantic operation times and input- or observation-owned facts are not
automatic Store transaction metadata. Owner-defined `requested_at`,
`resolved_at`, `closed_at`, `recorded_at`, and `consumed_at` values preserve the
single prepared `operation_now` or an owner-verified observation time, as the
method owner specifies. Likewise, `observed_at` and `started_at` preserve when
the source says the observation or activity occurred. A Core commit must not
overwrite these values merely to equal `committed_at`.

`project_state.state_version` and the persisted UTC floor are separate clocks:

- `state_version` orders committed authority-state transitions and supplies the
  public conflict and freshness basis.
- the UTC floor prevents later temporal-authority decisions from observing a
  project time earlier than time the project has already accepted.

Neither clock substitutes for the other. A normal Core authority commit
advances `state_version` and updates the floor atomically. The following
storage-owned effects update the floor to at least their own `created_at`
without incrementing `state_version` or creating an authority event or replay
row:

- issuing a request-bound local User Channel token
- creating an artifact-staging row
- fulfilling an evidence-capture receipt and its staging and source-claim rows

Those floor-only effects must be atomic with the row or row set whose timestamp
they preserve. Exact replay, rejected requests, `dry_run=true` planning,
read-only results, and failed transactions do not update the persisted floor.
A read may observe a later current project time without persisting it.

New project registration initializes `project_state.created_at` and
`project_state.updated_at` from the storage engine's current UTC time.
Re-registering an existing project must validate and preserve its exact
`updated_at` value while updating only owner-allowed registration metadata. It
must not reset the floor to host time or storage time, including when the
persisted value is later than the current live sample.

## Project State Version

`project_state.state_version` is the project-wide Core state clock for committed authority state changes. It is not a schema version, migration version, storage version, or compatibility marker.

It increments only when a complete owner-allowed state-changing transaction commits. It does not increment for rejected requests, dry-run responses, read-only results, startup checks, host verification, schema initialization, storage-profile validation, lock acquisition, status projection, rendered reports, or failed transactions.

Every newly committed authority mutation appends at least one durable `authority_events` row in the same transaction as the current projection updates. A normal committed mutation appends exactly one authority event. If an owner explicitly defines an event batch, all rows in that batch share the single resulting `project_state.state_version` for that committed state transition.

`tasks.state_version` is not a baseline authority field. A non-baseline `tasks.state_version` column is invalid storage shape and must not be used as a conflict, freshness, lock, or write-ticket basis.

Related fields:

- `write_tickets.basis_state_version` stores the resulting `project_state.state_version` after the write-ticket issuance commit. Core uses it as the freshness basis for later write-ticket consumption. Current write-ticket selection within a Task orders this field descending; the DDL makes the ordering key unique per Task, so timestamps and opaque record IDs are not authority-order tie-breakers.
- `evidence_summaries.produced_at_state_version` stores the resulting
  `project_state.state_version` of the commit that most recently inserted or
  updated that summary. Current Evidence Summary selection orders this field
  descending and never uses a timestamp or opaque record ID as a tie-break or
  substitute.
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

The storage unique key is exactly `(project_id, tool_name, idempotency_key)`.
`request_hash` is the conflict discriminator for the Core-owned canonical
request identity. This is ordinarily the public request payload. A token-bearing
local-web `volicord.resolve_user_action` additionally binds a domain-separated
token digest, the expected Agent Connection, and typed canonical completion
metadata before hashing. The raw token and that internal binding object are not
stored in `tool_invocations`; only the resulting request hash and response are
durable. Other invocation context such as `actor_source`, `operation_category`,
or `verification_basis` is not silently absorbed into this hash. The expected
connection in the local-web binding is deliberate method-owned credential
context and does not replace the separate verified replay-context checks.

New replay rows store `actor_source`, `operation_category`, the exact optional
`verification_basis`, and the exact optional canonical Git workspace context
from the verified invocation. The latter two coordinates preserve presence or
absence as well as value. A current replay row is eligible only when all four
stored coordinates exactly match the current verified replay context. Missing
required replay identity is invalid stored state, not a compatibility
projection.

Replay eligibility:

- a stored response must never be returned before the current invocation has a verified invocation context
- Core checks invocation-context compatibility before request-hash compatibility
- incompatible context, including a changed or newly absent/present
  `verification_basis` or Git workspace context, returns
  `INVOCATION_CONTEXT_MISMATCH` and must not expose the stored response
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
`volicord.resolve_user_action`, are not eligible for Agent Connection
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

Pre-commit failures have no storage effect. Transaction failures must leave no partial state-version increment, canonical-floor update, event, replay row, write-ticket change, artifact effect, evidence update, user-action request or resolution effect, close effect, lifecycle effect, or staged-handle consumption.

Examples:

- stale `expected_state_version`
- stale `WriteTicket.basis_state_version`
- validation failure
- malformed request
- corrupt typed owner state
- idempotency request-hash conflict
- invocation-context mismatch
- incompatible existing storage shape

Retry follows the rejected reason: refresh state for stale version conflicts, fix invalid input for validation failures, use the User Channel for pending user actions, use the required write-ticket flow when write compatibility is still needed, or recreate the Runtime Home when storage is incompatible.

## Owner Links

- Record-family overview and storage-owned values: [Storage Records](storage-records.md)
- SQLite DDL, constraints, indexes, and foreign keys: [Storage DDL](storage-ddl.md)
- Method storage effects: [Storage Effects](storage-effects.md)
- Public conflict behavior: [API error precedence](api/error-precedence.md#state-conflict-behavior)
- Public invocation-context mismatch code: [API error codes](api/error-codes.md#errorcode-invocation-context-mismatch)
- Runtime Home separation: [Runtime Boundaries](runtime-boundaries.md)
