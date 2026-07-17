# Core And Adapter Dependency Boundary

## Context

The same workflow is reached from a local CLI and a managed stdio MCP process.
Transport details, user-interface concerns, and host configuration must not
change Core authority meaning.

## Decision

`volicord-core` depends on shared types and Store-facing interfaces, never on
CLI or MCP crates. Core owns common preflight, structural validation order,
method planning, replay, policy, response construction, and commit selection.

The MCP adapter owns stdio lifecycle, JSON-RPC framing, tool metadata, public
argument decoding, server-owned invocation context, and safe projection. The
CLI owns administrative parsing, Codex setup, diagnostics, CLI inbox rendering,
and local-user provenance. Both call typed Core-facing interfaces.

Store owns strict persisted-record validation and transaction application. It
does not infer a product contract from adapter input.

## Consequences

- Public arguments cannot inject connection, actor, or project authority.
- Adapter projections cannot widen a Core result.
- MCP can create or resume UserAction requests but cannot resolve them.
- Persisted owner data that fails its current typed contract is a corrupt-data
  failure, not an adapter availability failure.
- New adapter behavior requires an owner-defined contract before implementation.

See [Request Lifecycle](../request-lifecycle.md),
[MCP Transport](../../reference/mcp-transport.md), and
[Failure Model](../../reference/failure-model.md).
