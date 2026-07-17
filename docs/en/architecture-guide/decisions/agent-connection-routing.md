# Agent Connection Routing

## Context

The first release accepts only `host_kind=codex` and
`integration_profile=record` through managed stdio.
A connection can be installed for one user or one selected project, while Core
requests must still resolve an explicit allowed Product Repository.

## Decision

Store one Agent Connection with exact `host_kind=codex`,
`integration_profile=record`, and `connection_scope=personal|shared`.
Maintain explicit project membership for that connection. Start each managed
stdio process with one selected connection and one selected allowed project.

The adapter validates the binding, receipt, Runtime Home, StorageManifest, and
project selection before accepting requests. It derives connection and project
context locally; public tool arguments cannot choose or override them.

A personal connection changes user-owned Codex configuration. A shared
connection changes the supported project-owned Codex configuration. Both use
the same Core and stdio boundary.

## Consequences

- One process cannot silently cross connection or project boundaries.
- Moving or replacing the project requires owner-defined verification or repair.
- Connection records do not grant operating-system permission or prove user
  identity.
- The CLI inbox remains the only UserAction resolution channel.

Exact fields and commands belong to
[Agent Connection](../../reference/agent-connection.md),
[Administrative CLI](../../reference/admin-cli.md), and
[MCP Transport](../../reference/mcp-transport.md).
