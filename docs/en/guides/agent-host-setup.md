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

Install `volicord` first with [Installation](../getting-started/installation.md),
then run the host setup sequence:

```sh
volicord init --host codex --repo /path/to/your-product-repo --profile record
volicord connection status codex --repo /path/to/your-product-repo
```

`/path/to/your-product-repo` is an example path for the Product Repository where
you want the agent to work. `volicord init` creates or reuses the Runtime Home
and installation profile when needed, registers or reuses that repository
project, derives the visible project name from the repository directory,
installs project-scoped MCP configuration for the selected host, writes
Volicord-managed guidance and policy metadata, records integration
status, and stores internal registry identities in the selected
`Volicord Runtime Home`. Generated host configuration starts
`volicord mcp --stdio`. `--profile record` does not require host lifecycle hook
installation or a session watcher.

Use `volicord connect` for lower-level connection variants after the
installation profile is ready, for example when selecting personal, global, or
read-only behavior directly. Use `--repo PATH` only when the process current
directory is not the target Product Repository:

```sh
volicord connect codex --repo /path/to/your-product-repo
```

## Integration Profiles

Detective status reports the selected profile and an observation summary for the
selected connection or session:

| Profile | How it is reached | Operational meaning |
|---|---|---|
| Record profile (`record`) | MCP tools and authority records are available without requiring host hooks or a session watcher. | Setup guidance and policy metadata can steer the host but cannot force it. |
| Detective profile (`detective`) | Project-local host hooks have verified generated config, cwd-independent and subdirectory-safe hook commands, native host output, required phases, write matchers, matching policy hash, runtime observation, and session watcher observation. | Cooperative host warning or denial decision signals, post-tool correlation, chat command capture, detective status, Unrecorded Changes, and close/write blockers can participate in the workflow. |

The Record profile can issue Volicord Write Tickets through the prepare-write
workflow. The Detective profile does not make Write Tickets into filesystem
enforcement, code review approval, final acceptance, or proof that a write
occurred; it adds supported hook and watcher observations that can later be
correlated with ticket-scoped writes and Unrecorded Changes.

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
volicord connection status codex --repo /path/to/your-product-repo
volicord connection verify codex --repo /path/to/your-product-repo
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
| Installation profile, executable, or Product Repository detection is not ready. | [Installation](../getting-started/installation.md) |
| Connection reports `action_required` or `failed`. | [Agent Host Troubleshooting](agent-host-troubleshooting.md) |
| Exact command behavior is unclear. | [Administrative CLI Reference](../reference/admin-cli.md) |
| Runtime Home and Product Repository boundaries matter. | [Runtime Boundaries](../reference/runtime-boundaries.md) |
