# Agent Integration Profile and host routing

## Context

Harness needs direct coding-agent integration for hosts such as Codex and
Claude Code while still supporting more than one registered Product Repository.
The integration-bound MCP startup model binds a server process to one Agent
Integration Profile rather than to one Product Repository. That shape supports
user-scoped host configuration, explicit multi-project allowlists, and
host-specific trust and approval flows.

MCP clients may provide roots or launch-directory context, but those values are
host hints. They are not Harness authority and cannot safely select a project
by themselves.

## Decision

Harness uses an Agent Integration Profile as the durable registry identity for
one coding-agent integration. An MCP server process is started for an
`integration_id`; project access is selected and validated per tool call rather
than fixed at process startup.

The design keeps these responsibilities separate:

- The registry stores the integration identity, bound coding-agent surface
  identity, explicit project membership, optional default project, and managed
  Host Installation inventory.
- `harness-mcp` validates the integration at startup, derives the bound surface
  context from the profile, exposes the public Harness tools plus the
  `harness.list_projects` helper, and rejects ambiguous project selection.
- The administrative CLI creates, verifies, updates, and removes integration
  setup for supported hosts, including project-scoped configuration and
  repository guidance files when explicitly authorized.
- Host trust, project approval, OAuth, reload, restart, and model behavior stay
  with the external host and user.

## Consequences

- A user-scoped host configuration can serve multiple explicitly added projects
  without granting all registered projects.
- Revoking or adding project membership does not require rewriting the host MCP
  command when the command already points at the same `integration_id`.
- Project selection failures become deterministic: the adapter can report
  missing or ambiguous project selection and direct the agent to list allowed
  projects.
- Host setup status can distinguish configured-but-awaiting-host-action from
  complete verification.
- Generated host configuration uses `harness-mcp --integration
  <integration_id>` and does not require project, surface, or surface-instance
  environment variables.

## Non-Goals

- This decision does not add a public Harness API method.
- It does not make CLI commands public API methods.
- It does not make MCP roots, current working directory, or host labels Harness
  authority.
- It does not grant all registered projects to a user-scoped host installation.
- It does not make repository guidance, MCP server instructions, or host rule
  files enforce model behavior.
- It does not permit Harness runtime state, SQLite databases, generated logs,
  QA results, acceptance records, close-readiness state, or residual-risk
  records in the Product Repository.

## Relevant Implementation Areas

Implementation work that conforms to this decision belongs in:

- [`crates/harness-mcp`](../../../../crates/harness-mcp): integration-bound
  startup, MCP initialization, tool discovery, project selection, and adapter
  validation before Core calls.
- [`crates/harness-cli`](../../../../crates/harness-cli): administrative
  install/status/verify/uninstall flows and repository guidance management.
- [`crates/harness-store`](../../../../crates/harness-store): registry schema,
  migrations, integration membership, Host Installation inventory, and Runtime
  Home access.
- Shared types used by those crates for stored value sets and machine-readable
  administrative output.

## Related Tests And Reference Owners

Tests for this design should cover startup validation, project selection,
membership revocation, host setup status, repository-write approval, managed
marker replacement, and rejection of unsupported startup forms.

Reference owners:

- [Agent Integration](../../reference/agent-integration.md)
- [MCP Transport](../../reference/mcp-transport.md)
- [Administrative CLI](../../reference/admin-cli.md)
- [Runtime Boundaries](../../reference/runtime-boundaries.md)
- [Storage Records](../../reference/storage-records.md)
- [Storage DDL](../../reference/storage-ddl.md)
- [Storage Versioning](../../reference/storage-versioning.md)
- [Security](../../reference/security.md)
