# Quickstart

This tutorial gets from a fresh source checkout to one working Agent Connection.
It assumes you are connecting a local host to a normal Git product repository.

Exact command contracts belong to
[Administrative CLI Reference](../reference/admin-cli.md). Agent Connection
meaning belongs to [Agent Connection Reference](../reference/agent-connection.md).

## Fast Path

```sh
cargo build --workspace --bins
./target/debug/volicord setup --link-bin ~/.local/bin
cd /work/acme-api
volicord connect codex
```

The repository is detected from the current directory. The project name comes
from the repository directory, so `/work/acme-api` becomes the visible project
name `acme-api` unless that name needs to be made unique. The default connection
intent is `personal`, the default mode is `workflow`, and internal identities
are managed by Volicord.

## Confirm The Setup

```sh
volicord doctor
volicord project current
volicord connection status codex
volicord connection verify codex
```

Completion state: the connection is ready when status or verification reports
`complete`. If it reports `action_required`, complete the named host-owned
trust, approval, reload, restart, or setup repair action, then rerun
verification.

## Choose A Host Intent

Use the shortest command that matches where the host configuration should live:

| Intent | Command shape | Use when |
|---|---|---|
| `personal` | `volicord connect codex` or `volicord connect claude-code` | The connection is for the current user's local host setup. |
| `shared` | `volicord connect codex --shared` or `volicord connect claude-code --shared` | The repository should carry an explicit project-shared host integration file. |
| `global` | `volicord connect claude-code --global` | The selected host supports user-wide configuration while project access remains constrained by Volicord records. |

Use `--read-only` only when the host should expose read-oriented behavior
instead of workflow tools:

```sh
volicord connect codex --read-only
```

Use `--repo PATH` only when the current directory is not the repository you want
to connect:

```sh
volicord connect codex --repo /work/acme-api
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

The export uses the detected repository and the setup profile. The exported file
is user-managed after export; Volicord does not claim that an arbitrary external
host loaded or approved it.

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
