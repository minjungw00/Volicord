# Repository Observation Boundary Design

## Purpose

This design explains the current implementation boundary for invocation-scoped
Product Repository snapshots, deterministic deltas, Guard aggregation,
expected-write matching, and Unrecorded Change creation.

## Design

`volicord-platform-fs` captures bounded stable snapshots and computes
content- and mode-aware net path transitions. Guard adapters decode one exact
typed Codex hook correlation and supply invocation targets or reviewed hints.
Store owns one aggregate row per exact host tool invocation and applies the
pre-tool and post-tool mutations atomically.

Core consumes only Store-validated actual Unrecorded Changes. Operational
observation-unavailable diagnostics remain separate from reconciliation and
close-readiness state.

## Invariants

- An allowed write-capable or unknown-effect invocation has a persisted stable
  pre-tool baseline.
- One complete observation uses one exact matching pre/post hook pair.
- A deterministic delta is calculated only from compatible stable snapshots.
- Expected writes match only their exact observation and complete delta.
- An Unrecorded Change contains only a non-empty unmatched observed delta.
- Replay uses the stored terminal result and never rescans the repository.
- Observation results do not claim actor identity or exclusive causation.

## Responsibility boundaries

`volicord-host-contract` owns typed hook correlation and the canonical Product
Repository effect catalog. `volicord-platform-fs` owns snapshot and delta
observation. CLI Guard modules own host adaptation and policy projection.
`volicord-store` owns strict aggregate persistence, digest verification,
atomic pre/post mutation, exact expected-write matching, and Unrecorded Change
materialization. Core owns reconciliation and close-readiness interpretation
of validated Unrecorded Change facts.

## Execution flow

1. Pre-tool adaptation decodes the exact hook invocation and captures a stable
   baseline.
2. Store atomically records the pre-tool event, invocation observation, and
   exact expected write.
3. Post-tool adaptation captures a stable outcome for the same invocation.
4. Store atomically verifies the open observation, records the post-tool
   event, stores the delta, matches the expected write, and creates any
   unmatched Unrecorded Change.
5. Core reads validated unresolved changes for reconciliation and close
   readiness.
6. Exact replay returns the stored terminal observation result.

## Failure behavior

Write-capable and unknown-effect invocations are denied when their baseline
cannot be captured or atomically persisted. A no-product-write invocation may
continue with an explicit unavailable observation. Missing, conflicting,
corrupt, or unavailable baseline state never becomes an empty delta or an
Unrecorded Change. Transaction failure rolls back the complete aggregate.

## Scope exclusions

This design does not define public method behavior, physical DDL, closed value
meanings, security guarantees, host process exit behavior, actor identity,
complete monitoring, or OS enforcement.

## Implementation routes

- [`crates/volicord-platform-fs/src/repository_observation/`](../../../../crates/volicord-platform-fs/src/repository_observation/):
  stable snapshots and deterministic deltas.
- [`crates/volicord-cli/src/guard_command/`](../../../../crates/volicord-cli/src/guard_command/):
  typed Codex hook adaptation and Guard result projection.
- [`crates/volicord-store/src/guards.rs`](../../../../crates/volicord-store/src/guards.rs):
  exact host correlation, repository-observation aggregates, expected writes,
  and Unrecorded Changes.
- [`crates/volicord-core/src/methods/reconcile_changes.rs`](../../../../crates/volicord-core/src/methods/reconcile_changes.rs)
  and [`close_readiness/`](../../../../crates/volicord-core/src/close_readiness/):
  reconciliation and close-readiness consumers.

## Reference owners

Exact behavior remains in
[Repository Observation](../../reference/repository-observation.md),
[Reconcile Changes](../../reference/api/method-reconcile-changes.md),
[Storage Records](../../reference/storage-records.md),
[Storage DDL](../../reference/storage-ddl.md),
[Storage Effects](../../reference/storage-effects.md), and
[Security](../../reference/security.md).
