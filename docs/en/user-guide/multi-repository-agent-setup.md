# Multi-Repository Agent Setup

Use this guide when one personal Codex connection is explicitly allowed to
serve more than one registered Product Repository. For a single repository or
a project-shared connection, use [Agent Host Setup](agent-host-setup.md).

## Topology

One stored Agent Connection may have explicit Connection Projects memberships.
The stdio process is bound to that connection. A request selects only a project
that belongs to the allowlist; cwd, directory scanning, and repository-name
guessing do not add membership.

## Connect Repositories

Register the first repository:

```sh
volicord connection add codex --repo /path/to/acme-api
volicord connection status codex --repo /path/to/acme-api
```

Add another explicit membership with the same personal intent:

```sh
volicord connection add codex --repo /path/to/billing-api
volicord connection status codex --repo /path/to/billing-api
```

Inspect the resulting connection and memberships:

```sh
volicord connection list
volicord connection verify codex
```

## Agent Selection

Start the managed `volicord mcp --stdio` process with its exact connection
binding. The agent should call `volicord.list_projects`, select one allowed
`project_id`, and pass that identity on project-scoped calls. Ambiguous or
unlisted selection must fail closed.

Each Product Repository retains its own Task, scope, Write Tickets, runs,
evidence, continuity, UserAction requests, and close state. Membership does not
merge project authority.

## Remove One Membership

Preview and remove the named repository membership:

```sh
volicord connection remove codex --repo /path/to/billing-api --dry-run
volicord connection remove codex --repo /path/to/billing-api
```

Recheck the remaining membership before restarting Codex. The command removes
the selected membership's Registry binding and Guard Installation but retains
the Agent Connection, its runtime sessions, shared host configuration, and the
other membership's integration rows. The Agent Connection and matching host
configuration are removed only with the last membership. Other Product
Repositories and their project-local history remain.

## Boundaries

- Only explicitly stored memberships are selectable.
- The first release uses the Codex `record` profile over managed stdio.
- A shared connection remains scoped to one Product Repository.
- UserAction resolution remains a local CLI operation for the selected
  repository.
- Exact routing belongs to [Agent Connection](../reference/agent-connection.md)
  and [MCP Transport](../reference/mcp-transport.md).
