# Quickstart

This tutorial gets from a fresh source checkout to one working Agent Connection.
It assumes you are connecting a local host to a normal Git product repository.

Exact command contracts belong to
[Administrative CLI Reference](../reference/admin-cli.md). Agent Connection
meaning belongs to [Agent Connection Reference](../reference/agent-connection.md).

## Fast Path

```sh
cargo build --workspace --bins
export PATH="$PWD/target/debug:$PATH"
volicord setup
cd /path/to/your-product-repo
volicord connect codex
```

The `export PATH=...` line affects only the current terminal session and lets
that shell find the freshly built `volicord` and `volicord-mcp` commands.
`/path/to/your-product-repo` is a placeholder for the Git product repository
you want the host to work on. Volicord detects that repository from the current
directory and uses the normal CLI defaults for a first host connection. Exact
project naming, connection defaults, and internal identity behavior belong to
[Administrative CLI Reference](../reference/admin-cli.md).

## Confirm The Setup

```sh
volicord doctor
volicord project current
volicord connection status codex
volicord connection verify codex
```

Completion state: the connection is ready when status or verification reports
`complete`. If it reports `action_required`, complete the named host-owned or
local repair action, then rerun verification. Exact result-state meaning belongs
to [Administrative CLI Reference](../reference/admin-cli.md#agent-connection-result-states).

## Choose A Host Intent

Start with the default command when you are connecting the current user's local
host setup. Add `--shared` only when the selected repository should carry the
project-shared integration file, and use `--global` only for a host path that
supports user-wide configuration. Exact intent semantics belong to
[Administrative CLI Reference](../reference/admin-cli.md#connection-intents-and-hosts);
host availability requirements belong to
[System Requirements](../reference/system-requirements.md#host-configuration-requirements).

Use `--read-only` only when the host should expose read-oriented behavior:

```sh
volicord connect codex --read-only
```

Use `--repo PATH` only when the current directory is not the repository you want
to connect:

```sh
volicord connect codex --repo /path/to/your-product-repo
```

## Inspect Or Change The Connection

```sh
volicord connections
volicord connection status codex
volicord connection verify codex
volicord connection mode codex read-only
volicord connection mode codex workflow
```

Removing the selected repository from the connection uses the same host and
intent selection:

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

The export uses the detected repository and the installation profile. Exact
output defaults belong to
[Administrative CLI Reference](../reference/admin-cli.md#generic-mcp-config-export).
The exported file is user-managed after export; Volicord does not claim that an
arbitrary external host loaded or approved it.

## Record User Judgment

Agent Connections may request or show focused judgment needs, but
authority-bearing user answers go through the local `User Channel`:

```sh
volicord user status
volicord user judgments
volicord user judgment show 1
volicord user judgment answer 1 1
```

Use `--repo PATH` only when you need to answer for a repository other than the
current one. Use `--task ID` when the active task is not the intended task.

## Next Steps

| Need | Read |
|---|---|
| Host setup details | [Agent Host Setup](../guides/agent-host-setup.md) |
| Troubleshooting `action_required` or `failed` | [Agent Host Troubleshooting](../guides/agent-host-troubleshooting.md) |
| User workflow and judgment boundaries | [User Guide](../guides/user-workflow.md) |
| Agent workflow boundaries | [Agent Guide](../guides/agent-workflow.md) |
