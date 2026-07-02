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
volicord init --host codex --repo /path/to/your-product-repo --profile record
```

`/path/to/your-product-repo` is an example path for the Product Repository where
you want the agent to work. `volicord init` is the primary first-run repository
setup and host-connection command. It creates or reuses the Runtime Home and
installation profile when needed, registers the selected repository, installs
project-scoped MCP configuration for the selected host, writes Volicord-managed
guidance and policy metadata, and records integration status.
Generated host configuration starts the single public executable as
project-bound `volicord mcp --stdio`.

This fast path uses the Record profile (`--profile record`), which does not
require host lifecycle hook installation or a session watcher. The Detective
profile (`--profile observe`) requires verified support for all required host
hook phases and session watcher observation. If those prerequisites are
unavailable, use `--profile record` or prepare a supported host, platform, and
repository configuration for observe before rerunning init. The Detective
profile can return cooperative host decision signals and detect unrecorded
changes after watcher coverage starts, but it does not provide OS enforcement,
actor proof, network isolation, or a sandbox. On native Windows, use this
Record profile fast path because observe is not supported until Windows host hooks and
watcher behavior are implemented and tested. Exact project naming, profile
behavior, connection defaults, and internal identity behavior belong to
[Administrative CLI Reference](../reference/admin-cli.md).

If you choose observe setup instead of this `record` fast path, generated
hook commands are designed to work when the host session starts from a
repository subdirectory. Status, verification, and doctor diagnostics report
`hook_path_safety`; a value such as `relative_path_unsafe`, `wrapper_missing`,
or `wrapper_not_executable` means observe host hooks are not active until the
generated hook commands or wrappers are repaired.

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
Observe hook path repair guidance belongs to
[Agent Host Troubleshooting](../guides/agent-host-troubleshooting.md#guard-hook-path-or-wrapper-is-unsafe).

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
path, prefer `volicord init --host HOST --repo PATH --profile record`.

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

When the host and client support it, the MCP adapter may use a host prompt for
the pending judgment. When observe status reports chat command capture as
`configured`, `observed`, or `active`, the chat path is a strict prompt command
such as `Volicord: answer J-3 1 #AB7K`. When host prompt input and chat command
capture are unavailable and the adapter can safely expose the fallback,
Volicord may return a loopback local consent URL with a short-lived one-time
token. Use the terminal commands below as the stable CLI inbox path when the
other User Channel input methods are unavailable or need inspection.

```sh
volicord inbox
volicord inbox answer JUDGMENT_ID --choice CHOICE_ID
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
