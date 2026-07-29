# Codebase Tour

This guide is a maintainer-oriented path through the Rust workspace. Exact
product behavior belongs to the focused Reference owners; implementation code
must preserve those contracts.

## Workspace Package Route

Start with the generated package responsibility and dependency tables in
[Architecture](architecture.md#workspace-package-architecture). They are
derived from the root Cargo metadata and are the current package-level route.
Use this page for request-oriented reading order and the [Source
Map](source-map.md) for module paths.

## Start With One Request

For an MCP call, follow:

- [`crates/volicord-mcp/src/stdio.rs`](../../../crates/volicord-mcp/src/stdio.rs)
  for the public stream facade;
- [`crates/volicord-mcp/src/binding.rs`](../../../crates/volicord-mcp/src/binding.rs)
  for Runtime Home, repository, Connection, and managed-session binding;
- [`crates/volicord-mcp/src/transport.rs`](../../../crates/volicord-mcp/src/transport.rs)
  and [`json_rpc.rs`](../../../crates/volicord-mcp/src/json_rpc.rs) for bounded
  framing and JSON-RPC envelopes;
- [`crates/volicord-mcp-protocol/src/lib.rs`](../../../crates/volicord-mcp-protocol/src/lib.rs)
  for exact supported-revision selection and the complete semantic capability
  profile consumed by the adapter;
- [`crates/volicord-mcp/src/lifecycle.rs`](../../../crates/volicord-mcp/src/lifecycle.rs)
  for initialize ordering, message admission, the closed session state, and
  termination;
- [`crates/volicord-mcp/src/tool_dispatch.rs`](../../../crates/volicord-mcp/src/tool_dispatch.rs)
  for tool-call decoding, adapter dispatch, and the shared result carrier;
- [`crates/volicord-mcp/src/mutation_projection.rs`](../../../crates/volicord-mcp/src/mutation_projection.rs),
  [`authority_refresh.rs`](../../../crates/volicord-mcp/src/authority_refresh.rs),
  and
  [`committed_result_recovery.rs`](../../../crates/volicord-mcp/src/committed_result_recovery.rs)
  for normal mutation projection, current-authority reread, and authority-first
  recovery of committed results that exceed the selected profile's projection
  budget;
- [`crates/volicord-mcp/src/user_action_projection.rs`](../../../crates/volicord-mcp/src/user_action_projection.rs)
  for the public UserAction result and CLI fallback projection;
- [`crates/volicord-mcp/src/telemetry.rs`](../../../crates/volicord-mcp/src/telemetry.rs)
  and
  [`session_metrics.rs`](../../../crates/volicord-mcp/src/session_metrics.rs)
  for diagnostic facts and runtime-session metrics;
- [`crates/volicord-mcp/src/adapter.rs`](../../../crates/volicord-mcp/src/adapter.rs)
  for context-bound adapter and Core invocation;
- [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs)
  for common preflight, replay, planning, and commit selection;
- the matching file under
  [`crates/volicord-core/src/methods/`](../../../crates/volicord-core/src/methods/)
  for method-specific planning; and
- [`crates/volicord-store/src/core_pipeline/`](../../../crates/volicord-store/src/core_pipeline/)
  for the `CoreProjectStore` facade, aggregate-owned reads and grouped
  mutations, Store validation, replay, and atomic commit coordination.

Use [Request Lifecycle](request-lifecycle.md) for the sequence and
[Storage and Transactions](storage-and-transactions.md) for transaction
boundaries.

## Start With Codex Setup

For managed connection work, follow:

- [`crates/volicord-command-model/src/lib.rs`](../../../crates/volicord-command-model/src/lib.rs)
  for command declaration and parsed command DTOs;
- [`crates/volicord-cli/src/connection_command/`](../../../crates/volicord-cli/src/connection_command/)
  for command orchestration;
- [`crates/volicord-cli/src/host_integration/codex/`](../../../crates/volicord-cli/src/host_integration/codex/)
  for Codex discovery, configuration, identity, and verification;
- [`crates/volicord-store/src/agent_connections.rs`](../../../crates/volicord-store/src/agent_connections.rs)
  for stored connection records; and
- [`crates/volicord-mcp/src/binding.rs`](../../../crates/volicord-mcp/src/binding.rs)
  for the bound process startup check and managed call correlation.

The supported connection intents are `personal` and `shared`; both launch
the Record-profile stdio boundary.

## Start With User-Owned Action

`volicord.request_user_action` creates or resumes the pending Core request.
Follow
[`crates/volicord-user-action-service/src/`](../../../crates/volicord-user-action-service/src/)
for the reusable semantic model, validation, canonical body and identity,
Store-backed typed fact acquisition, materialization, persistence mapping,
resolution, continuity, and projections. Core request orchestration remains in
[`methods/user_action.rs`](../../../crates/volicord-core/src/methods/user_action.rs),
with User Channel reads in the adjacent `user_action_read.rs` module and
continuity persistence in
[`continuity/user_action.rs`](../../../crates/volicord-core/src/continuity/user_action.rs);
reconciliation supplies semantic intent from
[`methods/reconcile_changes.rs`](../../../crates/volicord-core/src/methods/reconcile_changes.rs).

The local CLI inbox reads adapter-neutral Core facts, uses shared presentation
and a canonical command-model invocation to render the strict stored form, and
invokes the separate user-only resolution path. MCP constructs its own safe
projection and never renders or submits that form. Guard prompt capture is an
observation source only.

Read the exact contracts in
[User Action Schemas](../reference/api/schema-user-action.md),
[Request User Action](../reference/api/method-request-user-action.md), and
[Resolve User Action](../reference/api/method-resolve-user-action.md).

## Testing Route

Keep durable checks at the narrowest layer that owns the invariant:

- unit tests beside pure parsing, encoding, policy, and UserAction service semantics;
- crate integration tests for adapter and Store boundaries;
- workspace conformance tests for public cross-method behavior; and
- generic release-integrity tests for target, package, checksum, and workflow invariants.

See [Testing Strategy](testing-strategy.md) and
[Validation](../maintain/validation.md).
