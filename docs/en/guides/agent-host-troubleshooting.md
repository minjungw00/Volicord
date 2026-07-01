# Agent Host Troubleshooting

Use this guide when `volicord init`, `volicord setup`, `volicord connect`,
`volicord connection ...`, or `volicord export mcp-config` reports a host setup
problem. It assumes the simplified command model where Volicord detects Product
Repositories and manages internal identities.

Exact setup, doctor, and connection result-state meanings belong to
[Administrative CLI Reference](../reference/admin-cli.md#runtime-home-selection)
and
[Connection result states](../reference/admin-cli.md#agent-connection-result-states).

## Before You Change Anything

Collect the current local state:

```sh
volicord doctor
volicord project current
volicord connections
```

If the command is being run outside the intended Product Repository, either `cd`
into that repository or add `--repo PATH` to the project, connection, export, or
user command you are checking.

`volicord setup` and `volicord doctor` answer different status questions. Setup
reports whether installation-profile preparation still needs a user action.
Doctor reports whether the saved installation profile is usable. A profile can
therefore make doctor report `complete` while doctor still shows
command-availability warnings or recommended `PATH` and command-link actions for
future shells or agent hosts.

## Setup Has Not Been Completed

Observable symptom: ordinary project, connection, export, MCP, or user workflows
say setup has not been completed for the selected `Volicord Runtime Home`.

Bounded recovery:

If `volicord` is already available:

```sh
volicord setup
volicord doctor
```

If `volicord` is not available, rerun the release binary path in
[Installation](../getting-started/installation.md). If you are intentionally
working from a development source checkout:

```sh
cargo build --workspace --bins
./target/debug/volicord setup
```

Follow setup's prompt or `action_required` output if it asks how to make
`volicord` available. If it prints a shell command, run that
command in the terminal that will continue the setup. If it writes or asks you
to update a shell startup file, open a new shell or restart or reload the agent
host before checking again:

```sh
volicord doctor
```

Use `--link-bin PATH` only when you need a deterministic command-link directory
for automation or a special local layout. Setup can report the required `PATH`
action or a repair action if that directory is not writable, but it cannot
permanently change the parent shell.

Do not create Runtime Home files by hand. Use setup so the registry and setup
profile are created together.

## Setup Does Not Offer `~/.local/bin`

Observable symptom: interactive setup reports that commands are not available
on `PATH`, but it does not offer to create `~/.local/bin`.

Bounded recovery:

Setup offers a conventional user command directory only when it can identify a
safe candidate under `HOME`, create it safely when missing, and verify
writability before creating command links. It may leave a manual `PATH` action
instead when `HOME` is missing or not writable, the shell or platform is not
supported for that guided choice, the candidate path conflicts with an existing
unsafe entry, or setup is running in JSON, non-TTY, or explicit `--link-bin`
mode.

Safe next steps:

- Rerun `volicord setup` in an interactive terminal and follow the prompt or
  `action_required` output.
- Run `volicord setup --link-bin PATH` with a command directory you control.
  Setup creates the directory if needed, verifies it is writable, and does not
  edit shell startup files by itself.
- Create `~/.local/bin` manually only when that is the command directory you
  want to control, then rerun setup.

If setup prints a shell command or names a `PATH` action, run that command in
the terminal that needs it or update the supported startup file it names.
Volicord can help make commands available on `PATH`, but it cannot directly
mutate the current parent shell environment. Already-running agent hosts may
need restart or reload before they see a new command directory.

## Repository Is Not Detected

Observable symptom: project or connection commands say no Git repository root
was found.

Bounded recovery:

```sh
cd /path/to/your-product-repo
volicord project current
volicord project use
```

Or select the Product Repository explicitly:

```sh
volicord init --host codex --repo /path/to/your-product-repo --mode mcp-only
```

`/path/to/your-product-repo` is an example path for the Product Repository where
you want the agent to work. The user-facing project name comes from the
repository directory. Internal project identities are not recovery inputs.

## Host Cannot Be Selected

Observable symptom: `volicord connect` or `volicord connection ...` cannot infer
the host, or the host value is unsupported.

Bounded recovery: for ordinary first-run setup without lifecycle hook
installation, pass the host, repository, and `mcp-only` mode to init explicitly:

```sh
volicord init --host codex --repo /path/to/your-product-repo --mode mcp-only
```

For guarded or managed setup, use the full init contract in the
[Administrative CLI Reference](../reference/admin-cli.md#agent-host-setup-and-init);
missing verified hook support requires an explicit degraded opt-in.

For lower-level connection recovery, pass the host and repository to connect
explicitly:

```sh
volicord connect codex --repo /path/to/your-product-repo
volicord connection status codex --repo /path/to/your-product-repo
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
volicord connection status codex --repo /path/to/your-product-repo
volicord connection verify codex --repo /path/to/your-product-repo
```

Read the reported action and complete only that host-owned step. Common actions
include trusting a host entry, approving a project MCP entry, signing in through
the host, reloading the host, restarting the host, or completing
installation-profile repair. Then run verification again.

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
   and Product Repository.

Use the exact failure text to choose the next action. Do not delete Runtime Home
state or host configuration by hand unless an owner document or human operator
has identified that as the intended recovery.

## MCP Command Is Unavailable

Observable symptom: setup or verification reports that `volicord mcp --stdio`
cannot be found, launched, or initialized.

Bounded recovery:

Rerun setup with the installed release binary:

```sh
volicord setup
```

If you are intentionally working from a development source checkout:

```sh
cargo build --workspace --bins
./target/debug/volicord setup
```

Complete any setup prompt or `action_required` command-availability step, then
check the installation and connection again:

```sh
volicord doctor
volicord connection verify codex --repo /path/to/your-product-repo
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
Product Repository.

Bounded recovery:

```sh
volicord connection remove codex --dry-run
volicord connection status codex
volicord connections
```

Removal first removes the selected Product Repository membership. It removes the Agent
Connection and managed host configuration only when no owned membership remains
and safety checks permit it. It must not remove the `Product Repository`,
project state, Core records, artifact storage, or unrelated host entries.

## Security Boundary

Volicord setup and verification are local diagnostics. They do not prove that an
external host is secure, that a model will use Volicord tools, or that file
writes are safe. For exact security wording, use [Security](../reference/security.md).
