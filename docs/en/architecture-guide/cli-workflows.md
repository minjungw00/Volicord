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
| validate | canonical binding, external contract, storage, and policy checks |
| commit | one owner-defined atomic filesystem/Store boundary |
| render | text or one JSON document from the structured result |

Parsing and rendering must not become alternate Core or Store authorities.

## Codex Setup

`init` accepts only Codex, `record`, and personal/shared scope. The CLI resolves
current canonical inputs, asks the Codex adapter to build a
`ManagedHostBinding`, previews exact managed changes, and applies only after all
preconditions pass. Repair reuses that flow; remove deletes only matching
managed content.

A shared configuration forwards `VOLICORD_HOME` and starts managed stdio. A
personal configuration remains user-owned. Host-specific file syntax and
artifact inspection stay in the adapter; Core receives only canonical types and
a typed verification receipt.

## Connection Verification

`connection verify` performs current adapter inspection, matches the exact
artifact and platform against the embedded support catalog, validates the
complete binding, and issues
a `HostVerificationReceipt` only on success. The CLI then asks Core to validate
the receipt against current stored state. Diagnostic status never promotes
missing evidence.

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
- Runtime lookup uses only the embedded support catalog. Release claims require
  the separate exact six-cell evidence manifest to match that catalog.

## Related Routes

- [Source Map](source-map.md)
- [Request Lifecycle](request-lifecycle.md)
- [Agent Connection](../reference/agent-connection.md)
- [MCP Transport](../reference/mcp-transport.md)
- [Testing Strategy](testing-strategy.md)
