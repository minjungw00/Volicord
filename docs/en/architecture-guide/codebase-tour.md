# Codebase Tour

This guide is a maintainer-oriented path through the Rust workspace. Exact
product behavior belongs to the focused Reference owners; implementation code
must preserve those contracts.

## Workspace Layers

Read the workspace from the outside inward:

1. `volicord-cli` owns the local administrative command surface, Codex
   connection setup, the CLI inbox, and stdio process launch.
2. `volicord-mcp` owns MCP lifecycle, JSON-RPC stdio framing, public tool
   decoding, and response projection.
3. `volicord-core` owns method planning, policy, replay decisions, authority
   results, and atomic commit orchestration.
4. `volicord-store` owns Runtime Home discovery, SQLite access, strict stored
   record validation, and transaction application.
5. `volicord-types` owns shared closed values, identifiers, and canonical
   encodings.
6. `volicord-platform-fs` owns platform-specific filesystem inspection behind
   a narrow internal facade.

Core-facing code does not depend on CLI or MCP adapter details. Adapters derive
server-owned context and submit typed requests to Core.

## Start With One Request

For an MCP call, follow:

- [`crates/volicord-mcp/src/stdio.rs`](../../../crates/volicord-mcp/src/stdio.rs)
  for process lifecycle and framing;
- [`crates/volicord-mcp/src/adapter.rs`](../../../crates/volicord-mcp/src/adapter.rs)
  for public argument decoding and Core dispatch;
- [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs)
  for common preflight, replay, planning, and commit selection;
- the matching file under
  [`crates/volicord-core/src/methods/`](../../../crates/volicord-core/src/methods/)
  for method-specific planning; and
- [`crates/volicord-store/src/core_pipeline/`](../../../crates/volicord-store/src/core_pipeline/)
  for Store validation and atomic effects.

Use [Request Lifecycle](request-lifecycle.md) for the sequence and
[Storage and Transactions](storage-and-transactions.md) for transaction
boundaries.

## Start With Codex Setup

For managed connection work, follow:

- [`crates/volicord-cli/src/connection_command/`](../../../crates/volicord-cli/src/connection_command/)
  for command orchestration;
- [`crates/volicord-cli/src/host_integration/codex/`](../../../crates/volicord-cli/src/host_integration/codex/)
  for Codex discovery, configuration, identity, and verification;
- [`crates/volicord-store/src/agent_connections.rs`](../../../crates/volicord-store/src/agent_connections.rs)
  for stored connection records; and
- [`crates/volicord-mcp/src/stdio.rs`](../../../crates/volicord-mcp/src/stdio.rs)
  for the bound process startup check.

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
- six independent target/environment Codex release-validation cells for exact finalized artifact
  claims.

See [Testing Strategy](testing-strategy.md) and
[Validation](../maintain/validation.md).
