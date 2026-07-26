# Static Compact MCP Tool-List Design

## Purpose

This design explains how the current MCP adapter projects one closed
mode-specific tool catalog through the selected protocol capabilities while
keeping runtime schemas and mutation results bounded.

## Design

`volicord-types::tool_names::AgentToolId` is the canonical identity catalog for
Core methods and operational verification tools. `volicord-mcp-protocol`
selects one reviewed production profile and its semantic capabilities.
`tool_registry.rs` builds the canonical definitions once, filters them by
Connection mode, and projects their wire definitions through the selected
capabilities.

Runtime request schemas omit documentation-only examples. Tool results use
typed canonical content, schema validation, compact mutation projection,
bounded committed-result recovery, and owner-method-bearing next-action data.
Tool ownership does not fork by protocol revision.

## Invariants

- The current tool catalog is closed and keyed by `AgentToolId`.
- Connection mode filters availability without creating Task-phase discovery
  state.
- Protocol capabilities affect wire projection, not the semantic owner of a
  tool.
- Runtime schemas preserve required validation and authority fields while
  omitting documentation-only material.
- Compact mutation results retain actionable recovery and next-action
  coordinates.
- Generated contract snapshots are derived checks, not alternate tool owners.

## Responsibility boundaries

`volicord-types` owns tool identity and shared method/result types.
`volicord-mcp-protocol` owns protocol profiles and capability selection.
`volicord-mcp` owns canonical registry construction, mode filtering, schema
projection, dispatch, and result wrapping. Core and focused Reference owners
retain method behavior.

## Execution flow

1. MCP initialization selects a supported protocol profile.
2. Adapter startup resolves the current Connection mode and session context.
3. `tools/list` obtains the canonical mode-specific catalog.
4. Each definition projects through the selected protocol capabilities.
5. `tools/call` resolves the wire name to `AgentToolId`, decodes the current
   argument type, and dispatches to its owner.
6. The adapter validates and wraps the complete, compact, or recovery result.

## Failure behavior

Unknown tools, unsupported protocol profiles, missing required catalog entries,
invalid schemas, oversized projections, and post-effect response failures
retain their typed protocol or adapter routes. The adapter does not silently
drop validation fields, choose a version-specific alternate registry, or
re-execute a committed mutation to rebuild output.

## Scope exclusions

This design does not define the public tool list, schema fields, byte limits,
Connection modes, `next_action` meaning, or protocol support. It does not
provide state-dependent dynamic tool discovery.

## Implementation routes

- [`crates/volicord-types/src/tool_names.rs`](../../../../crates/volicord-types/src/tool_names.rs):
  canonical tool identity and verification roles.
- [`crates/volicord-mcp-protocol/src/lib.rs`](../../../../crates/volicord-mcp-protocol/src/lib.rs):
  closed production profiles and semantic capabilities.
- [`crates/volicord-mcp/src/tool_registry.rs`](../../../../crates/volicord-mcp/src/tool_registry.rs):
  canonical definitions, mode filtering, and projection.
- [`crates/volicord-mcp/src/tool_dispatch.rs`](../../../../crates/volicord-mcp/src/tool_dispatch.rs),
  [`mutation_projection.rs`](../../../../crates/volicord-mcp/src/mutation_projection.rs), and
  [`committed_result_recovery.rs`](../../../../crates/volicord-mcp/src/committed_result_recovery.rs):
  dispatch and bounded result paths.

## Reference owners

Exact behavior remains in [MCP Transport](../../reference/mcp-transport.md),
[Agent Connection](../../reference/agent-connection.md),
[API Methods](../../reference/api/methods.md), and the focused API schema and
method owners.
