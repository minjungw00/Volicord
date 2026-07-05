# Agent Host Setup

Use this guide to connect Codex, Claude Code, or a generic MCP host to
Volicord. The ordinary first-run path starts with `volicord init`, a host, a
Product Repository, and the integration profile that matches the host capabilities;
Volicord manages the internal host and registry values.

Exact CLI behavior belongs to
[Administrative CLI Reference](../reference/admin-cli.md). Agent Connection
meaning belongs to [Agent Connection Reference](../reference/agent-connection.md),
and runtime/file boundaries belong to
[Runtime Boundaries](../reference/runtime-boundaries.md).

## Setup Sequence

Install `volicord` first with [Installation](../user-guide/installation.md),
then run the host setup sequence:

```sh
volicord init --host codex --repo /path/to/your-product-repo --profile record
```

`/path/to/your-product-repo` is an example path for the Product Repository where
you want the agent to work. `volicord init` creates or reuses the Runtime Home
and installation profile when needed, registers or reuses that repository
project, derives the visible project name from the repository directory,
installs project-scoped MCP configuration for the selected host, writes
project-scoped Volicord guidance and local setup files, records integration
status, and stores internal registry identities in the selected
`Volicord Runtime Home`. Generated host configuration starts
`volicord mcp --stdio`. `--profile record` does not require host lifecycle hook
installation or a session watcher.

After init, complete the host-owned follow-up outside the terminal:

- For Codex project-scoped setup, open, restart, or reload Codex in the Product
  Repository and trust or approve the project configuration if Codex asks.
- For Claude Code project-scoped setup, open, restart, or reload Claude Code in
  the Product Repository and approve the project MCP entry, workspace, or
  project configuration if Claude Code asks.

Writing repo-local configuration is not the same as proving that an already
running host loaded, trusted, or approved it. Init can write
`.codex/config.toml` or `.mcp.json`, `.volicord/policy.json`, and the managed
`AGENTS.md` guidance
while the host still controls reload, restart, trust, and approval. Local
Volicord state is stored in the Runtime Home, separate from those Product
Repository files. CLI MCP preflight or handshake success means Volicord's MCP
server can start and respond from the terminal-side check path; it does not by
itself prove that Codex, Claude Code, or another host has loaded, trusted, or
approved the project configuration.

After completing any host prompt, use the terminal-side follow-up check:

```sh
volicord connection verify codex --shared --repo /path/to/your-product-repo
```

Use `claude-code` instead of `codex` for Claude Code.

Use `volicord connection add` for lower-level connection variants after the
installation profile is ready, for example when selecting personal, global, or
read-only behavior directly. Use `--repo PATH` only when the process current
directory is not the target Product Repository:

```sh
volicord connection add codex --repo /path/to/your-product-repo
```

## Integration Profiles

Detective status reports the selected profile and an observation summary for the
selected connection or session:

| Profile | How it is reached | Operational meaning |
|---|---|---|
| Record profile (`record`) | Cooperative Volicord workflow recording through MCP is available without requiring host hooks or a session watcher. | Generated setup guidance can steer the host but cannot force it. |
| Detective profile (`detective`) | Project-local host hooks have verified generated config, cwd-independent and subdirectory-safe hook commands, native host output, required phases, write matchers, matching policy hash, runtime observation, and session watcher observation. | Cooperative host warning or denial decision signals, post-tool correlation, chat command capture, detective status, Unrecorded Changes, and close/write blockers can participate in the workflow. |

The Record profile can issue Volicord Write Tickets through the prepare-write
workflow. It does not provide OS sandboxing, network isolation, malware
defense, full write prevention, actor identity proof, correctness proof, test
sufficiency proof, or human review completion. The Detective profile does not
make Write Tickets into filesystem enforcement, code review approval, final
acceptance, or proof that a write occurred; it adds supported hook and watcher
observations that can later be correlated with ticket-scoped writes and
Unrecorded Changes.

The observation summary reports whether host hooks and the session watcher
are active, whether cooperative pre-tool warning or denial is available, whether
unrecorded changes can be detected, whether actor identity can be proven, and
whether OS enforcement is provided. Current Volicord output reports no actor
identity proof and no OS enforcement.

## Detective Lifecycle

In detective profile, setup and activation are separate. `volicord init` installs or
updates MCP host configuration, Volicord-managed `AGENTS.md` guidance,
`.volicord/policy.json`, host hook or rule files, and detective installation state
for host-hook observation.
The host may still need reload, restart, trust, project MCP approval, or another
host-owned action before those files run.

Current verified detective adapters are host-specific:

- Codex detective setup writes project MCP configuration, Volicord-managed POSIX
  `sh` wrapper scripts under `.codex/hooks/`, `.codex/hooks.json`, and
  `.codex/rules/*.rules`. Pre-tool and post-tool matchers cover `Bash`,
  `apply_patch`, `Edit`, `Write`, and
  `mcp__.*__(write|edit|create|update|delete|remove|move|patch).*` tool names.
  The host may require project trust, hook trust, and restart or reload before
  the generated rule and hook files run.
- Claude Code detective setup writes `.mcp.json`, Volicord-managed POSIX `sh`
  wrapper scripts under `.claude/hooks/`, `.claude/settings.json`, and
  `.claude/rules/*.md`. Pre-tool and post-tool matchers cover `Bash`, `Edit`,
  `Write`, `MultiEdit`, and
  `mcp__.*__(write|edit|create|update|delete|remove|move|patch).*` tool names.
  Settings writes preserve unrelated settings and merge Volicord-managed
  entries; the host may require project MCP approval, workspace trust, and
  settings reload before the generated hook and rule files run.

Generated hook commands are cwd-independent and subdirectory-safe when
verification reports `hook_path_safety=ok`. Codex hook entries do not execute a
bare `.codex/hooks/...` path; they run a POSIX `sh` command that resolves the
Git work-tree root with `git rev-parse --show-toplevel` and then execs the
Volicord-managed dispatch wrapper under that root. The dispatch wrapper checks
that the phase wrapper exists and is executable before execing it. Claude Code
hook entries use exec-form commands rooted at `${CLAUDE_PROJECT_DIR}`, such as
`${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-pre-tool.sh`, with no args. Do
not replace generated commands with bare `.codex/hooks/...` or
`.claude/hooks/...` relative paths, because those paths depend on the host
session cwd and are reported as `relative_path_unsafe`.

Generated hook configs invoke the wrapper scripts with `--host-output codex` or
`--host-output claude-code`, so hook stdout is host-native JSON/context or empty
output, not Volicord wrapper JSON.

Detective init must be able to install and verify all required host
lifecycle hook phases. When the selected Codex or Claude Code adapter does not
know a reliable project-local hook schema or path for every required phase, init
fails instead of treating `AGENTS.md` or `.volicord/policy.json` as enforcement.
On native Windows, detective init fails with `DETECTIVE_WINDOWS_UNSUPPORTED`
because Windows host-hook wrappers and session watcher behavior are not
implemented and tested. Use `--profile record` on native Windows.
If detective prerequisites are unavailable, use `--profile record` or prepare a
supported host, platform, and repository configuration before rerunning init:

```sh
volicord init --host codex --repo /path/to/your-product-repo --profile record
```

`volicord connection verify` and `volicord doctor` keep file health, required
host action, observed activation, and observation facts separate. The
detective installation state becomes active only after Volicord observes a
matching host-hook event for the recorded project, Agent Connection, host kind,
integration profile, and policy hash. Hook path safety does not replace host
trust, reload, restart, or approval. `AGENTS.md` is instruction support, and
host hooks or rules are cooperative and detective guardrails; they are not OS
sandboxing, command isolation, network isolation, actor proof, or proof that
writes cannot happen outside Volicord-aware paths.

## Connection Intents

Connection intent describes where the host configuration belongs:

| Intent | Command shape | Host support |
|---|---|---|
| `personal` | `volicord connection add codex` or `volicord connection add claude-code` | Local setup for the current user. |
| `shared` | `volicord connection add codex --shared` or `volicord connection add claude-code --shared` | Project-shared configuration stored through an explicit integration file when the host supports it. |
| `global` | `volicord connection add claude-code --global` | User-wide host configuration for hosts that support it. |

`--shared` and `--global` are mutually exclusive. When neither is present,
Volicord uses `personal`.

## Workflow And Read-Only Mode

The default mode is `workflow`. Use `--read-only` for a connection that should
expose read-oriented behavior instead of workflow tools:

```sh
volicord connection add codex --read-only
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
volicord connection add codex --dry-run
volicord connection add claude-code --shared --dry-run
volicord connection remove codex --dry-run
```

Use dry run before changing shared host configuration or before removing a
connection whose host target you want to inspect first.

## Inspect And Verify

```sh
volicord connection list
volicord connection status codex --shared --repo /path/to/your-product-repo
volicord connection verify codex --shared --repo /path/to/your-product-repo
```

Default text output is a compact human summary for interactive setup work. For
connection status and verification, read `Status`, `Checks`, `Next`, and
`Diagnostics` first. Use `--json` for automation and full diagnostics; scripts
must not parse the compact text. Detailed guard state, hook diagnostics, MCP
handshake details, and host observations belong in JSON diagnostics.

`volicord connection status codex --shared --repo /path/to/your-product-repo`
has this compact shape:

```text
Agent Connection status for Codex

Status:
  Connection: enabled
  Mode: workflow
  Last verification: action required

Profile:
  record

Repository:
  /path/to/your-product-repo

Checks:
  Stored connection: enabled, mode workflow, last verification action required
  Current MCP configuration: match
  Last MCP preflight: passed
  Last MCP handshake: passed
  Host follow-up: action required

Next:
  1. Open, restart, or reload Codex in this repository.
  2. Trust or approve the project configuration if Codex asks.
  3. Run:
     volicord connection verify codex --shared --repo /path/to/your-product-repo

Limits:
  The record profile supports cooperative Volicord workflow recording through MCP.
  It does not provide OS sandboxing, network isolation, malware defense,
  full write prevention, actor identity proof, correctness proof, test
  sufficiency proof, or human review completion.

Diagnostics:
  Run:
    volicord connection status codex --shared --repo /path/to/your-product-repo --json
```

`volicord connection verify codex --shared --repo /path/to/your-product-repo`
uses the same section model while showing fresh verification checks:

```text
Agent Connection checked for Codex

Status:
  Verification: action required
  Connection: enabled
  Mode: workflow

Profile:
  record

Repository:
  /path/to/your-product-repo

Checks:
  MCP configuration: match
  MCP preflight: passed
  MCP handshake: passed
  Host follow-up: action required

Next:
  1. Trust or approve the project configuration if Codex asks.
  2. Run:
     volicord connection verify codex --shared --repo /path/to/your-product-repo

Limits:
  The record profile supports cooperative Volicord workflow recording through MCP.
  It does not provide OS sandboxing, network isolation, malware defense,
  full write prevention, actor identity proof, correctness proof, test
  sufficiency proof, or human review completion.

Diagnostics:
  Run:
    volicord connection status codex --shared --repo /path/to/your-product-repo --json
```

If more than one connection matches the same host and repository, include the
same intent flag used to select it:

```sh
volicord connection status codex --shared
volicord connection verify claude-code --global
```

Result states:

| State | Meaning in setup guidance |
|---|---|
| `complete` | Volicord-side state, managed host configuration, observable MCP startup, initialization, and expected tool exposure are ready. |
| `action_required` | Volicord-side state exists, but a named user-controlled host action remains. It is not a fatal CLI error by itself. |
| `failed` | A required local prerequisite, host configuration step, or verification step did not succeed. |
| `dry_run` | The command reported planned actions without persistent changes. |

## Generic MCP Host Configuration

For an MCP host that Volicord does not manage directly, first create a supported
Agent Connection, then configure the external host through its own settings to
start `volicord mcp --stdio --connection <connection_id> [--project
<project_id>]`. Use `volicord connection mode` when the connection should be
read-only. The external host configuration remains user-managed.

## User Channel Boundary

Agent Connections can request or display focused judgment needs. They do not
record authority-bearing user answers. Use the local `User Channel` commands
when a shown option must become the user's recorded judgment:

```sh
volicord inbox
volicord inbox answer JUDGMENT_ID --choice CHOICE_ID
```

## Removal

Remove the selected Product Repository from a connection:

```sh
volicord connection remove codex --dry-run
volicord connection remove codex
```

Removal deletes only matching managed host configuration when ownership and
safety checks permit it. It does not delete the `Product Repository`, Runtime
Home, project registration, project state, Volicord records, evidence attachment
storage, or unrelated host configuration.

## Troubleshooting Routes

| Symptom | Next document |
|---|---|
| Installation profile, executable, or Product Repository detection is not ready. | [Installation](../user-guide/installation.md) |
| Connection reports `action_required` or `failed`. | [Agent Host Troubleshooting](agent-host-troubleshooting.md) |
| Exact command behavior is unclear. | [Administrative CLI Reference](../reference/admin-cli.md) |
| Runtime Home and Product Repository boundaries matter. | [Runtime Boundaries](../reference/runtime-boundaries.md) |
