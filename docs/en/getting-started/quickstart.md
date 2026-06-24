# Quickstart

This tutorial is the shortest supported first setup path for a real local agent
host. It starts after [Installation](installation.md), uses one `Product
Repository`, and gives you a clear choice between a personal Codex user-scope
entry and a project-scoped Claude Code `.mcp.json` entry.

For complete host setup options, dry-run previews, repository guidance, removal,
and troubleshooting, see [Agent Host Setup](../guides/agent-host-setup.md).

## Audience, Goal, And Completion

Audience: first-time users or operators who have already verified local
`volicord` and `volicord-mcp` executables and want one agent host path to work
before expanding the setup.

Goal: install one supported host configuration, recognize whether the first
result is `complete` or `action_required`, and run an independent verification
command for the chosen path.

Completion state: the chosen path is complete when `volicord agent verify` for
that `integration_id` reports `status: complete` and the selected Host
Installation has `final_status: complete`. If the command reports
`action_required`, complete the named host-owned trust, approval, reload, or
restart action and rerun verification.

## Starting State And Example Values

Before running these commands:

- Complete [Installation](installation.md) in a POSIX-style shell.
- Keep `VOLICORD_BIN` set to the verified absolute directory containing both
  executables.
- Choose a `Product Repository` that is not the `Volicord Runtime Home` and is
  not inside or above it.
- Replace every example path and ID below with your real value.

Before choosing values, check the focused command help:

```sh
"$VOLICORD_BIN/volicord" agent install --help
```

That help shows the current command-specific contract for required arguments,
conditional project selection, conditional repository-write authorization,
optional values, and omission defaults. For the complete rules and edge cases,
use the
[Administrative CLI reference](../reference/admin-cli.md#volicord-agent-install).

The examples use these non-argument values:

| Value | Kind | How this walkthrough uses it |
|---|---|---|
| `VOLICORD_BIN="/absolute/path/to/selected/bin"` | Tutorial shell variable | Selected absolute directory containing both `volicord` and `volicord-mcp`. Volicord does not read `VOLICORD_BIN` as configuration. |
| `"$VOLICORD_BIN/volicord"` | Command invocation | Runs the `volicord` administrative CLI from the verified directory. |
| `"$VOLICORD_BIN/volicord-mcp"` | Executable path value | Supplies the verified absolute `volicord-mcp` path to Path A's `--mcp-command`. |
| `VOLICORD_HOME=/Users/alex/.volicord` | Environment variable assignment | Selects the Runtime Home for the administrative command when it appears before the command. It is not a CLI option. A later project-scoped host process also needs `VOLICORD_HOME` in its own launch environment if its default Runtime Home would differ. |
| `PATH="$VOLICORD_BIN:$PATH"` | Environment variable assignment | Lets the project-scope examples resolve the selected executables during the administrative command. A later Claude Code launch environment must still be able to find `volicord-mcp` on `PATH`. |
| `/Users/alex/.volicord` | Example path | `Volicord Runtime Home`; keep it distinct from the `Product Repository`. |
| `/work/acme-api` | Example path | Product Repository A. |
| `acme-api` | Example identifier | Stable logical project ID you choose or reuse for Product Repository A; it is not automatically derived from the directory name. |
| `int-codex-team`, `int-claude-acme` | Example identifiers | Predictable `integration_id` values used by later verify, status, configuration, and related commands. |
| `volicord-int-codex-team`, `volicord-int-claude-acme` | Derived identifiers | Stable host MCP server names derived from `integration_id` when `--server-name` is omitted. |

The table below covers every `volicord agent install` option used in the command
blocks and the two omitted options whose omission affects visible tutorial
output. It is not the complete option list.

| Argument | Example value | Meaning | Status in this walkthrough | Selection or omission rule |
|---|---|---|---|---|
| `--host` | Path A: `codex`; Path B: `claude-code` | Selects the host integration. | Always required. | Use `codex` with `--scope user` for Path A or `claude-code` with `--scope project` for Path B. Other host/scope combinations belong in the full setup guide and reference. |
| `--scope` | Path A: `user`; Path B: `project` | Selects where the host configuration is written or exported. | Always required. | Use `user` for the personal Codex config path. Use `project` for the repository-managed Claude Code `.mcp.json` path. The selected value must be compatible with `--host`. |
| `--project-id` | `acme-api` | Names the selected project with a stable logical project identifier chosen or reused by the operator. | Required for this new-project walkthrough. | Supply a stable ID for Product Repository A. It does not need to equal the directory name. Registered-project selection edge cases are in the Administrative CLI reference. |
| `--repo-root` | `/work/acme-api` | Identifies the `Product Repository` path associated with the selected project. | Required for this new-project walkthrough. | Supply the Product Repository path for Product Repository A. Do not use the `Volicord Runtime Home` path as the repository root. |
| `--integration-id` | Path A: `int-codex-team`; Path B: `int-claude-acme` | Selects an existing integration or the desired ID for a new integration. | Optional but pinned for reproducibility. | Keep the explicit IDs so later `verify`, `status`, generated configuration, and related commands have predictable identifiers. If omitted, the CLI derives a stable ID. |
| `--runtime-home` | Path A only: `/Users/alex/.volicord` | Selects the `Volicord Runtime Home` used by the administrative command. | Optional when normal Runtime Home resolution is acceptable; explicit in Path A. | Path A supplies the path so the tutorial does not rely on defaults. Path B uses the separate `VOLICORD_HOME` environment assignment for the administrative command because project-scoped host configuration must not persist a developer-specific Runtime Home path. |
| `--mcp-command` | Path A only: `"$VOLICORD_BIN/volicord-mcp"` | Selects the `volicord-mcp` command where an explicit command is allowed. | Optional; pinned only for the Codex user-scope example. | Path A pins the verified absolute executable in generated Codex configuration. Path B omits `--mcp-command` because project scope uses the portable `volicord-mcp` command when this option is omitted. |
| `--dry-run` | Path B preview command: present | Controls execution mode. | Optional execution control. | Include it to preview the install plan without performing the real write. Omit it for the apply command that performs the real installation. The corresponding dry run does not require `--allow-repository-write`. |
| `--output` | Path B preview command: `json` | Selects output formatting. | Optional output formatting. | `json` is chosen so the preview output is easy to inspect or compare during the tutorial. When omitted, output defaults to `text`. |
| `--allow-repository-write` | Path B apply command: present | Authorizes a repository-managed write. | Conditionally required for the real repository write. | Required for the non-dry-run project-scoped install that writes `/work/acme-api/.mcp.json`. Do not include it for the corresponding dry run. |
| `--default-project-id` | Omitted | Selects the integration default project. | Optional and intentionally omitted. | For a new integration in this walkthrough, omission makes the selected project the default project. |
| `--server-name` | Omitted | Selects the host MCP server name. | Optional and intentionally omitted. | Omission derives a stable `volicord-<integration>` server name, which is why the expected output and generated configuration use `volicord-int-codex-team` and `volicord-int-claude-acme`. |

## Choose One Host Path

| Path | Choose when | Consequence |
|---|---|---|
| Path A: Codex `user` scope | One personal Codex MCP entry should serve this repository now and may later serve more explicitly allowed repositories. | Host configuration lives in the Codex user config and stores an absolute `volicord-mcp` command path plus `VOLICORD_HOME`. |
| Path B: Claude Code `project` scope | Product Repository A should carry a team-shared Claude Code `.mcp.json` entry. | The project file uses portable `volicord-mcp`, omits personal `VOLICORD_HOME`, requires `--allow-repository-write` on the real apply command, and may remain `action_required` until Claude Code approval is complete. |

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
- `VOLICORD_BIN` names the verified absolute executable directory from
  Installation.
- Product Repository A is at `/work/acme-api`.
- `/Users/alex/.volicord` is separate from `/work/acme-api`.

Command:

```sh
"$VOLICORD_BIN/volicord" agent install \
  --host codex \
  --scope user \
  --integration-id int-codex-team \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --runtime-home /Users/alex/.volicord \
  --mcp-command "$VOLICORD_BIN/volicord-mcp"
```

Locations that may change:

| Location | What may change |
|---|---|
| `/Users/alex/.volicord` | Runtime Home registry, integration, project, surface, Host Installation, and project state records. |
| Codex user config, normally `~/.codex/config.toml` or `CODEX_HOME/config.toml` | A `[mcp_servers.volicord-int-codex-team]` table. |
| `/work/acme-api` | No file change unless repository guidance is selected separately. |

Because `--default-project-id` and `--server-name` are omitted, the new
integration uses the selected project as its default and the CLI derives a
stable host MCP server name from `integration_id`. Use `--server-name` only when
you need to pin a specific host configuration key.

Expected first result:

```text
status: complete
integration_id: int-codex-team
host_kind: codex
host_scope: user
server_name: volicord-int-codex-team
verification: complete
verification_detail: MCP initialize and tools/list succeeded
```

The generated Codex entry has this shape:

```toml
[mcp_servers.volicord-int-codex-team]
command = "/absolute/path/to/selected/bin/volicord-mcp"
args = ["--integration", "int-codex-team"]

[mcp_servers.volicord-int-codex-team.env]
VOLICORD_HOME = "/Users/alex/.volicord"
```

The actual `command` value is the shell-expanded absolute path selected through
`VOLICORD_BIN`; generated TOML does not contain `VOLICORD_BIN`.

Independent completion check:

```sh
"$VOLICORD_BIN/volicord" agent verify \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.volicord
```

Path A is complete when verification reports `status: complete`. If verification
reports `action_required`, read the named action. A common Codex user-scope cause
is that `codex` is missing from the administrative command `PATH` or cannot run
`codex --version`.

## Path B: Claude Code Project-Scope Setup

Use this when Product Repository A should carry a team-shared Claude Code
`.mcp.json` entry.

Prerequisites:

- `VOLICORD_BIN` names the verified absolute executable directory from
  Installation.
- `volicord-mcp` will be available on the `PATH` that Claude Code uses when it
  starts MCP servers.
- If Claude Code would not otherwise resolve `/Users/alex/.volicord` as its
  Runtime Home, the Claude Code launch environment must provide
  `VOLICORD_HOME=/Users/alex/.volicord`.
- Product Repository A is at `/work/acme-api`.
- `/Users/alex/.volicord` is separate from `/work/acme-api`.
- You intentionally allow the administrative command to write
  `/work/acme-api/.mcp.json`.

Optional dry-run before writing the project file:

```sh
VOLICORD_HOME=/Users/alex/.volicord \
PATH="$VOLICORD_BIN:$PATH" \
"$VOLICORD_BIN/volicord" agent install \
  --host claude-code \
  --scope project \
  --integration-id int-claude-acme \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --dry-run \
  --output json
```

Apply the setup:

```sh
VOLICORD_HOME=/Users/alex/.volicord \
PATH="$VOLICORD_BIN:$PATH" \
"$VOLICORD_BIN/volicord" agent install \
  --host claude-code \
  --scope project \
  --integration-id int-claude-acme \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --allow-repository-write
```

Locations that may change:

| Location | What may change |
|---|---|
| `/Users/alex/.volicord` | Runtime Home registry, integration, project, surface, Host Installation, and project state records. |
| `/work/acme-api/.mcp.json` | A Claude Code project-scoped MCP server entry. |
| Claude Code user approval state | Only after the user approves the project MCP server in Claude Code. |

Expected first result before host approval:

```text
status: action_required
integration_id: int-claude-acme
host_kind: claude_code
host_scope: project
server_name: volicord-int-claude-acme
verification: action_required
```

The output should name a host-owned follow-up such as Claude Code project MCP
approval. `action_required` is a successful administrative result, not command
failure.

The generated `.mcp.json` entry has this shape:

```json
{
  "mcpServers": {
    "volicord-int-claude-acme": {
      "command": "volicord-mcp",
      "args": ["--integration", "int-claude-acme"]
    }
  }
}
```

The generated `.mcp.json` intentionally omits `VOLICORD_HOME` and keeps the
portable `volicord-mcp` command. That portable command is the project-scope
default when `--mcp-command` is omitted. The `VOLICORD_HOME` and `PATH`
assignments on the install command apply only to that administrative invocation.
When Claude Code later starts the server, Claude Code's own launch environment
must be able to find `volicord-mcp` on `PATH` and must provide `VOLICORD_HOME` if
its default Runtime Home would be different.

Complete the host-owned action: start or restart Claude Code in
`/work/acme-api` from the intended environment, review the project MCP server,
and approve it in Claude Code.

Independent completion check after approval:

```sh
VOLICORD_HOME=/Users/alex/.volicord \
PATH="$VOLICORD_BIN:$PATH" \
"$VOLICORD_BIN/volicord" agent verify \
  --integration-id int-claude-acme
```

Path B is complete when verification reports `status: complete` and the
installation verification for `volicord-int-claude-acme` reports
`final_status: complete`. If verification still reports `action_required`, the
host-owned approval, reload/restart, or launch environment is still incomplete.

## Inventory, Verification, And Active Host Loading

Use `volicord agent status` to inspect registry and Host Installation inventory:

```sh
"$VOLICORD_BIN/volicord" agent status \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.volicord
```

`volicord agent status` is not proof that Codex or Claude Code loaded the MCP
server. Use `volicord agent verify` for the administrative verification gates,
and use the host's own UI, MCP list, or approval flow to confirm host-owned load
state when the host exposes it.

A successful `volicord-mcp --check --integration <integration_id>` is startup
validation for the MCP process only. It is not by itself complete host
integration.

## Setup State Meanings

| State | What to do next |
|---|---|
| `complete` | The administrative setup, host-owned gates, MCP initialization, and tool discovery succeeded. Use the host and confirm the server appears where the host exposes MCP servers or tools. |
| `action_required` | The command succeeded, but a named host-owned action remains. Complete that action, then run `volicord agent verify`. |
| `partial_failure` | Some durable action may have succeeded before a later step failed. Read `effects` and `residual_effects`, fix only the named issue, then rerun the same command or verify. |
| `failed` | The requested setup or verification did not establish usable durable integration state or host configuration. Fix the reported error before retrying. |

## Failure Routing

| Symptom | Safe next action | Route |
|---|---|---|
| `volicord`, `volicord-mcp`, or `VOLICORD_BIN` fails before setup. | Return to Installation and rerun the executable checks from the same shell. | [Installation](installation.md#verify-the-selected-directory) |
| Setup or verification cannot resolve `volicord-mcp`. | For user/local scope, use a valid absolute `--mcp-command`; for project scope, keep `volicord-mcp` portable and fix the host `PATH`. | [Agent Host Troubleshooting](../guides/agent-host-troubleshooting.md#missing-volicord-mcp) |
| A project-scoped command refuses to write `.mcp.json` or `.codex/config.toml`. | Rerun after deciding that the repository write is intended and include `--allow-repository-write`. | [Administrative CLI](../reference/admin-cli.md#noninteractive-approval-behavior) |
| Result is `action_required`. | Complete only the named host-owned trust, approval, reload, restart, or executable-availability action, then rerun `volicord agent verify`. | [Agent Host Troubleshooting](../guides/agent-host-troubleshooting.md#status-action_required) |
| Result is `partial_failure` or `failed`. | Read the reported `effects`, `residual_effects`, warnings, and verification details. Do not delete the Runtime Home, Product Repository, or unrelated host entries as a first response. | [Agent Host Troubleshooting](../guides/agent-host-troubleshooting.md#status-partial_failure) and [failed](../guides/agent-host-troubleshooting.md#status-failed) |
| One integration should serve several repositories. | Use a user-scope integration and add explicit project memberships; do not add one host entry per repository. | [Multi-Repository Agent Setup](../guides/multi-repository-agent-setup.md) |

## Continue

- Full host setup, dry-run preview, repository guidance, generic export, status,
  verification, and safe removal: [Agent Host Setup](../guides/agent-host-setup.md)
- One user-scope integration serving multiple repositories:
  [Multi-Repository Agent Setup](../guides/multi-repository-agent-setup.md)
- Agent workflow: [Agent Guide](../guides/agent-workflow.md)
- Exact `volicord` agent command behavior:
  [Administrative CLI](../reference/admin-cli.md#volicord-agent-install)
- Exact project selection and guidance boundaries:
  [Agent Integration](../reference/agent-integration.md)
- Exact `volicord-mcp` process behavior:
  [MCP Transport](../reference/mcp-transport.md)
- Exact runtime location boundaries:
  [Runtime Boundaries](../reference/runtime-boundaries.md)
