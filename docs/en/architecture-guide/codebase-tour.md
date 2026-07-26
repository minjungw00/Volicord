# Codebase Tour

This guide is a maintainer-oriented path through the Rust workspace. Exact
product behavior belongs to the focused Reference owners; implementation code
must preserve those contracts.

## Workspace Layers

Read the workspace from the outside inward:

1. `volicord-command-model` owns the complete Clap command declaration,
   command DTOs, syntax validation, visibility, and command introspection.
2. `volicord-cli` owns process startup, administrative command dispatch, Codex
   connection setup, the CLI inbox, rendering, and stdio process launch.
3. `volicord-mcp` owns MCP lifecycle, JSON-RPC stdio framing, public tool
   decoding, and response projection.
4. `volicord-core` owns method planning, policy, replay decisions, authority
   results, and atomic commit orchestration.
5. `volicord-store` owns Runtime Home discovery, SQLite access, strict stored
   record validation, and transaction application.
6. `volicord-types` owns shared closed values, identifiers, and canonical
   encodings.
7. `volicord-platform-fs` owns platform-specific filesystem inspection behind
   a narrow internal facade.

Core-facing code does not depend on CLI or MCP adapter details. Adapters derive
server-owned context and submit typed requests to Core.

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
The local CLI inbox renders the strict stored form and invokes the separate
user-only resolution path. MCP never renders or submits that form. Guard prompt
capture is an observation source only.

Read the exact contracts in
[User Action Schemas](../reference/api/schema-user-action.md),
[Request User Action](../reference/api/method-request-user-action.md), and
[Resolve User Action](../reference/api/method-resolve-user-action.md).

## Testing Route

Keep durable checks at the narrowest layer that owns the invariant:

- unit tests beside pure parsing, encoding, and policy;
- crate integration tests for adapter and Store boundaries;
- workspace conformance tests for public cross-method behavior; and
- generic release-integrity tests for target, package, checksum, and workflow invariants.

See [Testing Strategy](testing-strategy.md) and
[Validation](../maintain/validation.md).
