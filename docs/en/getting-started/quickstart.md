# Quickstart

This tutorial starts after [Installation](installation.md) has made
`volicord` available on `PATH` and gets to one working Agent Connection. It
assumes you are connecting a local host to a normal Git repository used as the
Product Repository.

Exact command contracts belong to
[Administrative CLI Reference](../reference/admin-cli.md). Agent Connection
meaning belongs to [Agent Connection Reference](../reference/agent-connection.md).

## Fast Path

```sh
volicord init --host codex --repo /path/to/your-product-repo --mode mcp-only
```

`/path/to/your-product-repo` is an example path for the Product Repository where
you want the agent to work. `volicord init` is the primary first-run repository
setup and host-connection command. It creates or reuses the Runtime Home and
installation profile when needed, registers the selected repository, installs
project-scoped MCP configuration for the selected host, writes Volicord-managed
guidance and policy metadata, and records guard installation status.
Generated host configuration starts the single public executable as
`volicord mcp --stdio`.

This fast path uses `--mode mcp-only`, which does not require host lifecycle
hook installation and has no pre-tool blocking hook. If a session watcher
becomes active for the selected session, guard health may report
`detective_watch` and create unrecorded-change findings from Product Repository
metadata changes, but the watcher does not prevent writes or identify who made
the change. Default `guarded` init requires verified support for all
required host hook phases; if support is missing, use `--allow-degraded` only
when you explicitly want degraded guard files and missing-hook diagnostics.
Managed init additionally requires a verified managed distribution contract and
fails with `MANAGED_MODE_UNSUPPORTED` when the selected host has none. Exact
project naming, guard-mode behavior, connection defaults, and internal identity
behavior belong to
[Administrative CLI Reference](../reference/admin-cli.md).

## Confirm The Setup

```sh
volicord doctor
volicord project current
volicord connection status codex --repo /path/to/your-product-repo
volicord connection verify codex --repo /path/to/your-product-repo
```

Completion state: the connection is ready when status or verification reports
`complete`. If it reports `action_required`, complete the named host-owned or
local repair action, then rerun verification. Exact result-state meaning belongs
to [Administrative CLI Reference](../reference/admin-cli.md#agent-connection-result-states).

## Choose A Host Intent

Use the lower-level `volicord connect` command only when you need a personal,
global, or read-only variant directly. Add `--shared` only when using
`volicord connect` to manage the project-shared integration file without the
ordinary `init` flow, and use `--global` only for a host path that supports
user-wide configuration. Exact intent semantics belong to
[Administrative CLI Reference](../reference/admin-cli.md#connection-intents-and-hosts);
host availability requirements belong to
[System Requirements](../reference/system-requirements.md#host-configuration-requirements).

Use `--read-only` only when the host should expose read-oriented behavior:

```sh
volicord connect codex --read-only
```

For lower-level connection management, use `--repo PATH` when the current
directory is not the target Product Repository:

```sh
volicord connect codex --repo /path/to/your-product-repo
```

`volicord connect` is still the lower-level connection-management command for
personal, shared, global, and read-only variants. For the ordinary first-run
path, prefer `volicord init --host HOST --repo PATH --mode mcp-only`.

## Inspect Or Change The Connection

```sh
volicord connections
volicord connection status codex --repo /path/to/your-product-repo
volicord connection verify codex --repo /path/to/your-product-repo
volicord connection mode codex read-only
volicord connection mode codex workflow
```

Removing the selected Product Repository from the connection uses the same host
and intent selection:

```sh
volicord connection remove codex --dry-run
volicord connection remove codex
```

`--dry-run` reports the plan without persistent changes.

## Export Generic MCP Config

For an MCP host that Volicord does not manage directly, export a host-neutral
config:

```sh
volicord export mcp-config --output /tmp/volicord.mcp.json
```

The export uses the detected Product Repository and the installation profile.
Exact output defaults belong to
[Administrative CLI Reference](../reference/admin-cli.md#generic-mcp-config-export).
The exported file is user-managed after export; Volicord does not claim that an
arbitrary external host loaded or approved it.

## Record User Judgment

Agent Connections may request or show focused judgment needs, but
authority-bearing user answers go through the local `User Channel`:

When the host and client support it, the MCP adapter may use MCP elicitation
for the pending judgment. When guard health reports prompt capture as
`configured`, `observed`, or `active`, the chat path is a strict prompt command
such as `Volicord: answer J-3 1 #AB7K`. When elicitation and prompt capture are
unavailable and the adapter can safely expose the fallback, Volicord may return
a loopback local web consent URL with a short-lived one-time token. Use the
terminal commands below as the stable recovery path when elicitation, prompt
capture, and local web consent are unavailable or need inspection.

```sh
volicord user status
volicord user judgments
volicord user judgment show 1
volicord user judgment answer 1 1
```

Use `--repo PATH` only when you need to answer for a Product Repository other
than the current one. Use `--task ID` when the active task is not the intended
task.

## Next Steps

| Need | Read |
|---|---|
| Host setup details | [Agent Host Setup](../guides/agent-host-setup.md) |
| Troubleshooting `action_required` or `failed` | [Agent Host Troubleshooting](../guides/agent-host-troubleshooting.md) |
| User workflow and judgment boundaries | [User Guide](../guides/user-workflow.md) |
| Agent workflow boundaries | [Agent Guide](../guides/agent-workflow.md) |
