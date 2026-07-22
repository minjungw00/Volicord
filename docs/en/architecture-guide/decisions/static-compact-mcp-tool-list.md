# MCP uses a static compact tool list by default

## Context

An MCP `tools/list` response is injected into agent context before ordinary
work begins. Embedding full request examples and long branch explanations in
every runtime tool schema consumes context even when the agent needs only one
next action. A large schema can also encourage speculative workflow calls.

Changing the visible tool set at every Task transition could reduce some
metadata, but it introduces client capability, cache invalidation, discovery,
and recovery questions. Those questions need evidence from real agent
evaluation before they become the default interaction model.

## Decision

Keep one static tool set for each existing connection and storage capability
mode, and make its runtime projection compact.

- Runtime schemas omit illustrative examples and keep descriptions focused on
  result, authority boundary, and when the tool is applicable.
- Documentation and contract fixtures may retain examples through a separate
  documentation projection.
- Workflow responses carry an actionable `next_action` with its owner method so
  the agent follows returned state instead of probing tools speculatively.
- Generated contract snapshots and a serialized-size test protect the compact
  runtime projection.
- State-dependent dynamic tool-list changes remain a possible extension only
  after client capability and agent-evaluation evidence justify them.

The exact public tool set, schema projection, byte bound, `next_action` shape,
degraded-mode behavior, and snapshot contract remain owned by
[MCP Transport](../../reference/mcp-transport.md), public API schema owners,
and Agent Connection.

## Consequences

- Every client can discover a stable set without supporting mid-session
  tool-list changes.
- Examples remain available to documentation and tests without occupying every
  runtime prompt.
- The adapter must populate next-action routing consistently across normal and
  bounded recovery responses.
- Snapshot changes require semantic review; reducing byte size cannot remove
  validation or authority fields.
- Agent evaluation can compare context and call cost before a dynamic-list
  extension is promoted.

## Non-goals

- This decision does not remove public methods or make their input validation
  less strict.
- It does not forbid dynamic tool lists forever.
- It does not make `next_action` a new authority source; it is a projection of
  current owner-defined state.
- A byte target is not proof that an agent will choose the correct tool.

## Rejected alternatives

- Keeping full examples in runtime schemas was rejected because the examples
  repeat on every discovery response and belong in documentation projection.
- Making the tool list Task-phase-specific by default was rejected because the
  current client and evaluation evidence is insufficient for the added stateful
  discovery behavior.
- Shrinking schemas by dropping validation detail was rejected because compact
  transport must preserve the public contract.

## Relevant implementation

- [`crates/volicord-mcp/src/tool_registry.rs`](../../../../crates/volicord-mcp/src/tool_registry.rs):
  tool definitions, descriptions, runtime and documentation schema projection.
- [`crates/volicord-mcp/src/stdio.rs`](../../../../crates/volicord-mcp/src/stdio.rs):
  `tools/list`, workflow response wrapping, and bounded recovery projections.
- [`crates/volicord-types/src/schema.rs`](../../../../crates/volicord-types/src/schema.rs):
  shared next-action response shape.
- [`tests/integration/public_contract_snapshots.rs`](../../../../tests/integration/public_contract_snapshots.rs):
  generated public contract projection and reviewed snapshots.

## Related tests and Reference owners

- MCP protocol-projection unit tests in
  [`crates/volicord-mcp/src/tests/protocol_projection.rs`](../../../../crates/volicord-mcp/src/tests/protocol_projection.rs),
  process coverage in
  [`crates/volicord-cli/tests/mcp_transport.rs`](../../../../crates/volicord-cli/tests/mcp_transport.rs),
  and integration coverage in
  [`tests/integration/mcp_connection.rs`](../../../../tests/integration/mcp_connection.rs).
- [MCP Transport](../../reference/mcp-transport.md),
  [API Methods](../../reference/api/methods.md),
  [API State Schemas](../../reference/api/schema-state.md), and
  [Agent Connection](../../reference/agent-connection.md).
