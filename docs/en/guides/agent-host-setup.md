# Agent host setup

Use this guide when you need to install, verify, inspect, guide, or remove a Harness MCP integration for Codex, Claude Code, or an unsupported host.

Start with [Installation](../getting-started/installation.md) to build or locate `harness` and `harness-mcp`, then [Quickstart](../getting-started/quickstart.md) for the shortest first setup. This guide covers the operational path after that.

Exact command behavior belongs to [Administrative CLI](../reference/admin-cli.md). Exact Agent Integration Profile, Host Installation, project selection, and guidance boundaries belong to [Agent Integration](../reference/agent-integration.md). Exact process behavior belongs to [MCP Transport](../reference/mcp-transport.md). Runtime and Product Repository write boundaries belong to [Runtime Boundaries](../reference/runtime-boundaries.md).

## Responsibilities

| Part | Owns | Notes |
|---|---|---|
| Harness installation | `harness` and `harness-mcp` executables. | Source builds write under `target/`; installed executables may live elsewhere. |
| `Harness Runtime Home` | Project registry, Agent Integration Profiles, integration project memberships, Host Installation inventory, and Harness runtime data. | Keep it separate from every `Product Repository`. |
| `Product Repository` | Product files and explicitly selected project-scoped integration files. | Harness runtime databases and runtime records are never stored there. |
| Codex or Claude Code | Host configuration, project trust, project MCP approval, reload/restart behavior, and model tool choice. | Harness cannot bypass host-owned decisions. |
| `harness-mcp` process | One integration-bound stdio server started with `--integration <integration_id>`. | Project selection happens per public tool call. |

## Setup State Semantics

| State | Meaning |
|---|---|
| `complete` | Durable integration state exists, managed host configuration matches its fingerprint, the host-specific loadability gate is satisfied, no required trust or approval action remains, integration preflight succeeded, MCP initialization succeeded, and tool discovery exposed the required tools. |
| `action_required` | Durable integration state and host configuration are present, but host trust, project approval, OAuth, reload, restart, or a comparable user-controlled host action remains. |
| `partial_failure` | Some durable administrative action succeeded, but a later install, verify, host target, or cleanup step failed. Rerun after fixing the reported issue. |
| `failed` | The requested install or verification did not establish usable durable integration state or host configuration. |

Codex project scope remains `action_required` while Codex project trust cannot be confirmed. Claude Code project scope remains `action_required` while project MCP approval is pending. Rejected, missing, changed, unavailable, and unknown host states are not `complete`. Generic export remains `action_required` because Harness cannot prove that a user-managed host loaded the exported configuration.

`harness-mcp --check --integration <integration_id>` is only MCP startup validation. A direct Harness-spawned MCP handshake is not proof that Codex or Claude Code loaded, trusted, approved, or exposed the server. Tool discovery does not guarantee every future model decision will choose Harness tools. Repository guidance improves discoverability, but it is advisory context rather than enforcement.

## Dry-Run Before Writes

Use dry-run when a command might write host configuration or `Product Repository` guidance:

```sh
/opt/harness/bin/harness agent install \
  --host codex \
  --scope user \
  --server-name harness-main \
  --integration-id int-codex-team \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --runtime-home /Users/alex/.harness \
  --mcp-command /opt/harness/bin/harness-mcp \
  --dry-run \
  --output json
```

Dry-run reports planned Runtime Home actions, host target paths, and guidance target paths. It creates or modifies nothing: no Runtime Home directories, SQLite databases or rows, WAL or SHM files, registry migrations, host configuration, `Product Repository` guidance, generic export files, MCP host state, `harness-mcp --check`, MCP initialization, or tool discovery.

When the selected Runtime Home has a schema version 1 registry, dry-run reports the pending migration without applying it:

```json
{
  "status": "dry_run",
  "runtime": {
    "registry_schema_version": 1,
    "registry_latest_supported_schema_version": 2,
    "registry_migration_planned": true
  },
  "actions": [
    {
      "target": "registry_migration",
      "action": "planned"
    }
  ]
}
```

After that preview, the registry remains at schema version 1 and no new Agent Integration tables, project-state databases, or migration metadata are written.

The examples below pin `--server-name harness-main` so the host snippets have a stable, human-readable key. The option is not required; if it is omitted, the CLI derives a stable server name from `integration_id` and reports it in the result.

## Codex User-Scope Install

Use user scope when one personal Codex configuration should load the same Harness integration across Codex projects.

```sh
/opt/harness/bin/harness agent install \
  --host codex \
  --scope user \
  --server-name harness-main \
  --integration-id int-codex-team \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --default-project-id acme-api \
  --runtime-home /Users/alex/.harness \
  --mcp-command /opt/harness/bin/harness-mcp
```

This may write:

- Runtime Home records under `/Users/alex/.harness`
- a Codex user `config.toml` entry such as `[mcp_servers.harness-main]`

It does not write `/work/acme-api` unless `--guidance codex`, `--guidance both`, or a separate guidance command is selected with `--allow-repository-write`.

Expected generated Codex shape:

```toml
[mcp_servers.harness-main]
command = "/opt/harness/bin/harness-mcp"
args = ["--integration", "int-codex-team"]

[mcp_servers.harness-main.env]
HARNESS_HOME = "/Users/alex/.harness"
```

Codex project scope is also supported, but it writes `/work/acme-api/.codex/config.toml`, requires `--allow-repository-write` in noninteractive execution, uses `harness-mcp` from `PATH`, and may report `action_required` until Codex trusts the project.

## Claude Code Project Or Local Install

Project scope writes a team-shared `.mcp.json` file in the `Product Repository`.

```sh
HARNESS_HOME=/Users/alex/.harness \
PATH="/opt/harness/bin:$PATH" \
/opt/harness/bin/harness agent install \
  --host claude-code \
  --scope project \
  --server-name harness-main \
  --integration-id int-claude-acme \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --mcp-command harness-mcp \
  --allow-repository-write
```

Expected `.mcp.json` shape:

```json
{
  "mcpServers": {
    "harness-main": {
      "command": "harness-mcp",
      "args": ["--integration", "int-claude-acme"]
    }
  }
}
```

Claude Code normally requires project MCP approval before it loads a project-scoped `.mcp.json` server. That result is `action_required`.

Local scope keeps the MCP server private to the current Claude Code project and uses Claude Code's own `claude mcp add --scope local` path through the CLI adapter:

```sh
HARNESS_HOME=/Users/alex/.harness \
/opt/harness/bin/harness agent install \
  --host claude-code \
  --scope local \
  --server-name harness-main \
  --integration-id int-claude-acme-local \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --mcp-command /opt/harness/bin/harness-mcp
```

Local and project scopes are single-repository scopes. Use user scope when one explicitly allowed integration should serve multiple repositories.

## Optional Repository Guidance

Repository guidance is optional and must be explicitly authorized.

Codex guidance writes a Harness-managed block in `AGENTS.md`:

```sh
/opt/harness/bin/harness agent guidance apply \
  --integration-id int-codex-team \
  --project-id acme-api \
  --host codex \
  --runtime-home /Users/alex/.harness \
  --dry-run \
  --allow-repository-write \
  --output json
```

Claude Code guidance writes `.claude/rules/harness.md`:

```sh
/opt/harness/bin/harness agent guidance apply \
  --integration-id int-codex-team \
  --project-id acme-api \
  --host claude-code \
  --runtime-home /Users/alex/.harness \
  --allow-repository-write
```

Before guidance, the target file is either absent or has no Harness-managed block:

```text
# Existing repository instructions
```

After Codex guidance, `AGENTS.md` contains a managed block:

```md
# Existing repository instructions

<!-- BEGIN HARNESS MANAGED GUIDANCE v1 -->
## Harness MCP guidance for Codex

...
<!-- END HARNESS MANAGED GUIDANCE v1 -->
```

After Claude Code guidance, `.claude/rules/harness.md` contains the same managed marker shape with `## Harness MCP guidance for Claude Code`.

The managed content tells the host to use Harness for scope, state, write preparation, run evidence, user judgment, and close-readiness tracking; to call `harness.list_projects` when the target repository is unclear; and not to invent Harness state in prose. The guidance also states that MCP server instructions and repository guidance cannot guarantee model behavior.

Guidance files are host configuration or advisory context. They are not Harness runtime state, Core authority, evidence, acceptance, close readiness, residual-risk acceptance, or a security guarantee.

## Status And Verification

Inspect registry and host inventory:

```sh
/opt/harness/bin/harness agent status \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness
```

Refresh verification:

```sh
/opt/harness/bin/harness agent verify \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness
```

Verification is performed per Host Installation. Add `--installation-id <id>` to verify exactly one installation; omit it to verify every Host Installation associated with the integration. Each installation keeps its own `last_verified_status`, and one installation's result does not overwrite another's.

Aggregate command status follows the selected installations:

| Selected installation results | Command status |
|---|---|
| All selected installations are `complete` | `complete` |
| At least one is `action_required`, and none is `partial_failure` or `failed` | `action_required` |
| At least one is `partial_failure`, and none is `failed` | `partial_failure` |
| At least one is `failed` | `failed` |

The aggregate status is never `complete` while any selected installation is not `complete`.

Direct MCP startup inspection:

```sh
HARNESS_HOME=/Users/alex/.harness \
/opt/harness/bin/harness-mcp --check --integration int-codex-team
```

`--check` should report `configuration: valid`, `transport: stdio`, the `integration_id`, allowed project counts, and `verification_scope: startup_check_only`. It does not prove the host loaded or exposed tools.

## Failure And Compensation

When installation or verification fails after some durable action has already happened, output distinguishes `failed` from `partial_failure`.

- `failed` means the requested operation did not leave usable durable integration state or host configuration.
- `partial_failure` means some durable administrative action succeeded but a later install, verify, host target, rollback, or cleanup step failed.

Human-readable output names `effects` and `residual_effects`; JSON output exposes the same facts as machine-readable entries. `effects` identify applied or rolled-back targets such as the integration record, project allowlist, default project, Host Installation inventory, managed host configuration, or managed guidance. `residual_effects` identify exact targets that remain and the operator action to take.

Harness attempts to reverse newly applied managed effects when it can do so safely. It deliberately does not roll back schema migrations, pre-existing project state, Core records, artifact storage, a `Product Repository`, or user-owned host/guidance edits. Fingerprint or ownership-marker conflicts protect manually changed host configuration and guidance; Harness reports the conflict instead of removing unrelated content.

## Safe Removal

A project that is still `default_project_id` cannot be removed. In a two-project integration, first change the default to the project that should remain:

```sh
/opt/harness/bin/harness agent project default set \
  --integration-id int-codex-team \
  --project-id billing-api \
  --runtime-home /Users/alex/.harness
```

Expected result includes:

```text
prior_default_project_id: acme-api
resulting_default_project_id: billing-api
```

After the default has moved, remove the formerly default project without rewriting host configuration:

```sh
/opt/harness/bin/harness agent project remove \
  --integration-id int-codex-team \
  --project-id acme-api \
  --runtime-home /Users/alex/.harness
```

Expected result includes:

```text
allowed_projects:
  billing-api
verification_detail: project membership removed; host configuration was not rewritten
```

To remove the final allowed project, clear the default first:

```sh
/opt/harness/bin/harness agent project default clear \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness
```

Then remove the final membership:

```sh
/opt/harness/bin/harness agent project remove \
  --integration-id int-codex-team \
  --project-id billing-api \
  --runtime-home /Users/alex/.harness
```

Expected result includes:

```text
allowed_project_count: 0
not executable until one is added
```

The Agent Integration Profile, Host Installation inventory, and host configuration can remain while no projects are allowed. Verification is not executable in that state. Add a project again without reinstalling the host entry:

```sh
/opt/harness/bin/harness agent project add \
  --integration-id int-codex-team \
  --project-id billing-api \
  --runtime-home /Users/alex/.harness
```

Fully remove managed host configuration and managed guidance:

```sh
/opt/harness/bin/harness agent uninstall \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness \
  --allow-repository-write \
  --remove-managed
```

Uninstall removes only Harness-managed host entries, blocks, files, or fingerprints. It does not delete a `Product Repository`, Runtime Home, project state, Core records, artifact store, or unrelated host configuration.

## Generic Export Fallback

Use generic export only for a host that Harness does not install directly:

```sh
/opt/harness/bin/harness agent install \
  --host generic \
  --scope export \
  --server-name harness-main \
  --integration-id int-generic-acme \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --runtime-home /Users/alex/.harness \
  --mcp-command /opt/harness/bin/harness-mcp \
  --export-dir /tmp/harness-mcp-export
```

The exported JSON contains one `mcpServers.harness-main` entry with `command`, `args = ["--integration", "int-generic-acme"]`, and `HARNESS_HOME` when applicable. Generic export does not claim the host loaded the server; install and verify results remain `action_required` until a future host-specific owner defines an observable loadability gate.
