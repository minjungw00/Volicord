# Repository Observation Boundary Design

## Purpose

This design explains the current implementation boundary for invocation-scoped
Product Repository snapshots, deterministic deltas, Guard aggregation,
expected-write matching, and Unrecorded Change creation.

## Design

`volicord-platform-fs` captures bounded stable snapshots and computes
content- and mode-aware net path transitions. A directly observed regular file
uses one bounded byte stream to derive its exact worktree-byte identity and to
feed Git's current path-aware clean conversion for the canonical blob identity.
An immutable-tree regular file carries only its canonical Git blob identity.
Direct-to-direct comparison uses exact worktree bytes; comparison involving a
tree-derived state uses canonical Git content. Guard adapters decode one exact
typed Codex hook correlation and supply invocation targets or reviewed hints.
The exact `PreToolUse` snapshot is the repository baseline and the matching
`PostToolUse` snapshot is the repository outcome. Store owns one aggregate row
per exact host tool invocation and applies the pre-tool and post-tool mutations
atomically.

Core consumes only Store-validated actual Unrecorded Changes. Operational
observation-unavailable diagnostics remain separate from reconciliation and
close-readiness state.

## Invariants

- An allowed write-capable or unknown-effect invocation has a persisted stable
  pre-tool baseline.
- One complete observation uses one exact matching pre/post hook pair.
- The aggregate state is exactly `open`, `complete`, or `unavailable`.
- A deterministic delta is calculated only from compatible stable snapshots.
- Every directly observed regular file has exact worktree-byte and canonical
  Git blob identities.
- Every tree-derived regular file has a canonical Git blob identity without
  worktree-byte evidence.
- Regular-file comparison selects the exact-byte or canonical Git domain from
  the two typed evidence sources and includes executable mode.
- Expected writes match only their exact observation and complete delta.
- An Unrecorded Change contains only a non-empty unmatched observed delta.
- Replay uses the stored terminal result and never rescans the repository.
- Observation results do not claim actor identity or exclusive causation.

## Responsibility boundaries

`volicord-host-contract` owns typed hook correlation and the canonical Product
Repository effect catalog. `volicord-platform-fs` owns snapshot and delta
observation, path-aware Git conversion policy, and bounded file streaming.
`volicord-platform-process` supplies child-process containment for Git
conversion. CLI Guard modules own host adaptation and policy projection.
`volicord-store` owns strict aggregate persistence, digest verification,
atomic pre/post mutation, exact expected-write matching, and Unrecorded Change
materialization. Core owns reconciliation and close-readiness interpretation
of validated Unrecorded Change facts.

## Execution flow

1. Pre-tool adaptation decodes the exact hook invocation and captures a stable
   baseline, including both regular-file content identities for directly
   observed worktree files.
2. Store atomically records the pre-tool event, invocation observation, and
   exact expected write.
3. Post-tool adaptation captures a stable outcome for the same invocation
   under the same typed content-evidence contract.
4. Store atomically verifies the open observation, records the post-tool
   event, rejects semantically empty stored transitions, stores the delta,
   matches the expected write, and creates any unmatched Unrecorded Change.
5. Core reads validated unresolved changes for reconciliation and close
   readiness.
6. Exact replay returns the stored terminal observation result.

## Failure behavior

Write-capable and unknown-effect invocations are denied when their baseline
cannot be captured or atomically persisted. A no-product-write invocation may
continue with an explicit unavailable observation. Missing, conflicting,
corrupt, or unavailable baseline state never becomes an empty delta or an
Unrecorded Change. Git conversion, filter, process, containment, malformed
output, and resource-limit failures produce an unavailable observation.
Transaction failure rolls back the complete aggregate.

## Scope exclusions

This design does not define public method behavior, physical DDL, closed value
meanings, security guarantees, host process exit behavior, actor identity,
complete monitoring, or OS enforcement.

## Implementation routes

- [`crates/volicord-platform-fs/src/repository_observation/`](../../../../crates/volicord-platform-fs/src/repository_observation/):
  stable snapshots and deterministic deltas.
- [`crates/volicord-cli/src/guard_command/`](../../../../crates/volicord-cli/src/guard_command/):
  typed Codex hook adaptation and Guard result projection.
- [`crates/volicord-store/src/guards.rs`](../../../../crates/volicord-store/src/guards.rs)
  and
  [`guards/repository_observation.rs`](../../../../crates/volicord-store/src/guards/repository_observation.rs):
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
