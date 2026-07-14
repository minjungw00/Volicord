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

For reviewed Codex `0.144.4`, the generated descriptor or local marker set is
launch provenance only. The managed process remains pending with zero managed
effects until exact `clientInfo` and strict per-call `_meta` bind the root
session and immutable process-local thread digest. Environment variables, PID,
cwd, process ancestry, timing, and hook-event rendezvous do not substitute for
that binding. See
[Managed-host session/thread binding and per-call turn validation](managed-host-session-turn-binding.md).

The Runtime Home selected by init is part of the managed process binding, not
an incidental platform default. Every generated managed MCP or hook child must
use that Runtime Home. Personal/local MCP entries bind the selected absolute
path directly. Shared repository entries remain clone-portable and do not embed
the path: Codex `.codex/config.toml` allow-forwards it with
`env_vars = ["VOLICORD_HOME"]`, while Claude Code `.mcp.json` uses
`"env": {"VOLICORD_HOME": "${VOLICORD_HOME}"}`. Repository-discovery startup
requires the forwarded value to be present, nonempty, and absolute and rejects
an absent, empty, or relative value before platform-default substitution.
Generated local lifecycle and final-output
wrappers export the selected absolute `VOLICORD_HOME` and invoke the
installation profile's absolute `volicord_command`. They also export a
versioned managed-process binding marker. Hidden managed `_hook` and
`_final-output` execution validates the explicit absolute Runtime Home, that
marker, and that the running executable is the installation profile command;
it does not trust ambient host defaults or a bare PATH-resolved command. Init
normalizes the selected Runtime Home to an absolute path before generating
bindings.

Managed host-hook configuration is derived from one validated command shape
that retains both the rendered command text and the actual execution argv. For
Codex Detective hooks, the generated wrapper is invoked as exactly
`sh -c <generated-hook-script>`. The companion prompt rule therefore matches
the three-token argv prefix `sh`, `-c`, and a closed choice of the exact script
for one of the five required phases: `session-start`, `pre-tool`, `post-tool`,
`prompt-capture`, or `stop`. The rule's positive and negative examples are
validated from that same command shape before configuration is written.

Codex prompt rules must match the argv executed by the host, not the `.codex
hooks` configuration location. A location-based prefix can be rejected before
any hook runs. The exact rule is generated managed host configuration and adds
no public API, storage record, DDL, or storage-profile contract. Managed files
are replaced through the normal ownership and fingerprint checks; unmanaged
files are not adopted. Volicord does not define a stable minimum Codex version,
so checked-in parser fixtures remain insufficient by themselves: release
validation must also load and check the generated rule with an applicable real
Codex parser.

Managed host projections are valid only when they carry the required exact
Runtime Home forwarding or local process binding. Init conditionally regenerates
an exact Volicord-owned projection whose fingerprint matches its stored managed
fingerprint; unmanaged or modified lookalikes remain conflicts. There is no
fallback that silently adopts a platform-default Runtime Home or bare PATH
command. This projection rule changes neither Registry schema nor project/Core
authority records. After a successful host apply, init publishes the refreshed
managed fingerprint and projection metadata to the Agent Connection Registry
record; this requires no DDL, storage-profile, or data migration.

## Consequences

- A user-scoped host configuration can serve multiple explicitly connected Projects without granting all registered Projects.
- Adding or removing a connected Project does not require rewriting a multi-project host MCP command when the command already points at the same `connection_id`; project-bound generated entries may be regenerated when their selected Project binding changes.
- Project selection failures are deterministic: the adapter can report missing or ambiguous project selection and direct the agent to list connected Projects.
- Ordinary project-bound startup can establish a session-watch baseline before tool handling. Managed Codex startup instead remains pending until its first exact call binding; coverage begins there and is explicitly partial. Multi-project startup also remains pending until explicit project selection.
- Host setup status can distinguish configured-but-awaiting-host-action from complete verification.
- Generated host configuration prefers `volicord mcp --stdio --connection <connection_id> --project <project_id>` for project-scoped entries and does not require connection-context or actor-provenance environment variables. Connection-only generated entries remain for flows that intentionally serve multiple connected Projects.
- Shared repository entries stay clone-portable, but the launching host must
  provide the clone's init-selected nonempty, absolute `VOLICORD_HOME`; init cannot change
  a parent host process's environment.
- Local generated wrappers are intentionally untracked process-binding files
  because they pin both the selected Runtime Home and executable path.

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
- Inferring a managed Codex session from `CODEX_THREAD_ID`, PID, cwd, process
  ancestry, timing, or the nearest hook event was rejected because those facts
  cannot bind one exact concurrent root session and thread.
- Embedding an absolute Runtime Home in a shared repository entry was rejected
  because the value is clone-local and would make the entry non-portable.
- Allowing repository discovery to substitute a platform-default Runtime Home
  was rejected because it can silently open a different or incompatible local
  registry instead of the one selected by init.
- Trusting an ambient host value for managed hooks, or invoking a bare
  PATH-resolved `volicord`, was rejected because either can bind the hook child
  to a different Runtime Home or executable than the managed installation.
- Keeping old managed projections through a compatibility fallback was
  rejected because rerunning init can safely refresh owned files without a
  storage migration, while fallback would preserve the ambiguous binding.

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
load-time checking by an applicable real Codex parser. They should also cover
the exact shared-host forwarding forms, absent, empty, and relative discovery
`VOLICORD_HOME` failures before default resolution, and local wrapper binding to
the selected absolute Runtime Home and `volicord_command` despite conflicting
ambient values. Managed Codex tests additionally cover pending zero effects,
exact client and call metadata, immutable root-session/thread binding, later
turn acceptance, and mismatch rejection without durable state.

Reference owners:

- [Agent Connection Reference](../../reference/agent-connection.md)
- [MCP Transport](../../reference/mcp-transport.md)
- [Administrative CLI](../../reference/admin-cli.md)
- [Runtime Boundaries](../../reference/runtime-boundaries.md)
- [Storage Records](../../reference/storage-records.md)
- [Storage DDL](../../reference/storage-ddl.md)
- [Storage Versioning](../../reference/storage-versioning.md)
- [Security](../../reference/security.md)
- [Managed-host session/thread binding and per-call turn validation](managed-host-session-turn-binding.md)
