# Agent Connection and host routing

## Context

Volicord needs managed coding-agent host support for Codex and Claude Code, plus user-managed generic MCP host configuration guidance, while still supporting more than one registered `Product Repository`. MCP roots and launch-directory context are host hints. They are not Volicord authority and cannot safely select a Project by themselves.

## Decision

Volicord uses an Agent Connection as the durable registry identity for one local MCP host connection. A `volicord mcp --stdio` process starts with `--connection <connection_id>` and may also carry `--project <project_id>` when the generated host entry is safely bound to one connected Project. Multi-project connections keep project access selected and validated per tool call rather than fixed at process startup.

The design keeps these responsibilities separate:

- The registry stores Agent Connection identity, host kind, host scope, target metadata, connection mode, enabled state, verification state, and explicit Connection Project membership.
- `volicord mcp --stdio` validates the Agent Connection at startup, derives current connection context from that connection, exposes MCP-visible tools according to connection mode, provides `volicord.list_projects`, and rejects ambiguous project selection.
- The administrative CLI creates, verifies, updates, and removes supported host connection setup.
- Host trust, project approval, OAuth, reload, restart, and model behavior stay with the external host and user.

Managed host-hook configuration is derived from one validated command shape
that retains both the rendered command text and the actual execution argv. For
Codex Detective hooks, the generated wrapper is invoked as exactly
`sh -c <generated-hook-script>`. The companion prompt rule therefore matches
the three-token argv prefix `sh`, `-c`, and a closed choice of the exact script
for one of the five required phases: `session-start`, `pre-tool`, `post-tool`,
`prompt-capture`, or `stop`. The rule's positive and negative examples are
validated from that same command shape before configuration is written.

The earlier managed rule used `.codex hooks` as its prefix. That described a
configuration location rather than the argv executed by the host, so current
Codex rule validation could reject the file before any hook ran. This correction
changes only generated managed host configuration. It adds no public API,
storage record, DDL, storage profile, or SemVer change. Existing managed files
are replaced through the normal ownership and fingerprint checks; unmanaged
files are not adopted. Volicord does not define a stable minimum Codex version,
so checked-in parser fixtures remain insufficient by themselves: release
validation must also load and check the generated rule with an applicable real
Codex parser.

## Consequences

- A user-scoped host configuration can serve multiple explicitly connected Projects without granting all registered Projects.
- Adding or removing a connected Project does not require rewriting a multi-project host MCP command when the command already points at the same `connection_id`; project-bound generated entries may be regenerated when their selected Project binding changes.
- Project selection failures are deterministic: the adapter can report missing or ambiguous project selection and direct the agent to list connected Projects.
- Project-bound startup can establish a session-watch baseline before tool handling. Multi-project startup reports watcher coverage as pending until explicit project selection.
- Host setup status can distinguish configured-but-awaiting-host-action from complete verification.
- Generated host configuration prefers `volicord mcp --stdio --connection <connection_id> --project <project_id>` for project-scoped entries and does not require connection-context or actor-provenance environment variables. Connection-only generated entries remain for flows that intentionally serve multiple connected Projects.

## Non-goals

- This decision does not add a public Volicord API method.
- It does not make CLI commands public API methods.
- It does not make MCP roots, current working directory, host labels, or copied `connection_id` values Volicord authority.
- It does not grant all registered Projects to a user-scoped connection.
- It does not make repository guidance, MCP server instructions, or host rule files enforce model behavior.
- It does not permit Volicord runtime state, SQLite databases, generated logs, QA results, acceptance records, close-readiness state, or residual-risk records in the `Product Repository`.

## Rejected alternatives

- Keeping the `.codex hooks` prefix was rejected because it never represents
  the generated hook process argv.
- Matching only `sh -c` was rejected because it would prompt unrelated shell
  commands rather than the closed Volicord hook set.
- Accepting additional scripts, phases, or a second canonical prompt rule was
  rejected because the generated rule would no longer be derived exactly from
  the required hook command set.
- Treating fixture validation as proof of host compatibility was rejected
  because the external Codex parser and its load-time checks remain host-owned.

## Relevant implementation areas

- [`crates/volicord-mcp`](../../../../crates/volicord-mcp): connection-bound startup, MCP initialization, tool discovery, project selection, and adapter validation before Core calls.
- [`crates/volicord-cli`](../../../../crates/volicord-cli): public `volicord mcp` process entry, host configuration command generation, and administrative connect/status/verify/uninstall flows.
- [`crates/volicord-store`](../../../../crates/volicord-store): registry schema initialization and validation, Agent Connection records, Connection Project membership, and Runtime Home access.
- Shared types used by those crates for stored value sets and machine-readable administrative output.

## Related tests and Reference owners

Tests for this design should cover startup validation, project selection,
membership revocation, host setup status, repository-write approval for project
scope, managed marker replacement, rejection of unsupported startup forms, the
exact five Codex hook argv alternatives, unrelated-command negatives, and
load-time checking by an applicable real Codex parser.

Reference owners:

- [Agent Connection Reference](../../reference/agent-connection.md)
- [MCP Transport](../../reference/mcp-transport.md)
- [Administrative CLI](../../reference/admin-cli.md)
- [Runtime Boundaries](../../reference/runtime-boundaries.md)
- [Storage Records](../../reference/storage-records.md)
- [Storage DDL](../../reference/storage-ddl.md)
- [Storage Versioning](../../reference/storage-versioning.md)
- [Security](../../reference/security.md)
