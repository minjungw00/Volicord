# Plan And Atomic Commit Design

## Purpose

This design explains how method modules plan typed result fields and effects
before Store mutation, and how the shared Core pipeline and aggregate-owned
Store modules complete one atomic commit.

## Design

Each Core method planner returns method-owned fields and planned effects
without constructing the final common response envelope. The result
declarations in `volicord-types::methods` pair a fields-only type with the
complete public result type. `OwnerPipelineBranch<F>` retains that type across
read-only, no-effect, dry-run, staging, rejection, and committed paths.

For a committed branch, Core builds `CommitMutationInput` from grouped
`CoreStorageMutation` values, planned events, replay coordinates, and a typed
response builder. `CoreProjectStore::commit_mutation` opens one immediate
transaction. A thin dispatcher preserves planner order and delegates each
mutation to the aggregate module that owns its validation, SQL, and typed
application facts. The coordinator advances state once, appends events,
stores replay output, and commits or rolls back.

## Invariants

- Method policy and planning remain in Core; SQL mechanics remain in Store.
- One result declaration owns both fields-only planning and complete result
  composition.
- Common branch facts are added only after the execution branch is known.
- Grouped mutations preserve planner order and aggregate ownership.
- A normal committed operation advances project state at most once and commits
  its events, replay response, and aggregate effects together.
- Transient artifact staging remains outside the normal Core mutation commit.

## Responsibility boundaries

Method modules own request-specific validation and planned result fields.
Focused Core policy modules own reusable authority evaluation. The pipeline
owns branch orchestration and final response composition. Store aggregate
modules own strict read/write logic for their records; the commit coordinator
owns only cross-aggregate transaction coordination.

## Execution flow

1. Common preflight derives verified invocation and method policy.
2. The method planner loads typed facts, evaluates focused policy, and creates
   method result fields plus planned effects.
3. The pipeline selects the typed branch.
4. A committed branch constructs the Store commit input and response builder.
5. Store checks replay and freshness under one immediate transaction.
6. Aggregate modules validate and apply their grouped mutations.
7. Store advances state, appends events, serializes the complete typed result,
   stores replay, and commits.

## Failure behavior

Planning failures create no Store mutation. Aggregate validation or SQL failure
rolls back every mutation, event, state update, and replay row in the
transaction. Replay and stale-state checks occur before new effects. A result
composition failure cannot leave an effect without its stored response.

## Scope exclusions

This design does not define any public method result, storage effect, DDL
shape, event meaning, or state-version contract. It does not make dry-run,
no-effect, or staging branches equivalent to a committed Core mutation.

## Implementation routes

- [`crates/volicord-types/src/methods.rs`](../../../../crates/volicord-types/src/methods.rs):
  result declarations and fields-only composition types.
- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs):
  `OwnerPipelineBranch`, preflight, branch execution, and final composition.
- [`crates/volicord-core/src/methods/`](../../../../crates/volicord-core/src/methods/):
  method-specific planners.
- [`crates/volicord-store/src/core_pipeline/commit.rs`](../../../../crates/volicord-store/src/core_pipeline/commit.rs),
  [`mutations.rs`](../../../../crates/volicord-store/src/core_pipeline/mutations.rs), and
  the neighboring aggregate modules: transaction coordination and owned
  mutation application.
- [`crates/volicord-store/src/artifacts.rs`](../../../../crates/volicord-store/src/artifacts.rs):
  separate staging path.

## Reference owners

Exact behavior remains in [API Methods](../../reference/api/methods.md),
[Core Model](../../reference/core-model.md),
[Storage](../../reference/storage.md),
[Storage Effects](../../reference/storage-effects.md), and
[Storage Versioning](../../reference/storage-versioning.md).
