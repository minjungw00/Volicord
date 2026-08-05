# Storage Versioning

Shaping progression is part of the single exact storage manifest. A Runtime Home whose manifest differs is incompatible and remains byte-preserved; no schema-version branch, row conversion, dual read, or dual write applies.

This document owns the current Volicord SQLite storage contract: manifest
identity, exact database-open validation, canonical schema metadata,
project-state clocks, atomic mutation boundaries, idempotency, and exact
replay.

It does not own physical table or column definitions, public API behavior,
record-family meaning, method-specific storage effects, artifact lifecycle,
Runtime Home placement, or security guarantees. Exact SQLite DDL remains with
[Storage DDL](storage-ddl.md).

<a id="surface-stability"></a>
## Surface Stability

For the stability vocabulary, see
[Documentation Policy](../maintain/documentation-policy.md#surface-stability-labels).

| Surface | Stability | Contract |
|---|---|---|
| `StorageManifest`, canonical SQL identity, exact-open validation, and `GeneratedSchemaMetadata` | `stable` | These identify the only accepted SQLite format. |
| `project_state.state_version`, the canonical Core UTC clock, atomic authority commits, and exact replay | `stable` | These remain authority, freshness, and idempotency contracts within the accepted format. |
| Manifest placement, generated Rust modules, metadata extraction helpers, and query implementation | `internal` | Implementations may change while preserving the exact generated facts and open behavior. |
| Bounded storage-open and corruption diagnostics | `diagnostic` | Diagnostic text is not a compatibility identity and must not expose raw owner data, SQL text, secrets, or sensitive absolute paths. |

## One Canonical SQLite Contract

Volicord supports exactly one SQLite storage format. Its sources of truth are
the current canonical SQL files:

- [`registry.sql`](../../../crates/volicord-store/src/schema/registry.sql)
- [`project.sql`](../../../crates/volicord-store/src/schema/project.sql)

New storage is created only from these sources. `project_state.state_version`
is a Core authority-state clock, not a storage-format identity. Ordinary open
accepts only the exact current manifest and physical schema and does not alter
an existing database. Development data can be created in a fresh location from
the canonical SQL and current manifest. Persisted authority data is never
silently discarded or recreated by ordinary open.

The separate non-authority `diagnostics.sqlite` database follows the same
single-current-contract rule through its own semantic
`volicord.sqlite.diagnostics` manifest and a canonical schema digest derived
from its SQL inventory. At an absent final path, the complete schema and
manifest are initialized and validated in one opaque, invocation-owned
same-directory staging database. After every SQLite handle is closed, no live
sidecar is required, permissions are hardened, and the file is synchronized,
one atomic no-replace publication makes it visible as `diagnostics.sqlite`.
Concurrent shared writers converge on the fully validated winner, clean only
their own staging files, and write their sessions to the final database. An
existing final database is validated exactly and is never initialized or
repaired. The contract accepts no numeric `PRAGMA user_version` dispatch,
inferred format, or partial schema, and its manifest is not an authority
`StorageManifest`.

## `StorageManifest`

The supported storage contract is identified by this exact manifest:

```schema
StorageManifest:
  contract_id: string
  canonical_ddl_digest: string
  integrity_constraints_digest: string
  enabled_capabilities: string[]
```

The current constants are exact:

```schema
contract_id: volicord.sqlite.canonical
enabled_capabilities:
  - artifact_storage
  - authority_event_chain
  - exact_operation_result
  - invocation_repository_observation
  - managed_codex_connection
  - operational_mcp_sessions
  - project_continuity
  - shaping_authority_reauthorization
  - shaping_checkpoint_lineage
  - shaping_decision_applications
  - shaping_decision_recovery
  - shaping_progression
  - user_action_cli_resolution
```

`enabled_capabilities` is this complete ascending UTF-8 byte order, not a set
whose serialized order may vary. An unknown or missing `contract_id`, an
unknown, missing, duplicate, reordered, or strict-subset capability list, or a
noncurrent value is invalid. Store supplies no default, alias, conversion, or
capability inference.

`invocation_repository_observation` identifies the single current storage
shape for exact invocation-scoped repository observations, observation-bound
expected writes, and unmatched-delta Unrecorded Changes. It is a semantic
capability identifier, not a numeric behavior switch or an upgrade instruction.

`shaping_progression` identifies the single current checkpoint/gap/UserAction
link aggregate and explicit work-phase transition storage contract. It is a
semantic capability identifier, not a migration switch, alternate decoder, or
numeric compatibility branch.

`shaping_checkpoint_lineage` identifies exact predecessor identity,
same-Task ownership, immutable lineage, matching predecessor supersession and
successor creation timestamps, and live-linked-UserAction detachment
protection. It is part of exact manifest matching.

`shaping_decision_applications` identifies deterministic first-class
application records, closed authority invalidation, and exact
checkpoint-application carry-forward lineage. It is part of exact manifest
matching.

`shaping_authority_reauthorization` identifies exact stale-application
retirement or reissue, a fresh unresolved successor request for reissue, and
immutable reauthorization lineage. It is part of exact manifest matching.

`shaping_decision_recovery` identifies outcome-specific shaping gap
dispositions, accepted-only application, exact rejected/deferred/expired
request retirement, and atomic successor request identity. It is part of exact
manifest matching.

Field meanings:

| Field | Contract |
|---|---|
| `contract_id` | The semantic identity of the SQLite storage contract. It is compared exactly and is not a numeric revision. |
| `canonical_ddl_digest` | The digest of the deterministic canonical encoding of the complete generated DDL metadata. |
| `integrity_constraints_digest` | The independent digest of the deterministic canonical encoding of all generated integrity constraints. |
| `enabled_capabilities` | The complete, sorted, duplicate-free capability set enabled by the format. Missing capabilities are not inferred. |

Manifest identity is exact, capability absence is meaningful, and format
selection does not use numeric comparison, field-presence inference, decoder
probing, or fallback. The physical manifest representation and its SQLite
placement belong to [Storage DDL](storage-ddl.md).

Only the manifest generated from the current canonical SQL is supported. A
producer emits one deterministic canonical manifest encoding. Map or set
iteration order, host path spelling, SQLite row order, and display formatting
must not affect either digest.

## Exact Database-Open Contract

Store accepts a database only after all of these checks succeed:

1. Read and strictly decode its complete `StorageManifest`.
2. Compare `contract_id`, both digests, and the complete capability set with
   the current built-in manifest.
3. Inspect the actual SQLite objects and constraints.
4. Derive the actual schema inventory using the same canonical metadata rules
   used to build the manifest.
5. Require an exact match among the persisted manifest, generated metadata,
   canonical SQL, and actual database.
6. Enable and verify foreign-key enforcement before exposing a Store handle.

The comparison rejects missing and unexpected tables, columns, indexes, and
constraints. It also rejects any other SQLite object or schema fact that the
canonical SQL does not authorize. Validation completes before authority or
policy records are read and before any mutation is possible.

Failure classification follows [Failure Model](failure-model.md):

- A missing, unknown, previous, or otherwise non-current storage contract is
  `Corrupt` (`corrupt`), as is a database whose manifest encoding, schema
  objects, constraints, digests, or typed owner state violates the current
  contract.
- An I/O, locking, or environmental failure that prevents the checks without
  establishing corruption is `Unavailable` (`unavailable`).

These failures are fail-closed. Store does not try another manifest, decoder,
profile, or SQL inventory; fill missing fields; ignore extra objects; or open a
partially validated database. Repeating open against unchanged bytes produces
the same classification.

After exact current-manifest validation succeeds, read-only inspection may
retain that decoded `StorageManifest` as a typed diagnostic value. Doctor JSON
serializes the value as a structured `storage_profile` object, and verbose
human output renders named fields plus the enabled-capability collection. No
renderer reparses the persisted carrier text. Failed strict decoding or exact
comparison produces the existing corrupt or incompatible inspection state and
never a raw-string fallback.

Fresh Runtime Home initialization is a distinct operation allowed only when
the final path is absent. It creates a same-parent staging directory, applies
the canonical SQL there, generates one opaque UUID-backed publication ID, and
records that provenance with the current manifest, Runtime Home singleton, and
installation metadata. It enables foreign keys and validates the complete
manifest and physical schema before publication. After staging
synchronization, a no-replace atomic rename publishes the directory. The
successful publisher receives an invocation-specific guard immediately after
rename and retains it through parent synchronization, read-back, and manifest
confirmation. An `AlreadyExists` caller removes only its staging and observes
the exact current winner without removal authority.

The setup service serializes inspection, planning, publication, Store
mutation, cleanup, reporting, and rollback with an external OS-backed lease
for the canonical Runtime Home. That lease is not a Registry record,
`StorageManifest` capability, schema identity, or storage lock. A supported
setup cannot use an observed no-replace result to continue Store mutation. An
unexpected `AlreadyExists` while the lease is held is read-only inspected and
reported as an external concurrent modification that requires a fresh plan.

If confirmation fails after publication, the primary confirmation error and
the guard-backed rollback attempt form one typed failure. It records whether
the final path was observed present, absent, or uncertain and keeps recursive
removal effect separate from parent-directory durability. Complete removal is
terminal even when parent synchronization fails; an incomplete or unknown
effect is also terminal and cannot be retried against a later path occupant.
These lifecycle facts do not select another storage profile or schema.

An existing Runtime Home follows the exact-open checks above through a
read-only connection before any setup mutation. The result is `Ready`,
`Incompatible`, or `Corrupt`; a mismatch includes bounded manifest-digest and
physical relation facts. Store does not alter an incompatible or corrupt home.

## Canonical SQL And Generated Metadata

Canonical SQL is the single source of truth. A deterministic build-time or
test-time extraction produces exactly this metadata:

```schema
GeneratedSchemaMetadata:
  tables: GeneratedTable[]
  columns: GeneratedColumn[]
  indexes: GeneratedIndex[]
  constraints: GeneratedConstraint[]
  canonical_ddl_digest: string
  integrity_constraints_digest: string
```

The extraction uses a fixed source order and deterministic ordering within
every collection. The digest inputs exclude the digest fields themselves.
Both digests are computed from the applicable canonical inventory encoding
consumed by validation; they are not copied from a separate hand-maintained
list.

The same generated artifact is shared by:

- runtime exact-schema validation
- executable DDL contract tests
- `StorageManifest` construction
- the maintained documentation schema inventory
- Store schema projections needed by queries and row decoders
- storage fixtures

No consumer keeps a separate authoritative table, column, index, or constraint
inventory. A fixture or documentation table can project generated facts, but
cannot redefine them. Exact SQL text and physical constraint definitions remain
with [Storage DDL](storage-ddl.md).

## Fail-Closed Connections And Atomic Mutations

Every accepted SQLite connection enables:

```sql
PRAGMA foreign_keys = ON;
```

An authority mutation uses `BEGIN IMMEDIATE` or an equivalent serialized write
boundary before it reads freshness, ticket compatibility, replay identity, or
the persisted canonical-UTC floor. Within that one transaction, Store
revalidates every fact on which the planned mutation depends.

A successful authority mutation atomically commits its current projections,
immutable authority event or owner-defined event batch, state-version advance,
canonical UTC floor update, and optional replay row. Any associated write-ticket,
artifact, evidence, user-action, lifecycle, or close-state effect owned by the
method commits in that same boundary. A failed transaction leaves none of
those effects partially visible.

Typed persisted owner data is decoded into its complete current type and
validated before use. Malformed JSON, missing required fields, unknown closed
variants, forbidden extra fields, and violated cross-field invariants are
`Corrupt`; they do not become empty values, defaults, absent state, or a
different storage contract.

This strict boundary includes every persisted `BaselineRef`. Scalar owner
columns are checked through one Store decoder, while baseline-bearing owner
JSON is decoded through the same semantic type. Empty values, surrounding
whitespace, and the string `"null"` are `Corrupt`; SQL `NULL` or JSON `null`
means absence only at an owner-declared nullable position. Store does not trim
or recover these values, and Core, MCP, and CLI surface the resulting
persisted-data corruption without mutation effects.

<a id="canonical-core-utc-clock"></a>
## Canonical Core UTC Clock

The canonical Core UTC clock is project-scoped and non-decreasing.
`project_state.updated_at` is its persisted floor, not display-only metadata or
a second public state version.

After common preflight, a prepared public Core operation takes one
`operation_now` sample and reuses it for all current-time decisions and public
operation timestamps. Checked timestamp arithmetic must remain representable
in the canonical RFC 3339 UTC form; overflow or an unrepresentable result is a
no-effect validation rejection.

A normal Core commit selects one `committed_at` inside its immediate write
transaction. With the production clock, it is the maximum of `operation_now`,
SQLite current UTC sampled in the transaction, the persisted floor, and any
later sample already accepted by the current Store handle. With an injected
clock, the injected live candidate replaces SQLite current UTC but cannot
replace the persisted or same-handle floor.

The transaction writes the same `committed_at` to
`project_state.updated_at`, every authority event in the commit, an optional
replay row, and transaction metadata generated for that commit. Semantic
operation or observation timestamps retain their owner-defined source and are
not overwritten merely to equal `committed_at`.

The UTC floor and `project_state.state_version` are separate clocks. An
owner-defined floor-only storage effect must update the floor atomically with
the rows whose timestamps it preserves. Exact replay, rejected requests,
`dry_run=true` planning, read-only results, database-open validation, and failed
transactions do not update the floor.

## Project State Version And Write-Ticket Ordering

`project_state.state_version` orders committed Core authority-state changes and
provides the public conflict and freshness basis. It advances exactly once only
when a complete owner-allowed authority mutation commits. It does not advance
for rejected requests, dry-run responses, exact replay, read-only results,
initialization, database-open validation, lock acquisition, or failed
transactions.

Each new committed authority mutation appends at least one immutable
`authority_events` row in the same transaction as its projections. An
owner-defined event batch shares the single resulting state version. UTC
timestamps do not replace this ordering.

`write_tickets.basis_state_version` records audit order only. It is not a
ticket-validity coordinate, and unrelated state-version advances do not
invalidate a ticket. Ticket compatibility and consumption are validated by
Core before planning and by Store again inside the committing transaction.
Rejected, dry-run, and replay-only branches issue, reuse, invalidate, or consume
no ticket.

## Idempotency And Exact Replay

`tool_invocations` stores replay data only for committed `dry_run=false` Core
`MethodResult` responses whose method owner defines a replay row. Its unique
key is exactly `(project_id, tool_name, idempotency_key)`, and `request_hash`
distinguishes the Core-owned canonical request identity.

Replay eligibility requires the current verified invocation context to match
the complete stored replay context exactly: `actor_source`,
`operation_category`, the exact optional `verification_basis`, and the exact
optional canonical Git workspace context. Optional coordinates preserve both
presence or absence and value. Invocation context is not silently absorbed
into `request_hash`; Core checks context compatibility before request-hash
compatibility:

- compatible context, the same key, and the same hash returns the stored
  original committed response exactly
- compatible context and the same key with a different hash returns
  `STATE_VERSION_CONFLICT`
- incompatible context returns `INVOCATION_CONTEXT_MISMATCH` without exposing
  the stored response

Before any replay path returns bytes, the immutable stored JSON must have no
duplicate decoded member at any depth and must strict-decode directly as the
current closed `MethodResult` selected by its stored method name. Its response
kind, effect kind, dry-run flag, and committed state version must match the
replay row. A malformed, non-current, wrong-method, or coordinate-mismatched
row is `Corrupt` and unavailable for replay. Store does not rewrite, redact,
or return replay bytes from that row.

Replay returns the stored response body. It does not recompute a field, append
an event, create another replay row, change state, promote or link an artifact,
or issue, invalidate, reuse, or consume a write ticket.

<a id="exact-operation-result-retrieval"></a>
### Exact operation-result retrieval

Every eligible `operation_category=agent_workflow` Core commit and exact replay
exposes an `OperationResultRef` for the immutable stored response. The retrieval
method reads contiguous UTF-8-safe pages; concatenating them in cursor order
must reproduce the stored response byte-for-byte. Retrieval never recomputes,
normalizes, reserializes, or reclassifies a field.

Before returning the first page, Store loads the exact bytes and computes their
length and SHA-256, and Core compares those facts with the reference after
verifying current actor and project access. The same strict committed-result
eligibility check used by replay applies. Any integrity, shape, method, access,
or coordinate failure returns no partial page. Retrieval is read-only and
creates no replay row, event, state transition, or state-version advance. Rows
outside `operation_category=agent_workflow` are not eligible for this Agent
Connection retrieval path.

`volicord.stage_artifact` remains outside the Core replay transaction. It
creates no replay row or `OperationResultRef`, and its complete serialized
result must satisfy the supported prospective size bound before any staging
effect occurs.

## Structured Store Diagnostics

Store diagnostics classify `rusqlite` failures from SQLite primary and extended
result codes when they are available. They never match SQLite message text.
The closed current codes are:

| Code | Source condition |
|---|---|
| `store.sqlite.readonly` | Primary code `SQLITE_READONLY`. |
| `store.sqlite.busy` | Primary code `SQLITE_BUSY`. |
| `store.sqlite.locked` | Primary code `SQLITE_LOCKED`. |
| `store.schema.mismatch` | Exact manifest or physical schema mismatch, including `SQLITE_SCHEMA`. |
| `store.integrity.corruption_failure` | `SQLITE_CORRUPT`, `SQLITE_NOTADB`, or failed typed integrity validation. |
| `store.record.missing` | A typed required record or query row is absent. |
| `store.transaction.failed` | SQLite abort or interruption of a transaction. |
| `store.serialization.failed` | A typed stored value cannot be encoded or decoded. |
| `store.constraint.violation` | `SQLITE_CONSTRAINT`; the safe `constraint_kind` fact is derived from the extended code where known. |

The finding may include numeric `sqlite_primary_code` and
`sqlite_extended_code`, database kind, entity, field, or an I/O error kind. It
does not include arbitrary SQLite messages, SQL text, row bodies, environment
values, secrets, or filesystem contents. An unmapped internal Store failure
uses `internal.unexpected_failure` rather than guessing from prose.

Recommended actions are derived from the typed code. Busy and locked findings
use `action.store.free_locked_database`; read-only, schema, corruption,
missing-record, serialization, transaction, and constraint findings use their
focused repair actions. None of these deterministic failures recommends a
generic host restart.

## Failure, Retry, And Development Data

Pre-commit and transaction failures have no partial storage effect. A retry
addresses the reported cause; it does not select a different storage contract
or mutate the failed database during open.

A storage contract outside the accepted identity requires a fresh Runtime Home
or project store created from the current canonical SQL. Corrupt persisted
authority data requires an explicit owner-defined recovery decision; ordinary
reads and writes do not repair it. Development-only databases may be deleted
and recreated from the current sources.

The current Registry contract stores integration verification as one immutable
attempt per complete semantic coordinate with a semantic observation policy
and typed repair/retry fields. Exact-open validation requires that complete
current relation and constraint inventory; Store never synthesizes acquisition
evidence or attempt state while opening a Runtime Home.

## Owner Links

- Cross-surface failure categories and no-default rules:
  [Failure Model](failure-model.md)
- Exact SQLite tables, columns, indexes, foreign keys, and constraints:
  [Storage DDL](storage-ddl.md)
- Persisted record-family meanings: [Storage Records](storage-records.md)
- Method-specific effects and no-effect branches:
  [Storage Effects](storage-effects.md)
- Public state conflict and invocation-context errors:
  [API error precedence](api/error-precedence.md#state-conflict-behavior) and
  [API error codes](api/error-codes.md#errorcode-invocation-context-mismatch)
- Runtime Home placement and separation:
  [Runtime Boundaries](runtime-boundaries.md)
