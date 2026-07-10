# Quickstart

This tutorial connects one supported agent host to one Product Repository. It
starts after [Installation](installation.md) has made `volicord` available on
`PATH`.

Use [Administrative CLI](../reference/admin-cli.md) for exact command behavior
and [Agent Connection](../reference/agent-connection.md) for exact connection
semantics.

## 1. Initialize The Repository Connection

For Codex, run:

```sh
volicord init --host codex --repo "<repo>" --profile record
```

`<repo>` is the Git repository where the agent will work. Use
`--host claude-code` for Claude Code.

The command creates or reuses local Volicord state, registers the repository,
and writes project-scoped MCP configuration and guidance. Generated host
configuration starts `volicord mcp --stdio` for this connection.

Read the command's `Next` section. It tells you which host-owned action remains,
such as restarting the host, trusting the project, or approving the MCP entry.
Writing configuration does not prove that a running host has loaded it.

Use the Record profile for this first connection. It does not require host
lifecycle hooks or a session watcher. The Detective profile has additional
host, platform, and repository prerequisites described in
[Agent Host Setup](agent-host-setup.md#integration-profiles).

## 2. Complete The Host Action

After setup, open or restart the selected host in the Product Repository.

| Host | Check in the host |
|---|---|
| Codex | Complete any project trust prompt, then confirm that the active session can see Volicord tools. |
| Claude Code | Complete any project MCP approval, check `/mcp`, then confirm that the active session can see Volicord tools. |

An already running host may have an older `PATH` or configuration snapshot. If
the host cannot launch `volicord`, restart it from an environment where the
command is available.

## 3. Verify The Connection

For the Codex connection created above, run:

```sh
volicord connection verify codex --shared --repo "<repo>"
volicord connection status codex --shared --repo "<repo>"
```

Use `claude-code` instead of `codex` for Claude Code.

Read `Status`, `Checks`, `Next`, and `Diagnostics` in the text output:

- `complete` means the checks required by this setup path are ready.
- `action_required` means a named local or host action remains. Complete it and
  rerun verification.
- `failed` means a required check did not succeed.

Use `--json` for automation or full diagnostics. Do not parse the compact human
text. Exact result-state meanings belong to
[Administrative CLI](../reference/admin-cli.md#agent-connection-result-states).

CLI verification can start and talk to the MCP process from its check
environment. That result alone does not prove active host tool exposure. In the
active host session, ask for these read-only calls:

1. `volicord.list_projects`
2. `volicord.status`

They confirm tool visibility, project selection, and readable project state
without creating a `Task`.

If only read-compatible tools appear, or the host exposes no Volicord tools,
use [Agent Host Troubleshooting](agent-host-troubleshooting.md). The guide
separates host trust, command availability, active tool exposure, and Runtime
Home write capability.

## 4. Start Normal Work

Ask for work in ordinary language:

```text
Inspect the current authentication flow, add the requested lockout message, run the focused checks, and tell me what still blocks close.
```

The agent should keep the current task, scope, evidence, pending User Judgment,
and Close Status visible. When a user-owned decision must be recorded, use the
User Channel path that Volicord shows. The stable CLI fallback is:

```sh
volicord inbox --repo "<repo>"
volicord inbox answer JUDGMENT_ID --choice CHOICE_ID --repo "<repo>"
```

## If The Fast Path Is Not Enough

Use the lower-level connection commands only when you need a personal, global,
or read-only connection, multi-repository operation, or explicit removal. These
choices are covered in [Agent Host Setup](agent-host-setup.md) and
[Multi-Repository Agent Setup](multi-repository-agent-setup.md).

| Need | Read |
|---|---|
| Host-specific setup and removal | [Agent Host Setup](agent-host-setup.md) |
| `action_required`, `failed`, or missing tools | [Agent Host Troubleshooting](agent-host-troubleshooting.md) |
| User workflow and decision boundaries | [User Workflow](user-workflow.md) |
| Agent operating guidance | [Agent Guide](agent-workflow.md) |
