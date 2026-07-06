# Agent Host Troubleshooting

Use this guide when `volicord init`, `volicord connection add`, or
`volicord connection ...` reports a host setup problem. It assumes the
simplified command model where Volicord detects Product Repositories and
manages internal identities.

Exact setup, doctor, and connection result-state meanings belong to
[Administrative CLI Reference](../reference/admin-cli.md#runtime-home-selection)
and
[Connection result states](../reference/admin-cli.md#agent-connection-result-states).

## Before You Change Anything

Collect the current local state:

```sh
volicord doctor
volicord project current
volicord connection list
```

If the command is being run outside the intended Product Repository, either `cd`
into that repository or add `--repo PATH` to the project, connection, or inbox
command you are checking.

`volicord init` and `volicord doctor` answer different status questions. Init
reports whether first-run repository setup and host connection still need a user
or host action. Doctor reports whether the saved installation profile is usable.
A profile can therefore make doctor report `complete` while doctor still shows
command-availability warnings or recommended `PATH` actions for future shells or
agent hosts.

For `volicord init`, read the compact onboarding output first: confirm the
title, profile, repository, repo file changes, stored Runtime Home path, and
then follow the `Next:` checklist. That checklist is the setup flow for host
reload or restart, project trust or approval, and the follow-up
`volicord connection verify ...` command.

Diagnostic commands do not all use the same text layout. For
`volicord connection status` and `volicord connection verify`, read the
`Status`, `Checks`, `Next`, and `Diagnostics` sections first. `Status` and
`Checks` show the connection state and attempted checks, `Next` names the
host-owned or local follow-up, and `Diagnostics` points to the JSON diagnostic
command. For other views, follow the result-line labels the command prints
rather than assuming the connection layout.

Use `--json` when you need the stable automation surface or full diagnostic
fields. Compact human text is for interactive recovery and should not be parsed
by scripts.

When MCP startup or tool discovery is the symptom and you have the process
binding values from JSON diagnostics or generated host configuration, inspect
startup storage capability directly:

```sh
volicord mcp --check --connection "<connection_id>" --project "<project_id>"
```

Read `registry_read`, `project_state_read`, `project_state_write`,
`startup_observation`, and `effective_tool_mode` together. A successful startup
check is not a complete host verification and does not prove active host tool
exposure.

## Installation Profile Is Missing

Observable symptom: ordinary project, connection, MCP, or inbox workflows say
`SETUP_REQUIRED` or report that the installation profile is missing for the
selected `Volicord Runtime Home`.

Bounded recovery:

If `volicord` is already available:

```sh
volicord init --host codex --repo "<repo>" --profile record
volicord doctor
```

If `volicord` is not available, rerun the release binary path in
[Installation](../user-guide/installation.md). If you are intentionally
working from a development source checkout:

```sh
cargo build --workspace --bins
./target/debug/volicord init --host codex --repo "<repo>" --profile record
```

Follow init's `action_required` output if it asks how to make `volicord`
available, approve host trust, or reload the host. If you update a shell startup
file yourself, open a new shell or restart or reload the agent host before
checking again:

```sh
volicord doctor
```

Do not create Runtime Home files by hand. Use init so the registry,
installation profile, project registration, and connection state are created
together.

## Command Is Not On PATH

Observable symptom: init or doctor reports that `volicord` is not available on
`PATH` for future terminals or agent hosts.

Bounded recovery:

Install or link the `volicord` binary into a command directory you control, then
ensure that directory is visible through `PATH`. Volicord cannot directly mutate
the current parent shell environment. Already-running agent hosts may need
restart or reload before they see a new command directory.

## Repository Is Not Detected

Observable symptom: project or connection commands say no Git repository root
was found.

Bounded recovery:

```sh
cd "<repo>"
volicord project current
volicord project use
```

Or select the Product Repository explicitly:

```sh
volicord init --host codex --repo "<repo>" --profile record
```

`<repo>` is the Product Repository path where you want the agent to work. The
user-facing project name comes from the
repository directory. Internal project identities are not recovery inputs.

## Windows Path Is Rejected

Observable symptom: native Windows setup reports that a Runtime Home or Product
Repository path is invalid because it is a UNC path, a WSL UNC path, or a
WSL-style `/mnt/<drive>` path.

Bounded recovery:

- Use a native local drive-letter path such as `C:\Users\you\product-repo` for
  `--repo` and for any explicit `VOLICORD_HOME` or `--home` value.
- If the Product Repository is inside WSL2, run the Linux Volicord binary inside
  that WSL2 environment instead of passing the WSL path to native Windows
  `volicord.exe`.
- Do not use a network share as the Runtime Home or Product Repository for
  native Windows setup.

## Host Cannot Be Selected

Observable symptom: `volicord connection add` or `volicord connection ...` cannot infer
the host, or the host value is unsupported.

Bounded recovery: for ordinary first-run setup without lifecycle hook
installation, pass the host, repository, and `record` profile to init explicitly:

```sh
volicord init --host codex --repo "<repo>" --profile record
```

For detective setup, use the full init contract in the
[Administrative CLI Reference](../reference/admin-cli.md#agent-host-setup-and-init);
missing host-hook or session watcher support must fail with an actionable
diagnostic. Use `--profile record` when detective prerequisites are unavailable.

On native Windows, detective setup is not supported. If init reports
`DETECTIVE_WINDOWS_UNSUPPORTED`, rerun with the record profile:

```powershell
volicord init --host codex --repo "<repo>" --profile record
```

Use WSL2, Linux, or macOS for detective only where the selected host hook and
session watcher contracts are supported and tested.

For lower-level connection recovery, pass the host and repository to connect
explicitly:

```sh
volicord connection add codex --repo "<repo>"
volicord connection status codex --repo "<repo>"
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
`action_required` in text or JSON output.

Bounded recovery:

```sh
volicord connection status codex --shared --repo "<repo>"
volicord connection verify codex --shared --repo "<repo>"
```

Read the `Status`, `Checks`, `Next`, and `Diagnostics` sections first. Complete
only the named host-owned or local step. Common actions include trusting a host
entry, approving a project MCP entry, signing in through the host, reloading the
host, restarting the host, or completing installation-profile repair. If
`Next` shows a `volicord connection verify ...` command, run it after the
host-side step.

Do not treat `action_required` as a fatal failure. Durable Volicord-side state
may already exist.

Other actionable `Next` lines stay local to the selected workflow. If output
names `volicord inbox`, inspect or answer the pending user judgment from the
terminal. If output says no local consent URL is available, use the shown CLI
answer command or the URL already shown by the MCP Judgment Inbox item. If the
selector is ambiguous or the wrong repository is selected, rerun with
`--repo PATH` and the matching intent flag such as `--shared` or `--global`.

## Read-Only Host Storage

Observable symptom: the MCP host environment can read Volicord configuration or
start `volicord`, but direct sandbox execution fails with a storage error such
as SQLite reporting an attempt to write a read-only database. An elevated or
less-restricted diagnostic run may make `initialize`, `tools/list`, and
read-only status calls work.

Expected behavior:

- MCP startup and `tools/list` are read-only tolerant when the Runtime Home
  registry and project state can be read.
- Mutation tools require writable project state in the selected
  `Volicord Runtime Home`; they may be absent from `tools/list` or return a
  structured `MCP_UNAVAILABLE` rejection under read-only storage.
- A `workflow` connection with readable but non-writable project state becomes
  `read_only_degraded` for effective tool discovery.
- Read-only-compatible tools such as `volicord.status`,
  `volicord.check_close`, and `volicord.list_projects` can still be visible
  when the project state is readable.

Bounded recovery:

```sh
volicord mcp --check --connection "<connection_id>" --project "<project_id>"
```

If `project_state_write` is `readonly` and `effective_tool_mode` is
`read_only_degraded`, repair the MCP host environment so the selected Runtime
Home and project state are writable when workflow mutation tools are expected.
If the intended host integration is read-only, keep the connection in
read-only mode and do not expect workflow tools. If elevated execution succeeds
while the normal host sandbox fails, treat that as a storage-capability
diagnostic, not as proof that the active host session has loaded or exposed the
same tools.

<a id="trusted-codex-project-but-host-runtime-is-not-observed"></a>
## Trusted Codex Project And CLI Handshake Passed But Tools Are Not Exposed

Observable symptom: connection status or verification reports all of these
facts together:

- `Codex project trust: trusted`
- `MCP configuration: match` or `Current MCP configuration: match`
- `CLI MCP preflight: passed` or `MCP preflight: passed`
- `CLI MCP handshake: passed` or `MCP handshake: passed`
- the active Codex session does not expose `volicord.*` tools

Other lines may show `Managed Codex MCP startup: not observed`,
`Managed Codex MCP startup: unknown`, or
`Host MCP command: uses volicord from the Codex host PATH`. Codex MCP
startup/tool-list logs may also show that startup completed, including a
`startup_complete` entry, but no cached tool snapshot or no listed
`volicord.*` tools for the active session.

This means the repo-local MCP configuration matches and terminal-side
verification succeeded, but that does not prove that the active Codex session
has registered Volicord tools. Codex may know the MCP server exists, or may
have started it, while the active session still lacks a tool snapshot or tool
listing. Volicord cannot fully diagnose Codex internal tool registration
without the Codex host logs.

Bounded recovery:

Use these branches before changing configuration:

- Inspect the JSON diagnostics first:

  ```sh
  volicord connection status codex --shared --repo "<repo>" --json
  volicord connection verify codex --shared --repo "<repo>" --json
  ```

  Read `checks[]`, `actions[]`, `verification.project_trust`,
  `verification.host_runtime`, `verification.active_tool_exposure`, and
  `verification.host_mcp_command` as separate facts. Do not collapse CLI MCP
  handshake success into active-session tool exposure.
- The active Codex session did not start the MCP server. Restart, reload,
  resume, or start a new Codex session in the Product Repository after
  confirming command launchability in that host environment.
- Codex startup or tool-list logs show server launch, `initialize`,
  `tools/list`, or tool-registration failure. Follow the host log failure first.
- The host can launch `volicord`, but project state is read-only. Run the
  `volicord mcp --check` startup diagnostic and inspect `project_state_write`
  and `effective_tool_mode`.
- CLI-side preflight or handshake succeeds, but the active host session still
  does not list `volicord.*` tools. Treat the CLI result as terminal-side MCP
  verification only.
- Elevated execution succeeds while sandbox execution fails. Compare Runtime
  Home and project-state write capability in the actual MCP host environment.

First check the active Codex session tool search or tool list for `volicord.*`
tools. Then inspect Codex MCP startup/tool-list logs for server launch,
`initialize`, `tools/list`, cached tool snapshot, and tool-registration entries.
If the logs show startup complete but no tool snapshot or no listed
`volicord.*` tools, restart, reload, resume, or start a new Codex session in
the Product Repository and compare whether tool exposure changes.

If you start Codex from a shell, check the same shell environment before
starting or resuming Codex:

```sh
command -v volicord
```

For a Codex IDE extension, inspect the PATH visible to the extension session or
its MCP startup logs. For a non-interactive Codex run, start a new run after
fixing the launch environment. For remote or executor-backed MCP startup,
confirm command availability in that executor; local CLI PATH does not prove
remote command launchability.

Inspect the generated `<repo>/.codex/config.toml` entry when configuration
match is in doubt. A Volicord-managed project-scoped Codex entry should include
the managed launch markers `VOLICORD_MCP_LAUNCH=managed_host`,
`VOLICORD_MCP_HOST=codex`, `VOLICORD_MCP_CONNECTION_ID=<connection_id>`, and
`VOLICORD_MCP_PROJECT_ID=<project_id>` together with the matching command and
args. If the command and args are present without those markers, rerun
Volicord setup or connection management to regenerate the managed entry.

After any host-side change, rerun terminal-side verification:

```sh
volicord connection verify codex --shared --repo "<repo>"
```

For a direct MCP lifecycle check outside Codex, use the manual or elevated
`VOLICORD_MCP_VERIFICATION=1` probes in
[MCP Transport](../reference/mcp-transport.md#manual-stdio-lifecycle-probe).
The process command shape is:

```sh
VOLICORD_MCP_VERIFICATION=1 volicord mcp --stdio --connection "<connection_id>" --project "<project_id>"
```

Pipe the JSON-RPC examples from the Reference page into that process. Expected
differences:

- `initialize` and `tools/list` check discovery and should not be treated as
  mutation readiness.
- `tools/call` before `notifications/initialized` should fail with JSON-RPC
  Invalid Request.
- A read-only status call after initialization can succeed when project state
  is readable.
- A mutation call under read-only storage may be absent from discovery or
  return a structured unavailable response.

These probes prove only that the MCP server can run in the environment where
the probe was launched. They still do not prove active-session Codex tool
exposure.

If your build reports smoke or schema diagnostics, use them as diagnostics
only. For example, `tools_list_schema_validation: passed` confirms Volicord's
MCP-visible tool schema for the effective mode; it does not prove that the
active Codex session registered those tools.

Advanced diagnostic: if the Codex host configuration format you use supports
`required = true` for an MCP server, using it for a diagnostic run can make MCP
startup failures more visible in that host. It can also prevent session startup
or resume when the server is unavailable. Do not treat `required = true` as the
default Volicord `record` profile behavior or as proof that tools are exposed.

## `failed`

Observable symptom: setup, connect, export, or verification reports `failed` or
exits with a runtime error.

Bounded recovery:

Inspect the installation profile:

```sh
volicord doctor
```

Then continue:

1. Fix the first failed setup or executable check it names.
2. Rerun the original command with `--dry-run` when the command supports it.
3. Rerun the real command only after the dry-run plan names the expected host
   and Product Repository.

Use the exact failure text to choose the next action. Do not delete Runtime Home
state or host configuration by hand unless a Reference document or human
operator has identified that as the intended recovery.

## MCP Command Is Unavailable

Observable symptom: init or verification reports that `volicord mcp --stdio`
cannot be found, launched, or initialized.

Bounded recovery:

Rerun init with the installed release binary:

```sh
volicord init --host codex --repo "<repo>" --profile record
```

If you are intentionally working from a development source checkout:

```sh
cargo build --workspace --bins
./target/debug/volicord init --host codex --repo "<repo>" --profile record
```

Complete any `action_required` command-availability or host step, then
check the installation and connection again:

```sh
volicord doctor
volicord connection verify codex --shared --repo "<repo>"
```

Init records the MCP command used by managed host configuration. Ordinary
`connection add` commands do not ask users to pass an MCP command path. If the
executable is installed somewhere init cannot discover by sibling lookup or
`PATH`, rerun init with `--mcp-command PATH`.

<a id="guard-hook-path-or-wrapper-is-unsafe"></a>
## Hook Path Or Wrapper Is Unsafe

Observable symptom: `volicord doctor`, connection status, or connection
verification reports `hook_path_safety` as a value other than `ok`, such as
`relative_path_unsafe`, `wrapper_missing`, `wrapper_not_executable`,
`absolute_path_stale`, `host_output_mismatch`, or `policy_hash_mismatch`.

Bounded recovery:

```sh
volicord doctor
volicord connection status codex --shared --repo "<repo>"
volicord init --host codex --repo "<repo>"
volicord connection verify codex --shared --repo "<repo>"
```

Use the same host and intent selector as the affected connection. For Claude
Code, replace `codex` with `claude-code` and include `--global` or `--shared`
when that is the selected connection.

Diagnostic meanings and repairs:

- `relative_path_unsafe`: the host hook config uses a bare `.codex/hooks/...`,
  `./.codex/hooks/...`, `.claude/hooks/...`, or `./.claude/hooks/...` path that
  depends on the host session cwd. Rerun `volicord init --host HOST --repo PATH`
  instead of hand-editing the hook command.
- `wrapper_missing` or `dispatch_missing`: a generated wrapper or Codex
  dispatch wrapper is missing. Rerun init for the selected Product Repository.
- `wrapper_not_executable`: a generated wrapper is present but is not
  executable on a supported Unix-like platform. Rerun init to restore the
  managed wrapper and executable bit.
- `absolute_path_stale`: a generated command still points at an old project
  root, often after moving the Product Repository. Rerun init with the current
  `--repo PATH`, then reload or restart the host when required.
- `host_output_mismatch`, `policy_hash_mismatch`, or `authority_mismatch`: the
  generated wrapper metadata no longer matches the expected host-output mode,
  policy hash, connection, or detective installation record. Rerun init so the
  managed files and registry state agree.
- `metadata_missing` or `placeholder_unsupported`: the generated configuration
  is not in the currently verified shape. Rerun init and avoid replacing the
  generated command with unsupported placeholders.

Codex detective host hook commands require the selected Product Repository to be a Git
work tree. If the wrapper stderr says the Git root could not be resolved, or
hooks fail only when the host session starts from a subdirectory, confirm that
the session is inside the intended Git work tree and that `git` is available to
the host process, then rerun init for that repository. Claude Code detective host hook
commands are rooted at `${CLAUDE_PROJECT_DIR}`; if the host does not provide
that project directory, reload or repair the host configuration through the
host's own trust and project-selection flow.

Unsafe hook paths keep detective host hooks inactive. Watcher availability is
reported separately in the observation summary. Path repair is still
separate from host trust, approval, restart, and reload; complete any reported
host-owned action and rerun verification after repair.

## Shared Connection Needs Host Approval

Observable symptom: a shared connection writes or updates a project integration
file, but the host still does not load Volicord tools.

Bounded recovery:

```sh
volicord connection status codex --shared
volicord connection verify codex --shared
```

Complete the host-owned project approval or reload action named by the command.
The `Product Repository` integration file is not Volicord authority and does not
prove that the host loaded, trusted, approved, or exposed the MCP server.

## Generic Host Does Not Show Volicord Tools

Observable symptom: an external MCP host with user-managed configuration does
not show Volicord tools.

Bounded recovery:

```sh
volicord doctor
volicord connection status codex --repo "<repo>"
```

Then inspect the external host's own configuration process. Its entry should
start `volicord mcp --stdio --connection <connection_id> [--project
<project_id>]` for an existing Agent Connection. The external configuration is
user-managed.

## Removal Completed Only Partially

Observable symptom: `volicord connection remove ...` reports that host
configuration could not be removed, or a connection still appears for another
Product Repository.

Bounded recovery:

```sh
volicord connection remove codex --dry-run
volicord connection status codex
volicord connection list
```

Removal first removes the selected Product Repository membership. It removes the Agent
Connection and managed host configuration only when no owned membership remains
and safety checks permit it. It must not remove the `Product Repository`,
project state, Volicord records, evidence attachment storage, or unrelated host
entries.

## Security Limits

Volicord setup and verification are local diagnostics. They do not prove that an
external host is secure, that a model will use Volicord tools, or that file
writes are safe. For exact security wording, use [Security](../reference/security.md).
