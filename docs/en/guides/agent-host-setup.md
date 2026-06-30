# Agent Host Setup

Use this guide to connect Codex, Claude Code, or a generic MCP host to
Volicord. The ordinary path starts with the host, Product Repository, and
connection intent; Volicord manages the internal host and registry values.

Exact CLI behavior belongs to
[Administrative CLI Reference](../reference/admin-cli.md). Agent Connection
meaning belongs to [Agent Connection Reference](../reference/agent-connection.md),
and runtime/file boundaries belong to
[Runtime Boundaries](../reference/runtime-boundaries.md).

## Setup Sequence

Install `volicord` first with [Installation](../getting-started/installation.md),
then run the host setup sequence:

```sh
volicord setup
volicord doctor
cd /path/to/your-product-repo
volicord connect codex
volicord connection status codex
```

`/path/to/your-product-repo` is an example path for the Product Repository where
you want the agent to work. The connection command detects the
repository root from the current directory, registers or reuses that repository
project, derives the visible project name from the repository directory, and
stores internal registry identities in the selected `Volicord Runtime Home`.

Use `--repo PATH` only when the process current directory is not the target
Product Repository:

```sh
volicord connect codex --repo /path/to/your-product-repo
```

## Connection Intents

Connection intent describes where the host configuration belongs:

| Intent | Command shape | Host support |
|---|---|---|
| `personal` | `volicord connect codex` or `volicord connect claude-code` | Local setup for the current user. |
| `shared` | `volicord connect codex --shared` or `volicord connect claude-code --shared` | Project-shared configuration stored through an explicit integration file when the host supports it. |
| `global` | `volicord connect claude-code --global` | User-wide host configuration for hosts that support it. |

`--shared` and `--global` are mutually exclusive. When neither is present,
Volicord uses `personal`.

## Workflow And Read-Only Mode

The default mode is `workflow`. Use `--read-only` for a connection that should
expose read-oriented behavior instead of workflow tools:

```sh
volicord connect codex --read-only
```

Change an existing connection mode with:

```sh
volicord connection mode codex read-only
volicord connection mode codex workflow
```

The host may need a reload or restart after a mode change.

## Dry Run Before Applying

Dry run reports the plan without persistent changes:

```sh
volicord connect codex --dry-run
volicord connect claude-code --shared --dry-run
volicord connection remove codex --dry-run
```

Use dry run before changing shared host configuration or before removing a
connection whose host target you want to inspect first.

## Inspect And Verify

```sh
volicord connections
volicord connection status codex
volicord connection verify codex
```

For a shared or global connection, include the same intent flag used to select
it:

```sh
volicord connection status codex --shared
volicord connection verify claude-code --global
```

Result states:

| State | Meaning in setup guidance |
|---|---|
| `complete` | Volicord-side state, managed host configuration, observable MCP startup, initialization, and expected tool exposure are ready. |
| `action_required` | Volicord-side state exists, but a named user-controlled host action remains. |
| `failed` | A required local prerequisite, host configuration step, or verification step did not succeed. |
| `dry_run` | The command reported planned actions without persistent changes. |

## Generic MCP Config Export

For an MCP host that Volicord does not manage directly:

```sh
cd /path/to/your-product-repo
volicord export mcp-config --output /tmp/volicord.mcp.json
```

The export uses the detected Product Repository and the installation profile. Add
`--read-only` when the exported config should bind a read-only connection. The
exported file remains user-managed after export.

## User Channel Boundary

Agent Connections can request or display focused judgment needs. They do not
record authority-bearing user answers. Use the local `User Channel` commands
when a Core-generated option must become the user's recorded judgment:

```sh
volicord user judgments
volicord user judgment show 1
volicord user judgment answer 1 1
```

## Removal

Remove the selected Product Repository from a connection:

```sh
volicord connection remove codex --dry-run
volicord connection remove codex
```

Removal deletes only matching managed host configuration when ownership and
safety checks permit it. It does not delete the `Product Repository`, Runtime
Home, project registration, project state, Core records, artifact storage, or
unrelated host configuration.

## Troubleshooting Routes

| Symptom | Next document |
|---|---|
| Installation profile, executable, or Product Repository detection is not ready. | [Installation](../getting-started/installation.md) |
| Connection reports `action_required` or `failed`. | [Agent Host Troubleshooting](agent-host-troubleshooting.md) |
| Exact command behavior is unclear. | [Administrative CLI Reference](../reference/admin-cli.md) |
| Runtime Home and Product Repository boundaries matter. | [Runtime Boundaries](../reference/runtime-boundaries.md) |
