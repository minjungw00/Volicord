# Quickstart

This tutorial is the shortest supported first setup path for a real local agent
host. It starts after [Installation](installation.md), uses one `Product
Repository`, and gives you a clear choice between a personal Codex user-scope
entry and a project-scoped Claude Code `.mcp.json` entry.

For complete host setup options, dry-run previews, repository guidance, removal,
and troubleshooting, see [Agent Host Setup](../guides/agent-host-setup.md).

## Audience, Goal, And Completion

Audience: first-time users or operators who have already verified local
`harness` and `harness-mcp` executables and want one agent host path to work
before expanding the setup.

Goal: install one supported host configuration, recognize whether the first
result is `complete` or `action_required`, and run an independent verification
command for the chosen path.

Completion state: the chosen path is complete when `harness agent verify` for
that `integration_id` reports `status: complete` and the selected Host
Installation has `final_status: complete`. If the command reports
`action_required`, complete the named host-owned trust, approval, reload, or
restart action and rerun verification.

## Starting State And Example Values

Before running these commands:

- Complete [Installation](installation.md) in a POSIX-style shell.
- Keep `HARNESS_BIN` set to the verified absolute directory containing both
  executables.
- Choose a `Product Repository` that is not the `Harness Runtime Home` and is
  not inside or above it.
- Replace every example path and ID below with your real value.

The examples use:

| Example value | Meaning |
|---|---|
| `HARNESS_BIN="/absolute/path/to/selected/bin"` | Selected absolute directory containing both `harness` and `harness-mcp`. |
| `"$HARNESS_BIN/harness"` | `harness` administrative CLI invocation. |
| `"$HARNESS_BIN/harness-mcp"` | Absolute `harness-mcp` command used for user/local-scope host configuration. |
| `/Users/alex/.harness` | `Harness Runtime Home`. |
| `/work/acme-api` | Product Repository A. |
| `acme-api` | Project ID for Product Repository A. |
| `int-codex-team`, `int-claude-acme` | Example `integration_id` values. |
| `harness-int-codex-team`, `harness-int-claude-acme` | Stable host MCP server names derived from `integration_id` when `--server-name` is omitted. |

`HARNESS_BIN` is a tutorial shell variable. Harness does not read it as
configuration. `HARNESS_HOME` is different: it is a real Runtime Home selection
input for the administrative command and for later `harness-mcp` process
startup when the default Runtime Home is not the intended one.

## Choose One Host Path

| Path | Choose when | Consequence |
|---|---|---|
| Path A: Codex `user` scope | One personal Codex MCP entry should serve this repository now and may later serve more explicitly allowed repositories. | Host configuration lives in the Codex user config and stores an absolute `harness-mcp` command path plus `HARNESS_HOME`. |
| Path B: Claude Code `project` scope | Product Repository A should carry a team-shared Claude Code `.mcp.json` entry. | The project file uses portable `harness-mcp`, omits personal `HARNESS_HOME`, requires `--allow-repository-write`, and may remain `action_required` until Claude Code approval is complete. |

If you need another host or scope, use [Agent Host Setup](../guides/agent-host-setup.md).
If one user-scope integration should serve multiple repositories, complete Path A
for the first repository and then follow
[Multi-Repository Agent Setup](../guides/multi-repository-agent-setup.md). Do not
repeat this quickstart mechanically for each repository.

## Path A: Codex User-Scope Setup

Use this when one personal Codex MCP entry should serve one or more explicitly
allowed `Product Repository` registrations.

Prerequisites:

- Codex can read its user `config.toml` through `CODEX_HOME` or `HOME`.
- The `codex` executable is available on the administrative command `PATH` for
  the compatibility check.
- `HARNESS_BIN` names the verified absolute executable directory from
  Installation.
- Product Repository A is at `/work/acme-api`.
- `/Users/alex/.harness` is separate from `/work/acme-api`.

Command:

```sh
"$HARNESS_BIN/harness" agent install \
  --host codex \
  --scope user \
  --integration-id int-codex-team \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --default-project-id acme-api \
  --runtime-home /Users/alex/.harness \
  --mcp-command "$HARNESS_BIN/harness-mcp"
```

Locations that may change:

| Location | What may change |
|---|---|
| `/Users/alex/.harness` | Runtime Home registry, integration, project, surface, Host Installation, and project state records. |
| Codex user config, normally `~/.codex/config.toml` or `CODEX_HOME/config.toml` | A `[mcp_servers.harness-int-codex-team]` table. |
| `/work/acme-api` | No file change unless repository guidance is selected separately. |

Because `--server-name` is omitted, the CLI derives a stable host MCP server
name from `integration_id`. Use `--server-name` only when you need to pin a
specific host configuration key.

Expected first result:

```text
status: complete
integration_id: int-codex-team
host_kind: codex
host_scope: user
server_name: harness-int-codex-team
verification: complete
verification_detail: MCP initialize and tools/list succeeded
```

The generated Codex entry has this shape:

```toml
[mcp_servers.harness-int-codex-team]
command = "/absolute/path/to/selected/bin/harness-mcp"
args = ["--integration", "int-codex-team"]

[mcp_servers.harness-int-codex-team.env]
HARNESS_HOME = "/Users/alex/.harness"
```

The actual `command` value is the shell-expanded absolute path selected through
`HARNESS_BIN`; generated TOML does not contain `HARNESS_BIN`.

Independent completion check:

```sh
"$HARNESS_BIN/harness" agent verify \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness
```

Path A is complete when verification reports `status: complete`. If verification
reports `action_required`, read the named action. A common Codex user-scope cause
is that `codex` is missing from the administrative command `PATH` or cannot run
`codex --version`.

## Path B: Claude Code Project-Scope Setup

Use this when Product Repository A should carry a team-shared Claude Code
`.mcp.json` entry.

Prerequisites:

- `HARNESS_BIN` names the verified absolute executable directory from
  Installation.
- `harness-mcp` will be available on the `PATH` that Claude Code uses when it
  starts MCP servers.
- If Claude Code would not otherwise resolve `/Users/alex/.harness` as its
  Runtime Home, the Claude Code launch environment must provide
  `HARNESS_HOME=/Users/alex/.harness`.
- Product Repository A is at `/work/acme-api`.
- `/Users/alex/.harness` is separate from `/work/acme-api`.
- You intentionally allow the administrative command to write
  `/work/acme-api/.mcp.json`.

Optional dry-run before writing the project file:

```sh
HARNESS_HOME=/Users/alex/.harness \
PATH="$HARNESS_BIN:$PATH" \
"$HARNESS_BIN/harness" agent install \
  --host claude-code \
  --scope project \
  --integration-id int-claude-acme \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --mcp-command harness-mcp \
  --dry-run \
  --output json
```

Apply the setup:

```sh
HARNESS_HOME=/Users/alex/.harness \
PATH="$HARNESS_BIN:$PATH" \
"$HARNESS_BIN/harness" agent install \
  --host claude-code \
  --scope project \
  --integration-id int-claude-acme \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --mcp-command harness-mcp \
  --allow-repository-write
```

Locations that may change:

| Location | What may change |
|---|---|
| `/Users/alex/.harness` | Runtime Home registry, integration, project, surface, Host Installation, and project state records. |
| `/work/acme-api/.mcp.json` | A Claude Code project-scoped MCP server entry. |
| Claude Code user approval state | Only after the user approves the project MCP server in Claude Code. |

Expected first result before host approval:

```text
status: action_required
integration_id: int-claude-acme
host_kind: claude_code
host_scope: project
server_name: harness-int-claude-acme
verification: action_required
```

The output should name a host-owned follow-up such as Claude Code project MCP
approval. `action_required` is a successful administrative result, not command
failure.

The generated `.mcp.json` entry has this shape:

```json
{
  "mcpServers": {
    "harness-int-claude-acme": {
      "command": "harness-mcp",
      "args": ["--integration", "int-claude-acme"]
    }
  }
}
```

The generated `.mcp.json` intentionally omits `HARNESS_HOME` and keeps the
portable `harness-mcp` command. The `HARNESS_HOME` and `PATH` assignments on the
install command apply only to that administrative invocation. When Claude Code
later starts the server, Claude Code's own launch environment must be able to
find `harness-mcp` on `PATH` and must provide `HARNESS_HOME` if its default
Runtime Home would be different.

Complete the host-owned action: start or restart Claude Code in
`/work/acme-api` from the intended environment, review the project MCP server,
and approve it in Claude Code.

Independent completion check after approval:

```sh
HARNESS_HOME=/Users/alex/.harness \
PATH="$HARNESS_BIN:$PATH" \
"$HARNESS_BIN/harness" agent verify \
  --integration-id int-claude-acme
```

Path B is complete when verification reports `status: complete` and the
installation verification for `harness-int-claude-acme` reports
`final_status: complete`. If verification still reports `action_required`, the
host-owned approval, reload/restart, or launch environment is still incomplete.

## Inventory, Verification, And Active Host Loading

Use `harness agent status` to inspect registry and Host Installation inventory:

```sh
"$HARNESS_BIN/harness" agent status \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness
```

`harness agent status` is not proof that Codex or Claude Code loaded the MCP
server. Use `harness agent verify` for the administrative verification gates,
and use the host's own UI, MCP list, or approval flow to confirm host-owned load
state when the host exposes it.

A successful `harness-mcp --check --integration <integration_id>` is startup
validation for the MCP process only. It is not by itself complete host
integration.

## Setup State Meanings

| State | What to do next |
|---|---|
| `complete` | The administrative setup, host-owned gates, MCP initialization, and tool discovery succeeded. Use the host and confirm the server appears where the host exposes MCP servers or tools. |
| `action_required` | The command succeeded, but a named host-owned action remains. Complete that action, then run `harness agent verify`. |
| `partial_failure` | Some durable action may have succeeded before a later step failed. Read `effects` and `residual_effects`, fix only the named issue, then rerun the same command or verify. |
| `failed` | The requested setup or verification did not establish usable durable integration state or host configuration. Fix the reported error before retrying. |

## Failure Routing

| Symptom | Safe next action | Route |
|---|---|---|
| `harness`, `harness-mcp`, or `HARNESS_BIN` fails before setup. | Return to Installation and rerun the executable checks from the same shell. | [Installation](installation.md#verify-the-selected-directory) |
| Setup or verification cannot resolve `harness-mcp`. | For user/local scope, use a valid absolute `--mcp-command`; for project scope, keep `harness-mcp` portable and fix the host `PATH`. | [Agent Host Troubleshooting](../guides/agent-host-troubleshooting.md#missing-harness-mcp) |
| A project-scoped command refuses to write `.mcp.json` or `.codex/config.toml`. | Rerun after deciding that the repository write is intended and include `--allow-repository-write`. | [Administrative CLI](../reference/admin-cli.md#noninteractive-approval-behavior) |
| Result is `action_required`. | Complete only the named host-owned trust, approval, reload, restart, or executable-availability action, then rerun `harness agent verify`. | [Agent Host Troubleshooting](../guides/agent-host-troubleshooting.md#status-action_required) |
| Result is `partial_failure` or `failed`. | Read the reported `effects`, `residual_effects`, warnings, and verification details. Do not delete the Runtime Home, Product Repository, or unrelated host entries as a first response. | [Agent Host Troubleshooting](../guides/agent-host-troubleshooting.md#status-partial_failure) and [failed](../guides/agent-host-troubleshooting.md#status-failed) |
| One integration should serve several repositories. | Use a user-scope integration and add explicit project memberships; do not add one host entry per repository. | [Multi-Repository Agent Setup](../guides/multi-repository-agent-setup.md) |

## Continue

- Full host setup, dry-run preview, repository guidance, generic export, status,
  verification, and safe removal: [Agent Host Setup](../guides/agent-host-setup.md)
- One user-scope integration serving multiple repositories:
  [Multi-Repository Agent Setup](../guides/multi-repository-agent-setup.md)
- Agent workflow: [Agent Guide](../guides/agent-workflow.md)
- Exact `harness` agent command behavior:
  [Administrative CLI](../reference/admin-cli.md#volicord-agent-install)
- Exact project selection and guidance boundaries:
  [Agent Integration](../reference/agent-integration.md)
- Exact `harness-mcp` process behavior:
  [MCP Transport](../reference/mcp-transport.md)
- Exact runtime location boundaries:
  [Runtime Boundaries](../reference/runtime-boundaries.md)
