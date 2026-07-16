# Storage Versioning

This document owns baseline storage-versioning rules for current Volicord SQLite storage, including the single supported offline v6-to-v7 copy conversion. It does not define public API behavior, Core authority meaning, security guarantees, administrative command syntax, policy-file discovery, or host integration.

## Storage Profile

The current baseline storage profile is `baseline_sqlite_v7`.

Baseline storage uses the canonical SQL sources [`registry.sql`](../../../crates/volicord-store/src/schema/registry.sql) and [`project.sql`](../../../crates/volicord-store/src/schema/project.sql). Runtime Home initialization applies those sources to empty SQLite databases. Baseline storage does not create `schema_migrations`, `schema_version`, `migration_version`, `storage_version`, or equivalent storage-version fields.

A database is usable only when its table shape, columns, indexes, foreign keys, constraints, and stored `storage_profile` match the current baseline. These conditions make storage or runtime unavailable:

- an unknown table that represents an old schema ledger
- a missing required table
- a forbidden storage-version column
- a storage-profile mismatch
- a malformed required record

Normal Store opening must not guess record meaning, silently rewrite data, or convert unsupported storage. It rejects v6 as incompatible with v7. The explicitly invoked offline copy conversion below is the only exception and never opens the v6 source for mutation.

Baseline registry storage includes Runtime Home identity, installation profile records, repository-root-based project registrations, project aliases, Agent Connection records, `connection_projects`, immutable host-capability verification history, current host-capability pointers, and `guard_installations`. Baseline project-state storage includes Core state projection records, `authority_events`, replay rows, staged artifacts, persistent artifacts, evidence, evidence-capture intents, receipts, exclusive source claims, immutable evidence producers, user-action requests, immutable user-action resolutions, request-bound local channel tokens, runs, blockers, `write_tickets`, host-observation records, and session-watch records.

`baseline_sqlite_v7` adds requested/effective control fields, the authoritative
`volicord-policy-v2` database copy and fingerprint, reusable state-bound write
tickets with stable invalidation reasons and optional idle timeout, unrecorded-
change confidence, and session-end authority receipts with
`completion_claim_allowed`. Privacy-bounded workflow metrics remain in the
separate non-authority diagnostics schema and are not project storage-profile
authority.

`baseline_sqlite_v6` adds `host_capability_verifications` and
`host_capability_state` to the Registry so credential-delivery eligibility can
depend on immutable, expiring, exact-profile live-host evidence instead of a
client declaration or mutable configuration-verification JSON. The baseline
provides no in-place conversion from `baseline_sqlite_v5`. A v5 Runtime Home is
an incompatible shape and must be recreated; Store must not relabel it, infer a
passing verification from existing connection state, or synthesize history.
The v6/0.9.0 host-capability shape includes its exact UTF-8 byte constraints:
general free-text history and current-pointer coordinates are 1 through 1,024
bytes, and managed MCP `client_name` and `client_version` are 1 through 256
bytes. Completing those constraints within the v6 batch does not create a v7
transition. A database labeled v6 but lacking the canonical constraints is
incompatible and must be recreated; Store must not trim, truncate, repair, or
legacy-decode its values.

The earlier `baseline_sqlite_v5` profile replaced the v4 judgment and direct user-observation
families with `user_action_requests`, immutable one-to-one
`user_action_resolutions` carrying closed tagged observation-resolution detail,
and request-bound local channel tokens. The baseline provides no in-place
conversion from `baseline_sqlite_v4`. A v4 Runtime Home is an incompatible
shape and must be recreated; Store must not relabel or guess-convert it.
`project_state.state_version` remains a Core state clock and is not the
storage-profile version.

The pre-major v5 contract stored a registered-connection capture's
closed source selector and Core-derived canonical selector digest in the intent.
Concrete event/watcher-observation identity, observation time, and raw-event or
snapshot/selection digest are receipt-owned facts. This correction changes no
canonical SQL table, column, index, foreign key, or constraint and is completed
inside the `baseline_sqlite_v5` / `0.8.0` batch. It therefore did not
create another storage-profile or package-version transition. Store does not
decode the removed caller-supplied future-observation-digest capture shape as a
legacy alias or fallback; a malformed required record fails closed.

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

- `write_tickets.basis_state_version` stores audit ordering for issue or reuse. It is not unique and is never a validity coordinate. Ticket validity uses the explicit Task, Change Unit, scope revision, baseline, workspace, approval basis, consumption/revocation state, and optional idle timeout.
- `evidence_summaries.produced_at_state_version` stores the resulting
  `project_state.state_version` of the commit that most recently inserted or
  updated that summary. Current Evidence Summary selection orders this field
  descending and never uses a timestamp or opaque record ID as a tie-break or
  substitute.
- `tool_invocations.basis_state_version` stores the project-wide state version observed before the committed mutation.
- `authority_events.state_version` stores the resulting project-wide version after the committed authority event or event batch.

## Write Tickets

A write ticket is reusable-until-consumed Volicord authority for compatible authorized product-file write intent. It is not OS permission, OS sandboxing, a filesystem ACL, network policy, secret isolation, global filesystem interception, or proof that a write occurred.

Write-ticket issuance and compatibility consumption follow normal state-version rules:

- issuance can commit only through an owner-defined method branch
- prepare-write may reuse an active unconsumed ticket when every validity coordinate matches, the existing allowed prefixes cover the requested prefixes, denied prefixes remain effective, and sensitive authority is equal or stronger
- consumption can commit only for an actual product-file write when the row is active, compatible, unconsumed, not revoked or invalidated, and within an optional configured idle boundary
- unrelated state-version increments do not invalidate tickets; explicit invalidation reasons are `scope_revision_changed`, `change_unit_changed`, `baseline_changed`, `workspace_changed`, `approval_basis_changed`, `idle_timeout`, `task_closed`, and `explicit_revoke`
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

Every stored public-method result has an additional whole-response eligibility
check before preflight replay, replay discovered inside the commit transaction,
or MCP resume. The immutable raw JSON must contain no duplicate decoded object
member at any nesting level, including escape-equivalent names, and must
strict-decode directly as the current concrete,
closed `MethodResult` type selected by its stored `method_name` before any
generic JSON-tree normalization. Its base must use `response_kind=result`,
`effect_kind=core_committed`, `dry_run=false`, and the exact
`state_version=tool_invocations.committed_state_version`. Methods that never
create a replay row are categorically ineligible. This check includes every
nested `StateSummary`. A request result must contain exactly one closed three-field
`AgentSafeUserActionRequestSummary`; a close result must contain the current
`pending_user_action_summaries` field and must not contain the legacy
`pending_user_action_inbox_items` field. A missing or malformed required field,
duplicate member, legacy full-form field, unknown extra field, generic rejected
or dry-run branch, wrong method shape, commit-coordinate mismatch, or mixed old
and new shape makes the row unavailable. Every replay path then fails closed at
the typed owner-state boundary as `MCP_UNAVAILABLE`; no stored bytes are
returned. Core never rewrites, redacts, or upgrades an existing replay row.

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
Core also applies the corresponding raw whole-response committed-result
eligibility check after access-context validation and before the first page for
every stored method. An ineligible duplicate-member, non-result, legacy,
wrong-method, commit-coordinate-mismatched, or mixed-shape row returns
`OPERATION_RESULT_UNAVAILABLE`, and no partial page is returned.
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

## Offline v6-to-v7 copy conversion

Normal v7 open rejects v6. The offline converter opens the v6 Runtime Home and
every source database read-only, validates the complete v6 shape and typed
owner state, creates a separate empty v7 destination from canonical DDL, and
copies transformed records in transactions. It never relabels or alters the
source, never updates tables in place, and never activates the destination as
part of conversion.

The transform preserves project, Task, Change Unit, Run, judgment, evidence,
artifact, blocker, event, and replay identifiers; user authority; residual-risk
decisions; canonical evidence and judgment hashes; and durable relationships.
Legacy `advisor` maps to `observe`; `direct` and `work` map conservatively to
`tracked`. Existing acceptance outcomes remain preserved. The initial v2 policy
copy uses a conservative tracked default. Observation-derived confidence is
transformed in two distinct domains: a copied `unrecorded_changes` row uses
`UnrecordedChangeConfidence::Confirmed` only when v6 facts deterministically
establish the product change and otherwise uses `Suspected`; a copied legacy
Detective assessment in `guard_events.result_json` uses
`ObservationConfidence::Confirmed` or `Structured` only when its v6 source
facts prove that level and otherwise is annotated `Heuristic`. Neither domain
borrows the other's value set. Every active v6 write ticket is copied as revoked with
`invalidation_reason=explicit_revoke`; consumed ticket/Run links remain intact.

Before reporting success, the converter verifies foreign keys, table and
eligible-row counts, identifier preservation, canonical JSON, policy and
record fingerprints, evidence/judgment hashes, ticket transformations, and
source immutability, and emits a bounded conversion report. Any failure leaves
the source untouched and the destination unaccepted; partial output is never a
successful v7 store. Activation is a separate administrative operation.

## Failure And Retry

Pre-commit failures have no storage effect. Transaction failures must leave no partial state-version increment, canonical-floor update, event, replay row, write-ticket change, artifact effect, evidence update, user-action request or resolution effect, close effect, lifecycle effect, or staged-handle consumption.

Examples:

- stale `expected_state_version`
- invalid or state-bound incompatible write ticket on a consuming attempt
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
