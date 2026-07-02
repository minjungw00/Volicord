# Core and adapter dependency boundary

## Context

Volicord public method behavior needs to be reachable through an adapter without
letting the adapter define method semantics. The Rust workspace also has a
local administrative CLI that prepares Runtime Home and host configuration, but
those commands are not public Volicord API methods.

## Decision

Core-facing behavior lives in `volicord-core` and depends on shared types and
Store, not on `volicord-mcp` or `volicord-cli`. MCP and CLI adapters may depend on
lower layers for their own responsibilities:

- `volicord-mcp` owns stdio startup, session binding, tool metadata, typed
  argument decoding, invocation-context derivation, and response wrapping, then
  calls `CoreService` for public method execution.
- `volicord-cli` owns local administrative setup, registration, setup planning,
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

- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs):
  `CoreService`, `MethodPolicy`, `OwnerPipelineBranch`, and common preflight.
- [`crates/volicord-mcp/src/tool_registry.rs`](../../../../crates/volicord-mcp/src/tool_registry.rs):
  `PUBLIC_METHOD_TOOL_NAMES`, `McpToolDefinition`, and public tool metadata.
- [`crates/volicord-mcp/src/routing.rs`](../../../../crates/volicord-mcp/src/routing.rs):
  `McpConnectionStartupInspection`, `McpConnectionContext`, and startup/project
  routing helpers.
- [`crates/volicord-mcp/src/adapter.rs`](../../../../crates/volicord-mcp/src/adapter.rs):
  `McpAdapter`, `McpAdapter::call_tool`, typed argument preparation, trusted
  envelope construction, and Core dispatch.
- [`crates/volicord-mcp/src/stdio.rs`](../../../../crates/volicord-mcp/src/stdio.rs):
  JSON-RPC stdio dispatch, `tools/call` result wrapping, and elicitation
  handling.
- [`crates/volicord-cli/src/connection_command.rs`](../../../../crates/volicord-cli/src/connection_command.rs):
  administrative host setup orchestration outside the Core/MCP adapter path.
- [`crates/volicord-cli/src/registration.rs`](../../../../crates/volicord-cli/src/registration.rs):
  registered connection capability and invocation metadata helpers.
- Cargo manifests for `volicord-core`, `volicord-mcp`, and `volicord-cli`.

## Related Tests And Reference Owners

- `adapter_and_direct_core_status_have_equivalent_response_meaning` in
  [`crates/volicord-mcp/src/tests.rs`](../../../../crates/volicord-mcp/src/tests.rs).
- `connection_invocation_is_injected_and_single_project_is_auto_selected` and
  `read_only_mode_rejects_agent_workflow_methods_before_core` in
  [`tests/integration/mcp_connection.rs`](../../../../tests/integration/mcp_connection.rs).
- [API Methods](../../reference/api/methods.md), [MCP Transport](../../reference/mcp-transport.md),
  [Administrative CLI](../../reference/admin-cli.md), and
  [Agent Connection](../../reference/agent-connection.md).
