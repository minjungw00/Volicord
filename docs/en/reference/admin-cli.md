# Administrative CLI reference

This document owns the local `volicord` administrative and bootstrap CLI
contract. The CLI establishes the `Volicord Runtime Home`, registers projects
from repository roots, manages Agent Connections without requiring users to
handle internal identities, provides the local `User Channel` command path,
exports generic MCP configuration, provides local guard hook commands, and
reports setup or connection diagnostics.
These commands are not public Volicord API methods.

It does not define public API method behavior, API schemas, storage record
layout, security guarantees, Core authority semantics, or MCP stdio transport
behavior.

## Owns / does not own

This document owns:

- `volicord` command names, command-line arguments, defaults, stdout/stderr
  routing, and process exit codes
- Runtime Home, installation profile, executable-link, and MCP command
  selection during `init` or `setup`
- repository-root project detection and administrative project commands
- Agent Connection command behavior for supported host integrations
- generic MCP config export behavior
- local serve command names, command-line arguments, defaults, stdout/stderr
  routing, and startup exit codes
- local `volicord guard` lifecycle command names, options, decisions, output,
  and event-recording behavior
- local `volicord changes` recovery command names and output
- local `User Channel` command names and command output
- diagnostic status, required user actions, dry-run behavior, JSON output, and
  noninteractive behavior
- the boundary between administrative commands, local `User Channel` commands,
  and public Volicord API methods

This document does not own:

- public Volicord API methods; see [API Methods](api/methods.md)
- Agent Connection, Connection Projects, connection mode, connection intent,
  and actor provenance meanings; see [Agent Connection](agent-connection.md)
- runtime data boundary meaning and `Product Repository` file-boundary
  exceptions; see [Runtime Boundaries](runtime-boundaries.md)
- MCP process startup, stdio and HTTP framing, wire behavior, response wrapping, and
  shutdown; see [MCP Transport](mcp-transport.md)
- external host hook protocol schemas and host-specific response semantics
- storage record layout, SQLite DDL, general storage migration definitions,
  Core authority semantics, and security guarantee meanings

## Command model

`volicord` is a local administrative/bootstrap executable. It is not a general
long-running server. The explicit `volicord serve` command is limited to the
local MCP transport process described in [MCP Transport](mcp-transport.md). The
`volicord user` command group is the local
`User Channel` CLI adapter over selected Core methods; its command names remain
administrative CLI commands, not public Volicord API methods.

Supported baseline commands:

```text
volicord --help
volicord --version
volicord init --host codex|claude-code --repo PATH [--mode mcp-only|guarded|managed] [--allow-degraded] [--home PATH] [--mcp-command PATH] [--dry-run] [--json]
volicord setup [--home PATH] [--link-bin PATH] [--mcp-command PATH] [--json]
volicord doctor [--json]
volicord connect [HOST] [--repo PATH] [--shared|--global] [--read-only] [--dry-run] [--json]
volicord connections [--repo PATH] [--json]
volicord connection status [HOST] [--repo PATH] [--shared|--global] [--json]
volicord connection verify [HOST] [--repo PATH] [--shared|--global] [--json]
volicord connection mode [HOST] workflow|read-only [--repo PATH] [--shared|--global] [--json]
volicord connection remove [HOST] [--repo PATH] [--shared|--global] [--dry-run] [--json]
volicord project use [PATH] [--json]
volicord project current [--json]
volicord project list [--json]
volicord project rename NAME [--repo PATH] [--json]
volicord project forget [PATH|NAME] [--json]
volicord export mcp-config [--output PATH] [--repo PATH] [--read-only] [--json]
volicord serve --transport streamable-http [--listen 127.0.0.1:8765] [--home PATH] [--connection <connection_id>] [--project PATH]... [--token TOKEN | --generate-token] [--allow-origin ORIGIN] [--allow-nonlocal-listen]
volicord guard session-start [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--guard-mode MODE] [--text]
volicord guard pre-tool [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--guard-mode MODE] [--text]
volicord guard post-tool [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--guard-mode MODE] [--text]
volicord guard prompt-capture [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--guard-mode MODE] [--text]
volicord guard stop [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--guard-mode MODE] [--text]
volicord changes reconcile [--repo PATH] [--task active|ID] [--json]
volicord user status [--repo PATH] [--task active|ID] [--json]
volicord user judgments [--repo PATH] [--task active|ID] [--json]
volicord user judgment show INDEX_OR_ID [--repo PATH] [--json]
volicord user judgment answer INDEX_OR_ID OPTION_INDEX_OR_ID [--repo PATH] [--note TEXT] [--json]
```

Supported `HOST` values are `codex` and `claude-code`. When `HOST` is omitted,
the command may use an unambiguous current host context. If the host cannot be
identified unambiguously, the command fails with a diagnostic action that names
the supported host values.

Exit and stream behavior:

- Successful commands write success output to stdout and exit with code `0`.
- `action_required` is a successful administrative result and exits `0`.
- `failed`, runtime errors, storage errors, verification failures, and
  conflicts exit `1`.
- Usage errors write diagnostics to stderr and exit with code `2`.
- `volicord --version` writes `volicord <version>` to stdout and does not
  require Runtime Home resolution.
- `--json` writes exactly one JSON document to stdout and does not mix human
  explanation into stdout.
- `volicord guard` writes JSON by default. Its `deny` decision exits `1`;
  `allow`, `warn`, and `inject_context` exit `0`.
- Errors remain stderr diagnostics under the CLI exit-code model.
- `volicord serve --transport streamable-http` is an explicit long-running MCP
  transport process. It keeps loopback as the default listener, requires bearer
  authentication, and delegates HTTP wire behavior and transport security checks
  to [MCP Transport](mcp-transport.md).

Not supported:

- The CLI has no general-purpose `server` or daemon command.
- `volicord serve` must not be treated as a public Volicord API service or an
  unauthenticated network service.
- Administrative commands are not public Volicord API methods and must not be
  added to the public method list.
- Guard commands are cooperative and detective hook commands, not OS-level
  sandboxing or a security-enforcement proof.
- Text-mode user flows must not require users to type `project_internal_id`,
  `connection_internal_id`, host config keys, protocol envelopes, or stored
  registry fields.

<a id="runtime-home-selection"></a>
## Setup and Runtime Home

`volicord setup` establishes or repairs the local installation profile without
connecting a repository. It creates or verifies the selected Runtime Home and
stores the command paths later administrative, Agent Connection, export, and
MCP process flows use. Setup is the standalone installation-profile command,
not the ordinary first-run repository path. `volicord init` is the
primary first-run path and may also select the Runtime Home path or MCP launch
command while performing repository setup and host connection. Setup can help
make `volicord` available on `PATH`, but it cannot change the parent shell's
current environment.

In text mode, `volicord setup` may prompt only when stdin and stdout are
interactive terminals, `--json` is absent, and `--link-bin` is absent. It
prompts only when the selected command paths are not ready on `PATH`. In
noninteractive conditions, JSON mode, or explicit `--link-bin` mode, setup must
report actions instead of prompting.

The top-level setup status answers whether installation-profile preparation
still needs a named user action. Setup may report `action_required` after
saving the Runtime Home and installation profile when selected commands are not
ready for future `PATH` lookup by shells or agent hosts. Setup output must keep
command-availability details and required actions explicit.

Arguments:

| Argument | Meaning |
|---|---|
| `--home PATH` | Selects the `Volicord Runtime Home`. Omission uses the platform default local runtime location. The selected path must satisfy the Runtime Home/Product Repository separation contract before project state is used. |
| `--link-bin PATH` | Creates the directory if needed, verifies it is writable, then creates or updates a command link for `volicord` there when feasible. The command reports the target path, refuses unsafe replacement, and does not by itself edit shell startup files or the parent shell `PATH`. |
| `--mcp-command PATH` | Stores the exact `volicord` command that managed host configuration and generic exports should use before the `mcp --stdio --connection <connection_id>` arguments. Omission uses the running `volicord` executable selected by setup. |
| `--json` | Selects machine-readable, noninteractive output. Setup does not prompt in JSON mode. |

Setup effects:

- creates or validates the Runtime Home registry
- records Runtime Home identity and installation profile metadata
- records the selected `volicord` command location and MCP launch command for
  later `init`, `connect`, `doctor`, export, and MCP startup flows
- inspects whether selected command paths resolve through the current process
  `PATH`
- may prompt in interactive text mode for safe command-availability choices:
  create command links in an existing setup-suggested directory whose
  writability was verified, create links after creating a missing conventional
  user command directory such as `~/.local/bin` under `HOME` when setup can
  safely create it and verifies writability after creation, write an approved
  shell startup `PATH` block, print a shell command, or skip linking
- may update the `volicord` command link named by `--link-bin` or selected
  through the interactive prompt
- may write a managed shell startup `PATH` block only after explicit
  interactive approval
- reports a `PATH` action when a link directory is not visible to the current
  process; existing shells and agent host processes may need restart or reload
- does not offer arbitrary missing paths as automatic interactive command-link
  choices; use `--link-bin PATH` for an explicit missing directory
- does not register a project unless a separate project or connection command
  selects a repository
- does not create a public Volicord API method or record a user-owned judgment

On Unix, interactive shell startup updates are supported for `bash`, `zsh`, and
`sh` when setup can identify `HOME` and `SHELL`. The target files are
`~/.bashrc`, `~/.zshrc`, and `~/.profile` respectively. Setup writes or updates
a Volicord-managed block in that file after the user approves the exact block.
Unsupported shells, unsupported platforms, missing environment variables, or
write failures leave a manual `PATH` action instead.

`volicord doctor` is the read-oriented diagnostic command for the installation
profile. Its top-level status answers whether the current installation profile
is usable. It verifies Runtime Home access, registry schema, installation
profile presence, stored command readiness, command availability through
`PATH`, and command-link or shim readiness when link metadata is present. When
stored command paths are executable, doctor may report `complete` while
reporting command-availability warnings and `actions_recommended` for future
shells or agent hosts. PATH or command-link recommendations must say when
existing agent hosts may need restart or reload. Doctor reports supported host
detection as a connection-verification concern. When guard installation records
exist, doctor may also report guard file installation, configuration health,
runtime hook observation health, effective guard health, and host reload
requirement as diagnostics. These guard diagnostics are local setup and
observation checks; they are not proof of OS enforcement, sandboxing, write
prevention, product correctness, or close
readiness. Doctor does not create projects, install host configuration, change
connection mode, or answer user judgments.

<a id="project-commands"></a>
## Project commands

Project commands use repository roots as the user-facing project identity.
Internal project identity is storage and provenance data; text-mode commands do
not require it.

Repository root detection:

- `--repo PATH` and `PATH` arguments are resolved before project lookup.
- When no path is supplied, commands use the process current working directory.
- The detected repository root is the nearest supported repository root
  containing the selected path. If no root can be detected, commands that need a
  project fail with a diagnostic action naming `volicord project use PATH`.
- Runtime Home and `Product Repository` paths must satisfy the
  [Runtime Home/Product Repository separation contract](runtime-boundaries.md#runtime-home-product-repository-separation).

`volicord project use [PATH]` registers or reuses the detected repository root.
Registration creates a `project_internal_id`, a user-facing project name, a
project home under the Runtime Home, and project-local state as needed. The
default project name is derived from the repository directory and made unique
inside the Runtime Home when needed.

`volicord project current` reports the project detected from the current working
directory. It does not create a project registration.

`volicord project list` lists registered projects by user-facing name, repository
root, status, and diagnostic availability.

`volicord project rename NAME [--repo PATH]` changes the user-facing project
name for the selected repository. It does not change `project_internal_id`,
repository root, project home, or Core state.

`volicord project forget [PATH|NAME]` removes the selected project registration
only when doing so does not orphan active Agent Connection membership or project
state that an owner requires to remain addressable. Forgetting a project must
not delete the `Product Repository`, unrelated Runtime Home data, host
configuration, artifact storage owned by another remaining registration, or
Core authority rows that must be preserved.

<a id="connection-intents-and-hosts"></a>
## Connection intents and hosts

Agent Connection setup uses connection intents instead of low-level host config
scope names:

| Intent | Selected by | Meaning |
|---|---|---|
| `personal` | default | User-owned host configuration for the current user's ordinary local flow. |
| `shared` | `--shared` | Project-owned or project-shared host configuration stored as an explicit integration file in the selected `Product Repository`. |
| `global` | `--global` | User-wide host configuration for the selected host, with project access still constrained by registered repository roots and Connection Projects. |

`--shared` and `--global` are mutually exclusive. When neither is present, the
intent is `personal`.

Connection modes:

- `workflow` is the default mode.
- `read-only` is explicit and exposes only read/project-discovery behavior
  through the Agent Connection.
- `volicord connection mode ... workflow|read-only` changes the stored mode for
  the selected connection without requiring users to edit host configuration.

The internal host configuration key `server_name` defaults to `volicord`.
Ordinary CLI flows do not expose a server-name option. A generated host
configuration may contain a `connection_id` process-binding value derived from
the stored `connection_internal_id`, server name, and command arguments so that
the host can start `volicord mcp --stdio`; those values are saved
process-binding details, not user authority tokens. Text-mode command input
uses the selected host, intent, and repository root instead.

Ordinary `volicord connect` commands use the saved profile in the resolved
Runtime Home instead of asking for an MCP command path or Runtime Home path.
Personal, local, or user-wide host configuration may carry that Runtime Home as
`VOLICORD_HOME`. Shared project host configuration must not embed a personal
Runtime Home path; it uses `volicord` as the command name and
`mcp --stdio --connection <connection_id>` as arguments that the future host
environment must resolve through `PATH`.

<a id="agent-host-setup-and-init"></a>
`volicord init --host codex --repo PATH --mode mcp-only` and
`volicord init --host claude-code --repo PATH --mode mcp-only` are the primary
lower-guarantee first-run repository setup and host-connection examples for
chat-first use when required hook support is not being installed. Init uses the
shared, project-scoped host layout so generated host MCP configuration starts
`volicord mcp --stdio` through `PATH` and does not embed a personal Runtime Home
path.

`--mode` selects the guard integration level:

- `mcp-only` writes MCP configuration, the managed `AGENTS.md` guidance block,
  and policy metadata with guard commands disabled. It records guard
  installation status without requiring guard activation.
- `guarded` is the default. It writes MCP configuration, the managed
  `AGENTS.md` guidance block, `.volicord/policy.json` guard command policy, and
  supported project-local host hook and rule files.
- `managed` requires a verified managed distribution source that is distinct
  from ordinary project-local configuration, such as a host-supported plugin,
  managed configuration bundle, or managed policy layer recorded in Volicord
  host contract data. If the selected host has no verified managed distribution
  contract, init fails with `MANAGED_MODE_UNSUPPORTED` and does not generate
  project-local guarded files as a managed substitute.

Full `guarded` initialization requires the selected host adapter to declare and
verify support for every required lifecycle hook:
`session-start`, `pre-tool`, `post-tool`, `prompt-capture`, and `stop`.
`AGENTS.md` and `.volicord/policy.json` are not host hook configuration. If the
adapter does not know a reliable project-local hook schema or path for every
required phase, init fails with `GUARDED_HOOKS_UNSUPPORTED` unless the caller
passes `--allow-degraded`. The explicit degraded opt-in may write MCP
configuration, guidance, policy, and supported hook or rule files, but it records
degraded guard status and reports missing required hook phases in human and JSON
output. `mcp-only` does not require hook installation.

Managed initialization must satisfy the guarded hook requirements and the
separate managed distribution requirement. For hosts without a verified managed
contract, `--allow-degraded` is reported as not applied and does not silently
turn `managed` into `guarded` or `mcp-only`.

For `guarded`, init records `reload_required` when the host still needs restart
or reload to load generated guard hooks, and `configured` when files are
installed but no matching guard hook has been observed. Init does not mark a
guard installation `active` merely because files were written.

`--home PATH` selects the Runtime Home for this initialization. `--mcp-command
PATH` stores the exact command path in the installation profile when init must
create or update that profile; project-scoped host MCP configuration still uses
`volicord` from `PATH`.

Non-dry-run `volicord init`:

- initializes the Runtime Home if it is missing
- creates or updates the installation profile when needed
- registers or reuses the selected `Product Repository`
- creates or updates the matching Agent Connection and Connection Projects
  membership
- writes project-scoped Codex `.codex/config.toml` or Claude Code `.mcp.json`
  with `volicord mcp --stdio --connection <connection_id>`
- writes or updates only the Volicord-managed block in `AGENTS.md`
- writes `.volicord/policy.json` with guard commands that invoke
  `volicord guard`
- writes supported host hook files such as `.codex/hooks.json` or
  `.claude/settings.json`
- writes supported host rule files such as `.claude/rules/volicord.md`
- records guard installation status in the Runtime Home registry
- rejects non-`mcp-only` guarded initialization when required host hook
  configuration is missing unless `--allow-degraded` was explicitly supplied
- reports the required host restart, reload, approval, or trust action when the
  host must load the new MCP or guard configuration

Re-running init is idempotent for matching Volicord-managed content. It updates
managed blocks, policy files, host MCP entries, and guard installation rows
without duplicating them. If an existing target contains unmanaged content where
Volicord requires ownership markers or a managed fingerprint, init must report a
conflict instead of overwriting it.

<a id="volicord-agent-install"></a>
## Agent Connection commands

Connection selection uses host, intent, and repository root. When no intent
flag is present and a repository is selected, status, verify, mode, and remove
select the single matching connection for that host and repository across
intents. If more than one connection matches, the command reports an ambiguous
selector and the caller must add the matching intent flag. The command derives
or looks up the stored `connection_internal_id`.

| Command | Runtime Home registry effect | Host configuration effect | Verification effect |
|---|---|---|---|
| `volicord init` | Initializes Runtime Home and installation profile if needed, registers or reuses the selected repository project, creates or updates the shared project-scoped Agent Connection, ensures Connection Projects membership, and records guard installation status. | Installs or updates managed project-local MCP configuration, `AGENTS.md` guidance, `.volicord/policy.json`, and supported host hook and rule files for `codex` or `claude-code`. | Runs host-config, MCP startup, initialization, and `tools/list` checks where observable, then reports any host reload, restart, trust, or approval action. |
| `volicord connect` | Registers or reuses the selected repository project, creates or updates the matching Agent Connection, records the connection intent and mode, and ensures the project is in Connection Projects. | Installs or updates managed host configuration for `codex` or `claude-code` according to the selected intent. | Runs host-config, MCP startup, initialization, and `tools/list` checks where observable. |
| `volicord connections` | Reads matching Agent Connections and connected projects. | Does not launch the host and does not rewrite host configuration. | Reports stored and diagnostic verification state without refreshing host checks. |
| `volicord connection status` | Reads one selected Agent Connection. | Does not launch the host and does not rewrite host configuration. | Reports full stored verification status and required user actions. |
| `volicord connection verify` | Reads the selected Agent Connection and updates last-known verification status. | Inspects the managed target when the host integration owns an observable target. | Runs the observable checks and stores the resulting verification state. |
| `volicord connection mode` | Updates the selected connection mode. | Does not rewrite host configuration unless the host entry must be regenerated to reflect the mode. | Reports diagnostics after the mode change. |
| `volicord connection remove` | Removes selected Connection Projects membership and removes the Agent Connection when no owned membership remains. | Removes only matching managed host configuration when ownership and safety checks permit removal. | Does not delete projects, Core state, Runtime Home, artifact storage, or unrelated host configuration. |

Rules:

- `volicord connect` must never connect every project in the Runtime Home by
  default.
- A selected project is always resolved from a repository root and registered
  automatically when the command needs a durable project registration.
- Shared intent may write only explicit integration files allowed by
  [Runtime Boundaries](runtime-boundaries.md#explicit-integration-files-in-product-repositories).
- Existing unmanaged host configuration for the same generated host target is a
  conflict. Matching Volicord-managed content may be updated or removed only by
  the owning command.
- Host trust, project trust, project MCP approval, OAuth, restart, reload, and
  comparable host-controlled actions remain user-controlled host actions.

<a id="agent-connection-result-states"></a>
<a id="agent-setup-result-states"></a>
## Connection result states

Agent Connection commands use these result states:

| State | Meaning |
|---|---|
| `not_verified` | No verification result is currently recorded for the selected Agent Connection. This is not proof that the host failed. |
| `complete` | Durable Agent Connection state exists, managed host configuration exists and matches the expected managed fingerprint, required host loadability and trust gates are satisfied, MCP startup succeeds, MCP initialization succeeds, and `tools/list` exposes the required tools for the mode. |
| `action_required` | Durable Agent Connection state and host configuration are present, but host trust, project approval, OAuth, reload, restart, command-link repair, installation-profile repair, or a comparable user-controlled action remains. |
| `failed` | The requested command or verification did not establish usable durable Agent Connection state, usable host configuration, or a required local prerequisite. |
| `dry_run` | The command reported the planned actions without persistent changes. |

Verification output must make checks and user actions first-class diagnostics.
Text output must show the overall status, each check that was attempted or
blocked, and the next user action when one is required. JSON output must include
top-level `status`, `checks`, and `actions` fields for diagnostic consumers.
Connection status and verification output must keep guard file installation,
configuration health, runtime hook observation health, effective guard health,
host reload requirement, prompt-capture availability, and last guard event when
known as separate diagnostics. Files installed or configured must not be
reported as an active observed guard hook.

A successful `volicord mcp --check` startup check alone must not be described as a
`complete` Agent Connection. It is startup validation for the MCP process only.

<a id="generic-mcp-config-export"></a>
## Generic MCP config export

`volicord export mcp-config [--output PATH] [--repo PATH] [--read-only] [--json]`
exports host-neutral MCP configuration. It is a separate export flow, not a
normal host connection intent.

Rules:

- The command resolves or registers the selected repository project by root.
- It uses the installation profile's stored MCP command unless the setup is invalid,
  in which case it reports an `action_required` setup diagnostic.
- It may create or update internal registry state needed for the exported
  command to start a bound `volicord mcp --stdio` process.
- Omission of `--read-only` uses workflow mode.
- When `--output PATH` is present, the configuration is written to that exact
  output file. When `--output` is omitted, the command writes the default MCP
  config file for the resolved repository context, using `volicord.mcp.json` as
  the default filename.
- Exported configuration remains user-managed after export. Volicord must not
  claim that an arbitrary external host loaded, trusted, approved, initialized,
  or exposed it.

## Guard hook commands

`volicord guard` commands are local hook entry points for hosts that can run a
command during agent lifecycle events. They inspect registered project state,
record guarded-operation events, and return a machine-readable local decision.
They do not replace Core methods, user-owned judgments, `Write Check`,
close-readiness checks, host trust, shell approval, or OS-level sandboxing.

Each guard command reads one JSON hook event from stdin by default. `--file PATH`
reads that JSON event from a file for tests or host integrations that stage
events. JSON output is the default and includes `decision`, `allowed`,
`guard_event_id`, optional `session_id`, and a command-specific `result`.
`--text` selects a concise human-readable line. Supported decisions are
`allow`, `deny`, `warn`, and `inject_context`.

Project selection uses `--repo PATH`, an event project or repository field when
present, or the current working directory. `--connection ID` supplies the
Agent Connection identity when the hook event does not contain `connection_id`.
`--session ID`, `--guard-installation ID`, `--host HOST`, and
`--guard-mode MODE` can pin the recorded session, installation, host kind, and
guard mode. Host kinds use storage values such as `codex`, `claude_code`, or
`generic`. Guard modes are `mcp_only`, `guarded`, or `managed`.

When a non-`mcp_only` guard command receives a valid event for the recorded
project, Agent Connection, guard installation, host kind, guard mode, policy
hash, and known hook phase, Volicord records observation metadata. The
observation can promote the guard installation to `active` only when required
hook configuration is complete and the installation is not degraded, stale, or
broken. Invalid project, connection, host kind, guard mode, policy hash, or hook
phase data does not activate the installation. `active` means Volicord observed
a matching hook event for a currently usable guard configuration; it does not
claim OS-level enforcement, sandboxing, or write prevention.

The input event contract is host-neutral. Guard parsing is tolerant of common
field placements for host kind, session, tool name, command, prompt, result,
and changed paths, and preserves unknown fields in the stored guard event's
redacted subject. Prompt-like fields are hashed or omitted by default; prompt
capture records store the prompt hash and omit prompt text unless a future
owner-defined policy says otherwise.

Lifecycle behavior:

- `session-start` records or reuses the Agent Session and returns
  `inject_context` with concise project, active task, Write Check, pending
  judgment, blocker, and unresolved-change context for host-session injection.
- `pre-tool` classifies read-only, clearly mutating, and uncertain tool
  attempts. Read and status commands are allowed without creating blockers. A
  product-file write attempt may return `deny` or `warn` when there is no active
  task, no current active `Write Check`, an attempted target is outside the
  selected Product Repository, or policy blocks a clearly mutating shell
  command. Uncertain shell commands default to `warn` unless guard policy asks
  for `deny`. When pre-tool allows a clearly mutating product-file write with a
  concrete in-repository path set, active task, current write readiness, and
  compatible project scope, it records an expected-write correlation row with
  project, connection, session, optional host invocation identity, tool kind,
  exact path policy, task/change-unit/write-check basis, and timestamp
  metadata. Read-only and uncertain commands do not create expected-write rows.
- `post-tool` records the observed tool outcome. When the event supplies
  changed Product Repository paths, post-tool first tries to match them to a
  prior expected-write row from the same project, connection, session, bounded
  time window, and exact path policy, using host invocation identity when the
  host supplies it. Matched in-scope writes do not create unresolved
  unrecorded-change rows. Unmatched, out-of-scope, or ambiguous observed
  Product Repository changes record an unresolved unrecorded-change row and
  return `warn`. Post-tool observation and matching are guarded-operation
  records, not proof of product correctness. It does not execute untrusted
  commands to discover changes.
- `prompt-capture` records prompt-capture metadata and recognizes strict
  chat judgment commands only when prompt-capture availability for the current
  host, project, and connection is `configured`, `observed`, or `active`, and
  the prompt contains an explicit line such as `Volicord: answer J-3 1 #AB7K`,
  `Volicord: answer J-3 reject #AB7K`, `Volicord: answer J-3 defer #AB7K`, or
  `Volicord: note J-3 "text" #AB7K`. Unsupported, unconfigured, reload-needed,
  or degraded prompt capture returns structured non-recording output such as
  `prompt_capture_unsupported`, `prompt_capture_not_configured`, or
  `prompt_capture_reload_required`, with one next action. Non-command prompts
  proceed normally only when prompt capture is available. Malformed, ambiguous,
  unknown, missing-code, wrong-code, stale, duplicate, wrong-project, or
  wrong-connection judgment commands return `deny` without recording a judgment.
  A valid command records the addressed pending judgment through the local
  `User Channel` with `actor_source=local_user` and
  `resolved_verification_basis=user_prompt_submit_hook`,
  omits the full prompt text from prompt-capture storage, and returns
  model-visible recorded-context output instead of treating the command as
  ordinary agent instruction.
- `stop` checks whether the active task can safely be treated as complete. It
  returns `deny` when close-readiness blockers remain, user-owned judgments are
  pending, or unresolved unrecorded changes remain; otherwise it returns
  `allow`.

## Change reconciliation command

`volicord changes reconcile [--repo PATH] [--task active|ID] [--json]` is the local recovery command for unresolved guarded unrecorded Product Repository change findings.

The command resolves the selected project from `--repo PATH` or the current working directory and selects the active Task by default. It calls the public `volicord.reconcile_changes` Core method with `actor_source=local_user` and `operation_category=local_recovery`, prints the number of resolved findings, pending user judgments, and remaining unresolved findings, and exits under the normal CLI exit-code model. Rejected Core responses remain rejected CLI results rather than successful reconciliation summaries.

The command may resolve deterministic findings or create pending user-owned judgments. It does not record a user answer, accept a change on the user's behalf, prove correctness, prove review or test sufficiency, or complete close readiness. When it creates pending judgments, the user records them through the existing `User Channel` paths, then reruns `volicord changes reconcile`.

## User Channel commands

<a id="user-channel-commands"></a>
<a id="user-interaction-commands"></a>

`volicord user` commands provide a local CLI path for a human user to inspect
task status and answer pending user judgments through the `User Channel`. They
do not create an Agent Connection, install MCP host configuration, or make an
Agent Connection eligible to act as the user.

When the initialized MCP client declares elicitation support, MCP elicitation
is the preferred interactive path for pending judgments created through
`volicord.request_user_judgment`. If elicitation is unavailable and
prompt-capture availability is `configured`, `observed`, or `active`, fallback
guidance may show exact chat commands such as `Volicord: answer J-3 1 #AB7K`
with the current verification code. The terminal `volicord user` commands
remain the local recovery and manual-inspection path when elicitation or prompt
capture is unavailable, disabled, degraded, or inappropriate for the workflow.

Project selection uses `--repo PATH` or the current working directory's
repository root. Task selection uses the active task by default; `--task active`
is explicit and `--task ID` selects a named task.

The ordinary text-mode judgment flow uses the numbered indexes printed by
`volicord user judgments` and `volicord user judgment show`. Stored judgment
and option identifiers remain reference and JSON details.

Commands:

- `volicord user status` shows user-oriented task status through
  `volicord.status` with `actor_source=local_user`, `operation_category=read`,
  and User Channel provenance.
- `volicord user judgments` lists pending judgments for the selected task, with
  stable display indexes for the current output.
- `volicord user judgment show INDEX_OR_ID` displays one pending or historical
  judgment, its context summary, and Core-generated options.
- `volicord user judgment answer INDEX_OR_ID OPTION_INDEX_OR_ID` records one
  selected Core-generated option through `volicord.record_user_judgment` with
  `actor_source=local_user`, `operation_category=user_only`, compatible User
  Channel provenance, and the selected option's stored machine action and
  outcome. `--note` is stored only as a note.

Recording one judgment records only the addressed judgment. Final acceptance and
residual-risk acceptance remain separate judgment kinds and actions; this
command must not collapse one into the other.

Status, judgment list, and show output expose selected owner state for the
user's next action. They do not create evidence, final acceptance,
residual-risk acceptance, or close readiness. Only
`volicord user judgment answer` mutates the addressed pending judgment, and it
does so only through the selected Core-generated option.

<a id="dry-run"></a>
## Dry run and JSON output

`--dry-run` performs planning, validation, conflict detection, host target
rendering, and output shaping without persistent changes.

Dry-run does not:

- create a `Volicord Runtime Home`
- create or modify SQLite databases
- create SQLite WAL or SHM files
- apply registry or project-state migrations
- register or update projects, Agent Connections, Connection Projects,
  installation profile rows, guard installation rows, or verification status
  rows
- create, modify, or remove host configuration files
- create, modify, or remove `Product Repository` files or directories
- create, modify, or remove generic export files
- invoke MCP startup checks, MCP initialization, or tool discovery

Text output must be human-readable and identify each resource action using
`created`, `reused`, `updated`, `removed`, `skipped`, `conflict`, or `planned`.

<a id="setup-output"></a>
JSON output is administrative CLI output, not a public Volicord API response
schema. Commands that report setup, connection, export, project, or user-channel
state must include enough structured status for noninteractive operators to
distinguish successful setup from required user action.

Required diagnostic JSON values:

- `status`: `complete`, `action_required`, `failed`, `not_verified`, or
  `dry_run`
- `checks[]`: ordered diagnostic checks with a stable check id, status, summary,
  and optional details
- `actions[]`: required or suggested user actions, each with a stable action id
  and human-readable command or instruction when one is available

Setup and doctor JSON must include `status_meaning` so diagnostic consumers can
distinguish setup action status from installation-profile health.
Doctor JSON must separate blocking local repairs in `actions_required[]` from
warning-only follow-up in `actions_recommended[]` when the top-level status
remains `complete`.

<a id="noninteractive-approval-behavior"></a>
## Noninteractive behavior

Noninteractive commands must not prompt for missing user input or
host-controlled action. They must report the missing condition through the
normal result model: recoverable user or host action as `action_required`,
usage mistakes as exit code `2`, and conflicts or runtime failures as exit
code `1`.

Rules:

- Shared-intent Product Repository writes are authorized by the explicit
  `--shared` command path and are limited to the managed integration files that
  command previews.
- Existing unmanaged content is a conflict. The CLI must not silently replace
  unrelated host configuration or product files.
- A broad shell approval, write approval, host trust decision, sensitive-action
  approval, or `Write Check` does not substitute for the explicit CLI command
  path required by this administrative contract.
- Host trust, project trust, project MCP approval, OAuth, restart, and reload
  actions remain user-controlled host actions and cannot be supplied by the CLI.

## Administrative boundary

The administrative CLI can initialize, register, connect, export, and diagnose
local resources. It does not create public Volicord API methods and does not by
itself create Core authority, Write Check compatibility, evidence sufficiency,
close readiness, user-owned judgment, acceptance, residual-risk acceptance,
artifact authority, or security guarantees.

Owner routes:

- Public method list and method routing: [API Methods](api/methods.md).
- Shared request and response schemas: [API Schema Core](api/schema-core.md).
- Agent Connection, Connection Projects, and actor context meaning:
  [Agent Connection](agent-connection.md).
- MCP process behavior: [MCP Transport](mcp-transport.md).
- Runtime location and repository write boundaries:
  [Runtime Boundaries](runtime-boundaries.md).
