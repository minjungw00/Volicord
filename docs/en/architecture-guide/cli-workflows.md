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

A shared configuration forwards `VOLICORD_HOME` and starts managed stdio. A
personal configuration remains user-owned. Host-specific file syntax stays in
the adapter. Core receives only a current `ValidatedAgentSession` produced from
Store-owned operational records.

## Connection Verification

`connection verify` performs current adapter and managed-configuration
inspection and returns the canonical three-state report. It does not hash the
host executable, consult release-certification catalogs, issue an authorization
receipt, or create an agent session. Authoritative managed runtime and project
sessions are recorded only by managed MCP lifecycle handling.

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
actions. `--json` serializes the structured result once. Human text, logs, and
diagnostic metadata are not parsed back into authority state.

## Boundaries

- CLI depends on Core and Store; Core does not depend on CLI.
- Codex-specific configuration remains in the adapter.
- No command starts a network transport.
- No noninteractive command supplies user judgment.
- Client and host version observations are diagnostics and never authorization
  credentials. Release claims remain a separate exact six-cell evidence flow.

## Related Routes

- [Source Map](source-map.md)
- [Request Lifecycle](request-lifecycle.md)
- [Agent Connection](../reference/agent-connection.md)
- [MCP Transport](../reference/mcp-transport.md)
- [Testing Strategy](testing-strategy.md)
