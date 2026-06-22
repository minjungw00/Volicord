# Core and adapter dependency boundary

## Context

Harness public method behavior needs to be reachable through an adapter without
letting the adapter define method semantics. The Rust workspace also has a
local administrative CLI that prepares Runtime Home and host configuration, but
those commands are not public Harness API methods.

## Decision

Core-facing behavior lives in `harness-core` and depends on shared types and
Store, not on `harness-mcp` or `harness-cli`. MCP and CLI adapters may depend on
lower layers for their own responsibilities:

- `harness-mcp` owns stdio startup, session binding, tool metadata, typed
  argument decoding, invocation-context derivation, and response wrapping, then
  calls `CoreService` for public method execution.
- `harness-cli` owns local administrative setup, registration, setup planning,
  preflight orchestration, and host config generation through Store and shared
  types, not through public Core methods.

This resembles a ports-and-adapters dependency direction, but this page names
only the structure visible in the repository.

## Consequences

- `CoreService` can be tested directly without starting MCP stdio.
- MCP integration tests can compare adapter-visible behavior with direct Core
  behavior.
- Adapter startup validation can use Store directly, but that Store use is not
  alternate public method behavior.
- Public method additions or behavior changes must update Core and Reference
  owners, not only adapter dispatch.

## Non-Goals

- This decision does not define the public method list or method behavior.
- It does not make CLI commands public API methods.
- It does not define MCP transport contracts or security guarantees.
- It does not prevent adapters from doing their own startup, binding, or config
  validation.

## Relevant Implementation

- [`crates/harness-core/src/pipeline.rs`](../../../../crates/harness-core/src/pipeline.rs):
  `CoreService`, `MethodPolicy`, `OwnerPipelineBranch`, and common preflight.
- [`crates/harness-mcp/src/lib.rs`](../../../../crates/harness-mcp/src/lib.rs):
  `PUBLIC_METHOD_TOOL_NAMES`, `McpIntegrationStartupInspection`,
  `McpIntegrationContext`, `McpAdapter`, `McpAdapter::call_tool`, and
  `prepare_integration_arguments`.
- [`crates/harness-cli/src/agent_command.rs`](../../../../crates/harness-cli/src/agent_command.rs):
  administrative host setup orchestration outside the Core/MCP adapter path.
- [`crates/harness-cli/src/registration.rs`](../../../../crates/harness-cli/src/registration.rs):
  registered surface capability and local-access metadata helpers.
- Cargo manifests for `harness-core`, `harness-mcp`, and `harness-cli`.

## Related Tests And Reference Owners

- `adapter_and_direct_core_status_have_equivalent_response_meaning` in
  [`crates/harness-mcp/src/lib.rs`](../../../../crates/harness-mcp/src/lib.rs).
- `mcp_session_derives_access_per_method_call` and
  `mcp_exposes_exactly_the_documented_public_methods` in
  [`tests/integration/mcp_surface.rs`](../../../../tests/integration/mcp_surface.rs).
- [API Methods](../../reference/api/methods.md), [MCP Transport](../../reference/mcp-transport.md),
  [Administrative CLI](../../reference/admin-cli.md), and
  [Agent Integration](../../reference/agent-integration.md).
