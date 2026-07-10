# Multi-Repository Agent Setup

Use this guide when one host-level Agent Connection should serve more than one
explicitly connected `Product Repository`.

This guide is operator workflow. Exact Agent Connection, project-selection, and
transport behavior belongs to [Agent Connection](../reference/agent-connection.md)
and [MCP Transport](../reference/mcp-transport.md).

This is not the default first-run path for one Product Repository. For ordinary
first-run setup, use [Agent Host Setup](agent-host-setup.md) and
`volicord init --host HOST --repo PATH --profile record`. Detective setup has the
supported host-hook and session watcher requirements described there. Use the
lower-level `volicord connection add` commands here only when one host-level or global
host entry must route to more than one explicitly allowed repository.

## Topology

This topology map shows how one host entry can reach multiple explicitly
connected Product Repositories through a host-level Agent Connection. Arrows
show the configured binding and allowed membership relationships; they are not
request execution order and do not imply access to every project in the Runtime
Home.

```mermaid
flowchart LR
  host["Host configuration\nCodex personal or Claude Code global"]
  mcp["volicord mcp --stdio\none Agent Connection"]
  memberships["Connection Projects"]
  a["acme-api\n/path/to/acme-api"]
  b["billing-api\n/path/to/billing-api"]

  host -- "starts one adapter" --> mcp
  mcp -- "uses explicit membership" --> memberships
  memberships -- "allows project" --> a
  memberships -- "allows project" --> b
```

One host entry starts one `volicord mcp --stdio` process for one Agent
Connection. That connection can route only to Product Repositories explicitly
connected to it. Adding one Product Repository does not grant access to every
project registered in the Runtime Home.

This topology fits host-level configuration:

- Codex personal connection: `volicord connection add codex`
- Claude Code global connection: `volicord connection add claude-code --global`

Project-shared and host-local connections remain flows for one Product
Repository.

The paths below are example Product Repository paths for repositories where you
want the agent to work.

## Connect The First Product Repository

Select the first Product Repository explicitly:

```sh
volicord connection add codex --repo /path/to/acme-api
volicord connection status codex --repo /path/to/acme-api
```

For Claude Code global configuration:

```sh
volicord connection add claude-code --global --repo /path/to/acme-api
volicord connection status claude-code --global --repo /path/to/acme-api
```

The command detects the Git repository root, registers or reuses the repository
project, derives the visible project name from the repository directory, and
stores internal registry identities in the Runtime Home.

## Add Another Product Repository

Run the same host and intent for the second Product Repository:

```sh
volicord connection add codex --repo /path/to/billing-api
volicord connection status codex --repo /path/to/billing-api
```

The same rule applies when the current working directory is already inside the
Product Repository; using `--repo` keeps the membership target unambiguous.

```sh
volicord connection add codex
volicord connection status codex
```

For the same host-level target, Volicord reuses the matching Agent Connection
and adds the selected Product Repository to Connection Projects. The operator
does not need to handle the internal connection identity.

## Inspect The Connection

```sh
volicord connection list
volicord connection verify codex
volicord connection status codex --repo /path/to/acme-api
volicord connection status codex --repo /path/to/billing-api
```

If verification reports `action_required`, complete the named host-owned trust,
approval, reload, restart, or installation-profile repair action and rerun
verification. For symptom-specific recovery, use
[Agent Host Troubleshooting](agent-host-troubleshooting.md).

## What The Agent Should Do

When a user asks which Product Repositories are available, the agent calls
`volicord.list_projects`. The result lists only projects connected to the bound
Agent Connection.

When more than one project is available, the agent uses the exact
`project_selector` returned for the intended repository. It must not invent a
selector from a directory name, display name, current working directory, MCP
root, host label, or memory. If a call is rejected because project selection is
ambiguous, the agent lists projects, chooses the intended repository, and
retries with the returned selector.

Exact MCP argument and omission rules belong to
[MCP Transport](../reference/mcp-transport.md).

## Remove One Product Repository

From the Product Repository to remove:

```sh
cd /path/to/billing-api
volicord connection remove codex --dry-run
volicord connection remove codex
```

Or select the Product Repository explicitly:

```sh
volicord connection remove codex --repo /path/to/billing-api --dry-run
volicord connection remove codex --repo /path/to/billing-api
```

Removing one Product Repository removes that Product Repository's Connection
Projects membership. It does not delete the `Product Repository`, project
registration, project state, Volicord Task, Evidence, or Run records, Evidence
attachment storage, or unrelated host configuration. If other connected
Product Repositories remain, the host entry remains. If none remain, Volicord
removes the matching managed host configuration when ownership and safety
checks permit it.

## Boundaries

- Agent Connections access only explicitly connected Product Repositories.
- Multiple connected Product Repositories require explicit `project_selector`
  in public MCP tool calls unless the call is `volicord.list_projects`.
- A `Product Repository` is a product-file boundary and may contain selected
  shared host configuration, but it is not Volicord authority.
- A Write Ticket records that one proposed product-file write was checked
  against the current work boundary. It is not OS permission, code review
  approval, final acceptance, or proof that a write occurred.
- Security limits and non-guarantees are owned by
  [Security](../reference/security.md).
