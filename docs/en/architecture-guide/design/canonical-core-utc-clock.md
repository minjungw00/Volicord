# Canonical Core UTC Clock Design

## Purpose

This design explains how Core and Store use one project-scoped,
non-decreasing UTC time boundary for prepared operations and durable temporal
coordination while keeping authority-state ordering separate.

## Design

`CoreService` samples `operation_now` once after common preflight. The Store
handle combines the configured live-time candidate with the persisted project
floor and any later accepted same-handle sample. A normal Core commit selects
one `committed_at` inside the serialized transaction and uses it for the
project floor, authority events, replay metadata, and Store-generated mutation
metadata.

The production `SystemClock` uses SQLite UTC as its live candidate. Injected
clocks replace that candidate for deterministic tests but do not bypass the
persisted floor. Storage-owned artifact, receipt, and User Channel writers
advance the floor through their own atomic paths without becoming Core
authority commits.

## Invariants

- One prepared operation reuses one `operation_now` value.
- The project time floor never moves backward.
- `state_version` orders authority-state transitions; the UTC floor orders
  temporal authority. Neither substitutes for the other.
- Semantic source and observation timestamps remain distinct from transaction
  metadata.
- Deadline derivation uses checked arithmetic and a representable canonical
  UTC timestamp.
- Read-only, rejected, dry-run, and exact replay paths do not introduce a
  hidden time-floor write.

## Responsibility boundaries

Core owns prepared-operation sampling and passes the time floor into planning
and commit orchestration. Store owns persisted-floor validation, transaction
time selection, and atomic floor updates. Method and policy modules consume the
prepared time; adapters and observed host timestamps do not replace it.

## Execution flow

1. Common preflight opens and validates the current project Store.
2. `CoreService` samples the canonical operation time once.
3. Method planning reuses that value for current-time checks and owned
   semantic operation timestamps.
4. The commit coordinator selects `committed_at` under the immediate
   transaction.
5. Grouped Store mutations, event and replay rows, and the persisted floor use
   the coordinated transaction time where their owners require it.

## Failure behavior

A malformed persisted floor, an unrepresentable timestamp, or deadline
overflow fails without a partial mutation. Future-valued owner data fails only
through the applicable focused owner rule; Store does not heuristically repair
it or reset a valid floor during project registration.

## Scope exclusions

This design does not redefine public timestamp fields, expiry rules, storage
effects, or schema meaning. It does not make host time or `state_version` a UTC
clock and does not require the floor to advance on every read or state change.

## Implementation routes

- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs):
  `Clock`, `SystemClock`, prepared-operation sampling, and commit-floor
  propagation.
- [`crates/volicord-store/src/core_pipeline/clock.rs`](../../../../crates/volicord-store/src/core_pipeline/clock.rs)
  and [`commit.rs`](../../../../crates/volicord-store/src/core_pipeline/commit.rs):
  project time sampling and transaction-time selection.
- [`crates/volicord-store/src/artifacts.rs`](../../../../crates/volicord-store/src/artifacts.rs),
  [`evidence_capture.rs`](../../../../crates/volicord-store/src/evidence_capture.rs), and
  [`bootstrap.rs`](../../../../crates/volicord-store/src/bootstrap.rs):
  storage-owned time writers and floor initialization.

## Reference owners

Exact temporal contracts remain in
[Storage Versioning](../../reference/storage-versioning.md),
[Storage Records](../../reference/storage-records.md),
[Storage Effects](../../reference/storage-effects.md),
[Core Model](../../reference/core-model.md), and the applicable public method
and schema owners.
