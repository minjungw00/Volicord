# Agent Host Troubleshooting

Use this guide when `volicord setup`, `volicord connect`, `volicord connection
...`, or `volicord export mcp-config` reports a host setup problem. It assumes
the simplified command model where Volicord detects repository projects and
manages internal ids.

Exact result-state meaning belongs to
[Administrative CLI Reference](../reference/admin-cli.md#agent-connection-result-states).

## Before You Change Anything

Collect the current local state:

```sh
volicord doctor
volicord project current
volicord connections
```

If the command is being run outside the intended repository, either `cd` into
the repository or add `--repo PATH` to the project, connection, export, or user
command you are checking.

## Setup Has Not Been Completed

Observable symptom: ordinary project, connection, export, MCP, or user workflows
say setup has not been completed for the selected `Volicord Runtime Home`.

Bounded recovery:

```sh
volicord setup
volicord doctor
```

If you built from source and want a command link:

```sh
./target/debug/volicord setup --link-bin ~/.local/bin
volicord doctor
```

If `volicord` is still not found, add the link directory to your shell
configuration and start a new shell or MCP host. Setup can report the required
`PATH` action, but it cannot permanently change the parent shell.

Do not create Runtime Home files by hand. Use setup so the registry and setup
profile are created together.

## Repository Is Not Detected

Observable symptom: project or connection commands say no Git repository root
was found.

Bounded recovery:

```sh
cd /work/acme-api
volicord project current
volicord project use
```

Or select the repository explicitly:

```sh
volicord project use /work/acme-api
volicord connect codex --repo /work/acme-api
```

The user-facing project name comes from the repository directory. Internal
project ids are not recovery inputs.

## Host Cannot Be Selected

Observable symptom: `volicord connect` or `volicord connection ...` cannot infer
the host, or the host value is unsupported.

Bounded recovery: pass the host explicitly:

```sh
volicord connect codex
volicord connect claude-code
volicord connection status codex
```

Use the same intent selector used for the connection:

```sh
volicord connection status codex --shared
volicord connection verify claude-code --global
```

Codex supports personal and shared connection intents. Claude Code supports
personal, shared, and global connection intents.

## `action_required`

Observable symptom: connection status or verification reports
`status: action_required`.

Bounded recovery:

```sh
volicord connection status codex
volicord connection verify codex
```

Read the reported action and complete only that host-owned step. Common actions
include trusting a host entry, approving a project MCP entry, signing in through
the host, reloading the host, restarting the host, or rerunning setup. Then run
verification again.

Do not treat `action_required` as a fatal failure. Durable Volicord-side state
may already exist.

## `failed`

Observable symptom: setup, connect, export, or verification reports `failed` or
exits with a runtime error.

Bounded recovery:

1. Run `volicord doctor`.
2. Fix the first failed setup or executable check it names.
3. Rerun the original command with `--dry-run` when the command supports it.
4. Rerun the real command only after the dry-run plan names the expected host
   and repository.

Use the exact failure text to choose the next action. Do not delete Runtime Home
state or host configuration by hand unless an owner document or human operator
has identified that as the intended recovery.

## MCP Command Is Unavailable

Observable symptom: setup or verification reports that `volicord-mcp` cannot be
found, launched, or initialized.

Bounded recovery:

```sh
cargo build --workspace --bins
./target/debug/volicord setup --link-bin ~/.local/bin
volicord doctor
volicord connection verify codex
```

Setup is the place that records the MCP command used by managed host
configuration and generic exports. Ordinary `connect` commands do not ask users
to pass an MCP command path. If the executable is installed somewhere setup
cannot discover by sibling lookup or `PATH`, rerun setup with
`--mcp-command PATH`.

## Shared Connection Needs Host Approval

Observable symptom: a shared connection writes or updates a project integration
file, but the host still does not load Volicord tools.

Bounded recovery:

```sh
volicord connection status codex --shared
volicord connection verify codex --shared
```

Complete the host-owned project approval or reload action named by the command.
The `Product Repository` integration file is not Core authority and does not
prove that the host loaded, trusted, or exposed the MCP server.

## Generic Export Does Not Appear In The Host

Observable symptom: `volicord export mcp-config` produced a file, but an external
host does not show Volicord tools.

Bounded recovery:

```sh
volicord export mcp-config --output /tmp/volicord.mcp.json
volicord doctor
```

Then load or reload the exported file through the external host's own
configuration process. The exported file is user-managed after export.

## Removal Completed Only Partially

Observable symptom: `volicord connection remove ...` reports that host
configuration could not be removed, or a connection still appears for another
repository.

Bounded recovery:

```sh
volicord connection remove codex --dry-run
volicord connection status codex
volicord connections
```

Removal first removes the selected repository membership. It removes the Agent
Connection and managed host configuration only when no owned membership remains
and safety checks permit it. It must not remove the `Product Repository`,
project state, Core records, artifact storage, or unrelated host entries.

## Security Boundary

Volicord setup and verification are local diagnostics. They do not prove that an
external host is secure, that a model will use Volicord tools, or that file
writes are safe. For exact security wording, use [Security](../reference/security.md).
