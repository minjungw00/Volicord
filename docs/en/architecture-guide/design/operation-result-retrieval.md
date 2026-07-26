# Operation-Result Retrieval Design

## Purpose

This design describes how Core retrieves a bounded exact historical operation
result from the immutable replay response without repeating the original
mutation or creating another result store.

## Design

Normal committed Core mutations persist their serialized public response in
the replay row owned by Store. Eligible projections carry an
`OperationResultRef` derived from that row. The read-only Core method validates
the current invocation and reference, asks Store for the exact scoped replay
response, verifies its content facts, and returns a UTF-8-safe page.

MCP mutation projection keeps a compact actionable result when the complete
body exceeds the transport budget. `committed_result_recovery.rs` and
`mutation_projection.rs` preserve the operation-result reference across that
projection so the caller can use the independent read-only method.

## Invariants

- Exact replay and exact retrieval read the same immutable response body.
- A result reference and cursor are locators, not authorization credentials.
- Every page revalidates current access, result identity, cursor binding, and
  integrity before returning bytes.
- Page boundaries do not split UTF-8 code points.
- Retrieval does not replay effects, append authority events, create replay
  rows, or advance project state.
- Historical output remains distinct from current Core authority.

## Responsibility boundaries

Core owns invocation validation, reference and cursor checks, integrity
decisions, and page composition. Store owns the scoped immutable replay-row
read. MCP owns compact mutation projection and preservation of recovery
coordinates; it does not rebuild the historical result.

## Execution flow

1. A normal committed mutation stores the exact serialized response in its
   replay row.
2. Core derives the result reference for an eligible projection.
3. The MCP adapter returns the complete or compact projection with that
   reference.
4. The read-only retrieval method validates the request and loads the exact
   replay response.
5. Core verifies the reference and cursor, selects a bounded UTF-8 page, and
   returns the next cursor when needed.

## Failure behavior

Missing, corrupt, malformed, cross-project, cross-actor, cross-result, or
cursor-incompatible reads fail before returning a partial page. Response-budget
failure does not cause mutation replay, artifact substitution, or a duplicate
result table.

## Scope exclusions

This design does not define the public method schema, page limits, error
codes, retention, or access contract. It is not a general artifact, event, or
Runtime Home file-download architecture and does not make prior results
current.

## Implementation routes

- [`crates/volicord-core/src/methods/operation_result.rs`](../../../../crates/volicord-core/src/methods/operation_result.rs):
  read-only planning, validation, and paging.
- [`crates/volicord-store/src/core_pipeline/replay.rs`](../../../../crates/volicord-store/src/core_pipeline/replay.rs):
  scoped replay-row lookup.
- [`crates/volicord-mcp/src/committed_result_recovery.rs`](../../../../crates/volicord-mcp/src/committed_result_recovery.rs)
  and [`mutation_projection.rs`](../../../../crates/volicord-mcp/src/mutation_projection.rs):
  bounded mutation projection and recovery coordinates.
- [`crates/volicord-mcp/src/tool_dispatch.rs`](../../../../crates/volicord-mcp/src/tool_dispatch.rs):
  transport dispatch and final projection selection.

## Reference owners

Exact behavior remains in
[`volicord.get_operation_result`](../../reference/api/method-get-operation-result.md),
[API Schema Core](../../reference/api/schema-core.md),
[MCP Transport](../../reference/mcp-transport.md),
[Storage Records](../../reference/storage-records.md),
[Storage Effects](../../reference/storage-effects.md),
[Storage Versioning](../../reference/storage-versioning.md), and
[Security](../../reference/security.md).
