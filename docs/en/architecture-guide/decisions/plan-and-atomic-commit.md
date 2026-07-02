# Planning before atomic mutation commit

## Context

Public methods need method-specific validation and planning, but normal
committed effects must be applied atomically with state-version changes,
events, replay records, and response JSON storage. Store should not own method
policy, and method planners should not own SQL transaction mechanics.

## Decision

Method modules plan before effect execution. A planner selects an
`OwnerPipelineBranch`, and committed branches provide result fields, event
payloads, and `CoreStorageMutation` values. The shared Core pipeline builds
`CommitMutationInput`, then `CoreProjectStore::commit_mutation` applies the
normal committed mutation inside one immediate Store transaction.

Transient artifact staging is a separate storage-owned effect path and does
not use the normal Core mutation commit.

## Consequences

- Dry-run, read-only, no-effect, transient staging, and committed mutation
  paths remain distinguishable in code and tests.
- Store can enforce replay, stale-state, event append, response storage, and
  rollback behavior at one commit boundary.
- Method code can express intended effects without embedding raw storage
  mechanics.
- Changes to committed method effects usually touch a method planner, Store
  mutation application, focused tests, and the applicable Reference owner.

## Non-Goals

- This decision does not define exact storage effects for any public method.
- It does not reproduce DDL, storage records, or schema field meanings.
- It does not make dry-run or no-effect branches product acceptance.
- It does not require artifact staging to become a normal Core mutation commit.

## Relevant Implementation

- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs):
  `OwnerPipelineBranch`, `CoreService::execute_prepared_request`, and Core
  commit orchestration.
- [`crates/volicord-core/src/methods/`](../../../../crates/volicord-core/src/methods/):
  method-specific planners such as `plan_intake` and `plan_prepare_write`.
- [`crates/volicord-store/src/core_pipeline.rs`](../../../../crates/volicord-store/src/core_pipeline.rs):
  `CoreStorageMutation`, `CommitMutationInput`,
  `CoreProjectStore::commit_mutation`, `MutationCommitOutcome`, and
  `ProjectMutation`.
- [`crates/volicord-store/src/artifacts.rs`](../../../../crates/volicord-store/src/artifacts.rs):
  `CoreProjectStore::create_artifact_staging`.

## Related Tests And Reference Owners

- `committed_mutation_increments_state_version_once`,
  `idempotency_replay_returns_stored_response`, and
  `stale_expected_state_version_is_rejected_without_effect` in
  [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs).
- `transaction_replay_returns_stored_response_before_stale_expected_state` and
  `transaction_replay_hash_conflict_rejects_without_effect` in
  [`crates/volicord-store/src/core_pipeline.rs`](../../../../crates/volicord-store/src/core_pipeline.rs).
- `stage_artifact_creates_transient_handle_without_core_commit` in
  [`crates/volicord-core/src/methods/tests.rs`](../../../../crates/volicord-core/src/methods/tests.rs).
- [Storage Effects](../../reference/storage-effects.md),
  [Storage Versioning](../../reference/storage-versioning.md), and the linked
  public method owner from [API Methods](../../reference/api/methods.md).
