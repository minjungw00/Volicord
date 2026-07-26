# Canonical Core UTC clock

## Context

Volicord makes expiry, freshness, capture-time, and lifecycle decisions across
Core planning, atomic Core commits, storage-owned staging and receipt writers,
local User Channel tokens, bootstrap, and host observations. Using process wall
time independently at each call site would let one project observe time moving
backward, and using `state_version` as time would conflate authority-state order
with UTC deadlines.

The implementation also needs to distinguish semantic times such as when an
operation or observation occurred from Store transaction metadata that records
when rows were committed.

## Decision

Use one project-scoped canonical Core UTC clock. Store persists its
non-decreasing floor in `project_state.updated_at`; no additional schema field
or clock table is introduced. A current sample is selected from the configured
live-time candidate, the persisted floor, and a later project sample already
accepted by the Store handle. `SystemClock` uses SQLite current UTC as that
candidate.

A custom or injected Clock may replace the live candidate, but the Core service
boundary still composes it with the persisted floor and same-handle sample by
taking the maximum. It does not rewrite stored owner timestamps to make them
current. A future-valued row fails closed only where its owner defines that
value as invalid. Every TTL derivation uses checked addition and requires a
representable canonical RFC 3339 UTC result before commit.

A public operation that reaches method planning samples `operation_now`
exactly once after common preflight. The planner reuses that value for every
current-time check and owner-defined semantic operation timestamp. The commit
candidate branch follows the configured Clock. With the production
`SystemClock`, transaction `committed_at` is the maximum of `operation_now`,
SQLite current UTC sampled inside the transaction, the persisted floor, and a
later same-handle accepted sample. With an injected or custom Clock, its
injected live-time candidate replaces the transaction's SQLite candidate; the
maximum still includes `operation_now`, the persisted floor, and the
same-handle sample. The custom branch does not add SQLite current UTC as another
live candidate.

The normal Core commit uses exact `committed_at` for the project floor, every
event in the batch, the optional replay row, and Store-generated transaction
metadata such as applicable `created_at`, `updated_at`, `retired_at`, and
`promoted_at`. It does not replace semantic `requested_at`, `resolved_at`,
`closed_at`, `recorded_at`, or `consumed_at` values, or verified observation
facts such as `occurred_at`, `observed_at`, and `started_at`.

Storage-owned artifact staging, evidence-capture receipt fulfillment, and local
User Channel token issuance update the floor atomically to at least their own
creation time without becoming Core authority commits. Exact replay,
rejection, dry run, and read-only observation do not persist a later floor.

`state_version` remains the authority-state and conflict clock. The canonical
UTC floor remains the temporal-authority lower bound. Neither substitutes for
the other, and the UTC floor is non-decreasing rather than required to advance
strictly on every state transition.

Bootstrap initializes the floor for a new project. Re-registration validates
and preserves an existing floor and fails closed on malformed owner state; it
does not reset future-valued valid time.

## Consequences

- One operation cannot cross an expiry boundary merely because separate
  planning checks sampled different times.
- A Core commit has one auditable transaction timestamp while retaining the
  semantic distinction between operation, observation, and commit time.
- Storage-only temporal writers remain distinguishable from authority-state
  transitions and do not manufacture events or state versions.
- Host clock skew and delayed observations cannot rewind or advance current
  Core authority boundaries.
- A corrupt persisted floor is corrupt owner state, not a value to repair
  heuristically.
- Test clocks remain useful without gaining authority to bypass stored project
  time, and TTL overflow is a controlled no-effect rejection.

## Rejected alternatives

- Process wall time at every call site permits backward movement and
  within-operation drift.
- `state_version` cannot represent UTC deadlines or observation time and would
  hide storage-only temporal effects.
- A new clock table or schema version field duplicates the existing project
  header without improving ownership.
- Rewriting all timestamp-shaped fields to commit time destroys semantic source
  and operation facts.
- Advancing the persisted floor for replay, rejection, dry run, or reads turns
  no-effect paths into hidden writes.

## Relevant implementation

- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs):
  prepared operation sampling and commit-floor propagation.
- [`crates/volicord-store/src/core_pipeline/`](../../../../crates/volicord-store/src/core_pipeline/)
  and [`core_pipeline/commit.rs`](../../../../crates/volicord-store/src/core_pipeline/commit.rs):
  project-time sampling and canonical transaction-time selection.
- The aggregate mutation modules under
  [`crates/volicord-store/src/core_pipeline/`](../../../../crates/volicord-store/src/core_pipeline/)
  and [`workflow_records.rs`](../../../../crates/volicord-store/src/workflow_records.rs):
  transaction-metadata application with the coordinator-selected timestamp.
- [`crates/volicord-store/src/artifacts.rs`](../../../../crates/volicord-store/src/artifacts.rs)
  and [`evidence_capture.rs`](../../../../crates/volicord-store/src/evidence_capture.rs):
  storage-owned floor writers.
- [`crates/volicord-store/src/bootstrap.rs`](../../../../crates/volicord-store/src/bootstrap.rs):
  floor initialization, preservation, and validation.

## Reference owners

Exact contracts remain in [Storage Versioning](../../reference/storage-versioning.md),
[Storage Records](../../reference/storage-records.md),
[Storage Effects](../../reference/storage-effects.md),
[Core Model](../../reference/core-model.md), and the applicable public method
and schema owners. See also [Planning before atomic mutation commit](plan-and-atomic-commit.md)
and [Unified user-action request and resolution](unified-user-action-request-resolution.md).
