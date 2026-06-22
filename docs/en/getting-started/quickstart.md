# Quickstart

This page owns the shortest supported first setup path for a real local agent host. It assumes you can build or locate the Harness Server executables and that you have a `Product Repository` you want to allow.

For build details and executable discovery rules, see [Installation](installation.md). For complete host setup options, dry-run previews, repository guidance, removal, and troubleshooting, see [Agent Host Setup](../guides/agent-host-setup.md).

The examples use:

| Example value | Meaning |
|---|---|
| `/opt/harness/bin/harness` | installed `harness` executable |
| `/opt/harness/bin/harness-mcp` | installed `harness-mcp` executable |
| `/Users/alex/.harness` | `Harness Runtime Home` |
| `/work/acme-api` | Product Repository A |
| `acme-api` | project ID for Product Repository A |
| `harness-int-codex-team`, `harness-int-claude-acme` | stable host MCP server names derived from `integration_id` |

## Stage 1: Prepare Harness Server

Working directory: Harness Server source repository root, if building from this repository.

```sh
cargo build -p harness-cli -p harness-mcp
```

This provides:

- `target/debug/harness`
- `target/debug/harness-mcp`

Use those files by absolute path, or use installed executables that provide the same `harness` and `harness-mcp` commands.

## Path A: Codex User-Scope Setup

Use this when one personal Codex MCP entry should serve one or more explicitly allowed `Product Repository` registrations.

Prerequisites:

- Codex can read its user `config.toml`.
- `harness-mcp` is available by absolute path.
- Product Repository A is at `/work/acme-api`.
- `/Users/alex/.harness` is separate from `/work/acme-api`.

Command:

```sh
/opt/harness/bin/harness agent install \
  --host codex \
  --scope user \
  --integration-id int-codex-team \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --default-project-id acme-api \
  --runtime-home /Users/alex/.harness \
  --mcp-command /opt/harness/bin/harness-mcp
```

Locations that may change:

| Location | What may change |
|---|---|
| `/Users/alex/.harness` | Runtime Home registry, integration, project, surface, Host Installation, and project state records. |
| Codex user config, normally `~/.codex/config.toml` or `CODEX_HOME/config.toml` | A `[mcp_servers.harness-int-codex-team]` table. |
| `/work/acme-api` | No file change unless repository guidance is selected separately. |

Because `--server-name` is omitted, the CLI derives a stable host MCP server name from `integration_id`. Use `--server-name` only when you need to pin a specific host configuration key.

Expected result:

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
command = "/opt/harness/bin/harness-mcp"
args = ["--integration", "int-codex-team"]

[mcp_servers.harness-int-codex-team.env]
HARNESS_HOME = "/Users/alex/.harness"
```

Verify later:

```sh
/opt/harness/bin/harness agent status \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness

/opt/harness/bin/harness agent verify \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness
```

Recognize success:

- `status: complete` on install or verify means durable integration state exists, host configuration was installed, host-owned trust or approval gates are satisfied or not applicable, MCP initialization succeeded, and tool discovery succeeded.
- `harness agent status` is inventory/status reporting. Its verification section may say it does not prove host loading.

## Path B: Claude Code Project-Scope Setup

Use this when Product Repository A should carry a team-shared Claude Code `.mcp.json` entry.

Prerequisites:

- `harness-mcp` is available on the `PATH` that Claude Code will use.
- Product Repository A is at `/work/acme-api`.
- `/Users/alex/.harness` is separate from `/work/acme-api`.
- You are willing to write `.mcp.json` in Product Repository A.

Command:

```sh
HARNESS_HOME=/Users/alex/.harness \
PATH="/opt/harness/bin:$PATH" \
/opt/harness/bin/harness agent install \
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

Expected result:

```text
status: action_required
verification: action_required
verification_detail: Claude Code requires user approval before project-scoped .mcp.json servers load
```

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

`action_required` is not a setup failure. Start Claude Code in `/work/acme-api`, review and approve the project-scoped MCP server, then run:

```sh
HARNESS_HOME=/Users/alex/.harness \
/opt/harness/bin/harness agent verify \
  --integration-id int-claude-acme
```

## Dry-Run First

Use `--dry-run --output json` before writing project-scoped configuration or repository guidance:

```sh
/opt/harness/bin/harness agent install \
  --host codex \
  --scope user \
  --integration-id int-codex-team \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --runtime-home /Users/alex/.harness \
  --mcp-command /opt/harness/bin/harness-mcp \
  --dry-run \
  --output json
```

Dry-run output reports `status: dry_run`, planned actions, host target paths, and guidance target paths when selected. It creates or modifies no Runtime Home directories, SQLite files or rows, WAL or SHM files, registry migrations, host configuration, `Product Repository` guidance, or generic export files.

## Setup State Meanings

| State | What to do next |
|---|---|
| `complete` | The administrative setup, host-owned gates, and MCP verification path succeeded. Use the host and confirm the server appears in its MCP UI or tool list. |
| `action_required` | Complete the host-owned action named in the output, such as Codex project trust or Claude Code project MCP approval, then run `harness agent verify`. |
| `partial_failure` | Some durable action may have succeeded before a later step failed. Fix the reported issue and rerun the same command. |
| `failed` | The requested setup did not establish usable durable integration state or host configuration. Fix the reported error before retrying. |

A successful `harness-mcp --check --integration <integration_id>` is only startup validation for the MCP process. It is not by itself complete host integration. Host configuration presence is not the same as host loading or tool discovery. Tool discovery also does not guarantee that every future model decision will choose Harness tools.

## Continue

- Full host setup, dry-run preview, repository guidance, generic export, status, verification, and safe removal: [Agent Host Setup](../guides/agent-host-setup.md)
- One user-scope integration serving multiple repositories: [Multi-Repository Agent Setup](../guides/multi-repository-agent-setup.md)
- Agent workflow: [Agent Guide](../guides/agent-workflow.md)
- Exact `harness` agent command behavior: [Administrative CLI](../reference/admin-cli.md#harness-agent-install)
- Exact project selection and guidance boundaries: [Agent Integration](../reference/agent-integration.md)
- Exact `harness-mcp` process behavior: [MCP Transport](../reference/mcp-transport.md)
- Exact runtime location boundaries: [Runtime Boundaries](../reference/runtime-boundaries.md)
