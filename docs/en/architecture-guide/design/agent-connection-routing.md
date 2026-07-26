# Agent Connection Routing Design

## Purpose

This design explains how the managed MCP path binds one running adapter to its
current Agent Connection, Runtime Home, and explicitly admitted Product
Repository without accepting those authority coordinates from public tool
arguments.

## Design

The hidden CLI launcher validates the current managed configuration and
creates a one-time launch lease in the Registry. `volicord-mcp` consumes the
claim in memory, resolves the Connection and repository binding, records the
runtime and project-session observations, and constructs `McpAdapter` with a
typed `McpConnectionContext`.

The public `volicord mcp serve` path uses the same transport implementation but
remains a manual runtime source. Connection scope, mode, project membership,
integration revision, and active session checks remain separate typed
coordinates.

## Invariants

- One adapter process has one admitted Runtime Home and Connection identity.
- Project routing resolves only through current Connection Project membership.
- Public arguments do not select or replace connection, actor, session, or
  project authority.
- A mutation context must carry the same canonical Runtime Home identity as the
  adapter routing context.
- Stored configuration and session observations do not by themselves prove
  actor, binary, or operating-system identity.

## Responsibility boundaries

`volicord-cli` owns managed configuration, launcher orchestration, and local
operator reporting. `volicord-mcp` owns startup binding, lifecycle admission,
request-time routing, and adapter-derived invocation facts.
`volicord-store` owns Connection, membership, lease, runtime-session, and
project-session persistence. Core receives a verified typed invocation context
and remains independent of host configuration.

## Execution flow

1. The hidden launcher revalidates the managed entry and issues a one-time
   Registry lease.
2. MCP startup consumes the lease, resolves the canonical Runtime Home and
   Connection, and records the runtime session.
3. Initialization selects the current protocol profile and records lifecycle
   milestones.
4. Each tool call revalidates Connection mode, project membership, runtime and
   project sessions, revisions, and mutation-context identity.
5. The adapter derives `InvocationContext` and calls Core for public method
   execution.

## Failure behavior

Missing, stale, mismatched, corrupt, or already-consumed routing state stops the
managed path before Core execution. Lifecycle and routing failures retain their
typed diagnostic identity; adapters do not convert them into an empty project
selection or alternate Connection.

## Scope exclusions

This design does not define host trust, user identity, OS permission, public
Connection fields, session-authorization contracts, or command behavior. It
does not create a second user-action resolution channel.

## Implementation routes

- [`crates/volicord-cli/src/host_launch.rs`](../../../../crates/volicord-cli/src/host_launch.rs)
  and [`host_integration/`](../../../../crates/volicord-cli/src/host_integration/):
  managed-entry validation and launch orchestration.
- [`crates/volicord-mcp/src/binding.rs`](../../../../crates/volicord-mcp/src/binding.rs),
  [`routing.rs`](../../../../crates/volicord-mcp/src/routing.rs), and
  [`adapter.rs`](../../../../crates/volicord-mcp/src/adapter.rs): startup,
  routing, and Core invocation context.
- [`crates/volicord-store/src/agent_connections.rs`](../../../../crates/volicord-store/src/agent_connections.rs),
  [`managed_launch_leases.rs`](../../../../crates/volicord-store/src/managed_launch_leases.rs),
  and [`operational_sessions.rs`](../../../../crates/volicord-store/src/operational_sessions.rs):
  persisted routing and session state.

## Reference owners

Exact behavior remains in [Agent Connection](../../reference/agent-connection.md),
[MCP Transport](../../reference/mcp-transport.md),
[Administrative CLI](../../reference/admin-cli.md),
[Runtime Boundaries](../../reference/runtime-boundaries.md), and
[Security](../../reference/security.md).
