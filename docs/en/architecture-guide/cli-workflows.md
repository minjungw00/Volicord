# CLI Workflows

This guide explains durable implementation flow for the supported administrative
CLI. Exact command behavior belongs to
[Administrative CLI](../reference/admin-cli.md).

## Ownership Map

| Stage | Implementation responsibility |
|---|---|
| parse and normalize | CLI command DTOs reject unknown or conflicting input |
| resolve context | Runtime Home, canonical Product Repository, project, and Agent Connection selection |
| plan | read-only inspection builds exact proposed file and Store changes |
| validate | managed configuration, Connection, session, storage, and policy checks |
| commit | one owner-defined atomic filesystem/Store boundary |
| render | text or one JSON document from the structured result |

Parsing and rendering must not become alternate Core or Store authorities.

## Codex Setup

`init` accepts only Codex, `record`, and personal/shared scope. The CLI resolves
current canonical inputs, asks the Codex adapter to build a
managed configuration, previews exact managed changes, and applies only after
all preconditions pass. Repair reuses that flow; remove deletes only matching
managed content.

One typed managed MCP launch contract supplies generation, strict parsing,
validation, and fingerprint projection. A personal configuration binds the
selected absolute Runtime Home as static `VOLICORD_HOME`; a shared configuration
forwards only `VOLICORD_HOME` and remains clone-portable. Host-specific TOML
syntax and approval overlays stay in the adapter. Core receives only a current
`ValidatedAgentSession` produced from Store-owned operational records.

## Connection Verification

`init` and the selected-Connection `add`, `status`, `verify`, `mode`, and
`remove` flows build one typed command report whose checks and actions use the
canonical verification types. One optional tagged result owns setup,
mode-transition, or removal facts without creating another status tree. The
JSON and text renderers consume that report, and binary exit handling reads its
typed aggregate status. Rendering does not reconstruct a parallel state tree
or parse its own output. Connection list retains a focused list projection that
does not depend on the command-report state.

`connection status` reads current files and Store observations without running
active probes or writing files, reports, observations, or timestamps.
`connection verify` performs current adapter and managed-configuration
inspection, runs permitted local probes, reads actual managed-host and Guard
observations, and commits at most one report through the Store owner. Executable
path and version are diagnostic probe facts. Authoritative managed runtime and
project sessions are recorded only by managed MCP lifecycle handling; the CLI
self-test records `session_source=cli_preflight` and cannot authorize a
managed-host call.

## Project And Policy Workflows

Project commands resolve canonical registered Git work trees. Policy apply uses
plan, strict validation, and atomic commit. Neither command family infers
authority from a display name or repairs unknown stored values.

## UserAction Workflow

`inbox` reads strict typed pending requests and renders the local user-owned
form. `inbox resolve` submits one stored choice or evidence observation through
`volicord.resolve_user_action`. The MCP adapter can create or resume a request
but cannot call this resolution path.

Guard prompt observations never become a CLI answer. Corrupt stored request or
resolution data fails with a persisted-data error rather than a default form.

## Reconciliation

`changes reconcile` routes through the public Core method. Suppression is
explicitly `Applied` or `Unavailable`; rendering must preserve every remaining
path and the unavailable reason.

## Diagnostics And Output

`doctor`, status, and preflight collect read-only facts and report named next
actions. For connection-report commands, `dry_run` is an operation boolean and
the aggregate remains three-state. `--json` serializes the typed
result once. Human text, logs, and diagnostic metadata are not parsed back into
authority state.

## Boundaries

- CLI depends on Core and Store; Core does not depend on CLI.
- Codex-specific configuration remains in the adapter.
- No command starts a network transport.
- No noninteractive command supplies user judgment.
- Client and host version observations are diagnostics. A changed host version
  renews operational observation; managed-call authorization uses current
  authoritative session ownership and exact bindings.

## Related Routes

- [Source Map](source-map.md)
- [Request Lifecycle](request-lifecycle.md)
- [Agent Connection](../reference/agent-connection.md)
- [MCP Transport](../reference/mcp-transport.md)
- [Testing Strategy](testing-strategy.md)
