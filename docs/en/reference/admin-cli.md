# Administrative CLI reference

This document owns the local `volicord` administrative and bootstrap CLI
contract. The CLI establishes the `Volicord Runtime Home`, registers projects
from repository roots, manages Agent Connections without requiring users to
handle internal identities, provides the local `User Channel` command path,
and reports setup or connection diagnostics. Hidden internal hook commands exist
only for generated host integration wrappers and are not normal user-facing
commands. These commands are not public Volicord API methods.

It does not define public API method behavior, API schemas, storage record
layout, security guarantees, Core authority semantics, or MCP stdio transport
behavior.

## Owns / does not own

This document owns:

- `volicord` command names, command-line arguments, defaults, stdout/stderr
  routing, and process exit codes
- Runtime Home, installation profile, and MCP command selection during `init`
- repository-root project detection and administrative project commands
- Agent Connection command behavior for supported host integrations
- local serve command names, command-line arguments, defaults, stdout/stderr
  routing, and startup exit codes
- hidden internal hook lifecycle command names, options, decisions, output, and
  event-recording behavior for generated host wrappers
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
- storage record layout, SQLite DDL, canonical storage schema definitions,
  Core authority semantics, and security guarantee meanings

<a id="surface-stability"></a>
## Surface Stability

For label meanings, see [Documentation Policy](../maintain/documentation-policy.md#surface-stability-labels).

| Surface | Stability | Notes |
|---|---|---|
| Supported administrative command names, options, stdout/stderr routing, process exit codes, dry-run behavior, and local User Channel command names | `stable` | These are local CLI contracts, not public Volicord API methods. |
| `detective` profile setup, host-hook observation, session watcher observation, local consent availability reporting, and host-specific integration capability reporting | `beta` | These are supported cooperative observation surfaces with capability gates and owner-defined non-guarantees. |
| Hidden hook lifecycle namespace, generated wrapper details, stored internal identities, host config keys, and process-binding values | `internal` | These details support generated host integrations and must not become normal user-facing command inputs. |
| Human-readable status summaries, doctor reports, connection verification reports, compact summary cards, action text, and diagnostic disclosures | `diagnostic` | JSON field presence and stable IDs are contracts only where this page explicitly requires them; text formatting is not a public API schema. |

## Command model

`volicord` is a local administrative/bootstrap executable. It is not a general
long-running server. The explicit `volicord serve` command is limited to the
local MCP transport process described in [MCP Transport](mcp-transport.md). The
`volicord inbox` command group is the user-facing `Judgment Inbox` and local
`User Channel` CLI adapter over selected Core methods; its command names remain
administrative CLI commands, not public Volicord API methods.

Supported baseline commands:

```text
volicord --help
volicord --version
volicord init --host codex|claude-code --repo PATH [--profile record|detective] [--home PATH] [--mcp-command PATH] [--dry-run] [--json]
volicord status [--repo PATH] [--task active|ID] [--json]
volicord doctor [--json]
volicord connection add [HOST] [--repo PATH] [--shared|--global] [--read-only] [--dry-run] [--json]
volicord connection list [--repo PATH] [--json]
volicord connection status [HOST] [--repo PATH] [--shared|--global] [--json]
volicord connection verify [HOST] [--repo PATH] [--shared|--global] [--json]
volicord connection mode [HOST] workflow|read-only [--repo PATH] [--shared|--global] [--json]
volicord connection remove [HOST] [--repo PATH] [--shared|--global] [--dry-run] [--json]
volicord project use [PATH] [--json]
volicord project current [--json]
volicord project list [--json]
volicord project rename NAME [--repo PATH] [--json]
volicord project forget [PATH|NAME] [--json]
volicord mcp --stdio --connection <connection_id> [--project <project_id>]
volicord serve --transport local-http [--listen 127.0.0.1:8765 | --container-listen 0.0.0.0:8765] [--home PATH] [--connection <connection_id>] [--project PATH]... [--token-file PATH | --token TOKEN | --generate-token] [--allow-origin ORIGIN]
volicord changes reconcile [--repo PATH] [--task active|ID] [--dry-run] [--json]
volicord inbox [--repo PATH] [--task active|ID] [--json]
volicord inbox answer <judgment-id> --choice <choice> [--repo PATH] [--note TEXT] [--json]
volicord inbox open <judgment-id> [--repo PATH] [--json]
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
- Hidden internal hook commands use `--output volicord-json` by default. In
  `volicord-json` mode they write the Volicord wrapper JSON, `deny` exits `1`,
  and `allow`, `warn`, and `inject_context` exit `0`. `--output text` uses the
  same exit behavior with a concise human-readable line.
- Hidden internal hook commands with `--host-output codex|claude-code` write
  host-native hook output instead of the Volicord wrapper JSON. Policy decisions
  use the host's stdout, stderr, and exit-code rules; generated Codex and Claude
  Code hook wrapper scripts use this mode, and Claude Code policy blocks are not
  represented as exit code `1`.
- Errors remain stderr diagnostics under the CLI exit-code model.
- `volicord serve --transport local-http` is an explicit long-running MCP
  transport process. Native local runs accept only loopback listen addresses.
  The explicit `--container-listen` mode is limited to Docker host-loopback
  publishing. The MCP local HTTP endpoint requires bearer authentication,
  prefers `--token-file PATH` over direct command-line token values, generates
  a process-local token when no token is supplied, applies Origin checks to
  browser-facing requests, and delegates HTTP wire behavior and
  transport-bound authentication and Origin checks to
  [MCP Transport](mcp-transport.md). It is local/Docker transport only, not a
  public network API, SaaS endpoint, multi-user server, or security boundary.

Not supported:

- The CLI has no general-purpose `server` or daemon command.
- `volicord serve` must not be treated as a public Volicord API service, SaaS
  endpoint, multi-user server, security boundary, or unauthenticated network
  service. `--container-listen` exists only for Docker host-loopback publishing;
  it is not a public host-interface or remote serving option.
- Administrative commands are not public Volicord API methods and must not be
  added to the public method list.
- Hidden hook commands are cooperative detective signal commands, not OS-level
  sandboxing or a security-enforcement proof, and they are not shown in normal
  top-level help.
- Text-mode user flows must not require users to type `project_internal_id`,
  `connection_internal_id`, host config keys, protocol envelopes, or stored
  registry fields.

Major status-like user-facing surfaces, including `volicord status`,
`volicord doctor`, `volicord connection status`, `volicord connection verify`,
`volicord changes reconcile`, and `volicord inbox`, use a compact summary card
before detailed diagnostics when the command can compute one. Text output uses
the public labels `Task`, `Recording`, `Profile`, `Write Ticket`, `Evidence`,
`User Judgment`, `Changes`, `Close Status`, `Transport`, `Next`, and
`Guarantee`. `Next` must name the immediate next action when the command can
know one, may include a follow-up verification command when that is how the user
confirms the action, and uses `none` only when no next action is known for the
selected view.
Text-mode summary output must not expose internal IDs by default unless an ID is
needed to perform the displayed next action. Matching JSON output exposes the
same stable data as `summary_card`; JSON consumers must not parse text-only
formatting.
When pending judgments are visible, `volicord status` and `volicord inbox` text
output may include an `Available answer paths:` line that summarizes host
prompt input, chat command capture, local consent URL, and CLI inbox
availability. The line tells the user where to answer; it does not record a
judgment or let an Agent Connection act as the user. JSON output carries the
same facts in `user_channel_availability` or `answer_path_availability`.
When text output reports `action_required` or a degraded diagnostic state, it
also includes concise `Result`, `Why`, `Next`, and `Does not prove` lines. The
`Result` line must not present `action_required` as a fatal CLI error. `Next`
names the immediate user action, such as reloading or restarting the host,
approving host permission, repairing managed configuration, or using
`volicord connection verify ...` after the host-side action has been completed.

<a id="runtime-home-selection"></a>
## Runtime Home Selection

`volicord init` is the public first-run path for creating or reusing the local
installation profile. It creates or verifies the selected Runtime Home, records
the command paths later administrative, Agent Connection, and MCP process flows
use, registers the selected repository, and installs the selected host
connection. Init can select the Runtime Home path or MCP launch command while
performing repository setup and host connection. It cannot change the parent
shell's current environment.

The top-level setup status answers whether installation-profile preparation or
host connection still needs a named user action. Init may report
`action_required` after saving the Runtime Home and installation profile when
selected commands are not ready for future `PATH` lookup by shells or agent
hosts. Output must keep command-availability details and required actions
explicit.

Arguments:

| Argument | Meaning |
|---|---|
| `--home PATH` | Selects the `Volicord Runtime Home`. Omission uses the platform default local runtime location. The selected path must satisfy the Runtime Home/Product Repository separation contract before project state is used. |
| `--mcp-command PATH` | Stores the exact `volicord` command that managed host configuration should use before the `mcp --stdio --connection <connection_id> [--project <project_id>]` arguments when init creates or updates the installation profile. Omission uses the running `volicord` executable selected by init. |
| `--json` | Selects machine-readable, noninteractive output. Init does not prompt in JSON mode. |

Init effects that relate to Runtime Home and installation profile selection:

- creates or validates the Runtime Home registry
- records Runtime Home identity and installation profile metadata
- records the selected `volicord` command location and MCP launch command for
  later `init`, `connection`, `doctor`, and MCP startup flows
- inspects whether selected command paths resolve through the current process
  `PATH`
- reports a `PATH` action when selected commands are not visible to the current
  process; existing shells and agent host processes may need restart or reload
- registers or reuses the selected repository as part of the first-run host
  setup path
- does not create a public Volicord API method or record a user-owned judgment

`volicord doctor` is the read-oriented diagnostic command for the installation
profile. Its top-level status answers whether the current installation profile
is usable. It verifies Runtime Home access, registry schema, installation
profile presence, stored command readiness, command availability through
`PATH`, and command-link or shim readiness when link metadata is present. When
stored command paths are executable, doctor may report `complete` while
reporting command-availability warnings and `actions_recommended` for future
shells or agent hosts. PATH or command-link recommendations must say when
existing agent hosts may need restart or reload. Doctor reports supported host
detection as a connection-verification concern. When detective-profile
installation records exist, doctor may also report detective host hook file
installation, configuration health, runtime hook observation health, effective
detective health, and host reload requirement as diagnostics. These diagnostics
are local setup and observation checks; they are not proof of OS enforcement,
sandboxing, write prevention, product correctness, or Close Status. Doctor also reports
`selected_profile` and an observation summary that includes host-hook,
session-watcher, cooperative decision, unrecorded-change detection, actor-proof,
and OS-enforcement facts. Runtime-only capabilities such as session watcher
observation and local web consent are reported as unavailable unless the
reporting process actually owns that runtime state. Doctor does not create
projects, install host configuration, change
connection mode, or answer user judgments.
Text and JSON doctor output include a diagnostic disclosure and the compact
`summary_card` for the diagnostic view. JSON output uses
`disclosure.guarantee_class=detective_observation` and `non_guarantees` values
such as `NotOsSandbox`, `NotNetworkIsolation`, `NotFullWritePrevention`,
`NotActorAttributionProof`, `NotCorrectnessProof`, `NotTestSufficiencyProof`,
and `NotHumanReviewReplacement`.

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
configuration may contain `connection_id` and, when safely project-bound,
`project_id` process-binding values derived from stored internal identities,
server name, and command arguments so that the host can start
`volicord mcp --stdio`; those values are saved process-binding details, not
user authority tokens. Text-mode command input uses the selected host, intent,
and repository root instead.

Ordinary `volicord connection add` commands use the saved profile in the resolved
Runtime Home instead of asking for an MCP command path or Runtime Home path.
Personal, local, or user-wide host configuration may carry that Runtime Home as
`VOLICORD_HOME`. Shared project host configuration must not embed a personal
Runtime Home path; it uses `volicord` as the command name and project-bound
`mcp --stdio --connection <connection_id> --project <project_id>` arguments
when the generated entry is for one selected project. Connection-only generated
arguments are reserved for entries that intentionally serve more than one
connected project. The future host environment must resolve the command through
`PATH`.

<a id="agent-host-setup-and-init"></a>
`volicord init --host codex --repo PATH --profile record` and
`volicord init --host claude-code --repo PATH --profile record` are the primary
first-run repository setup and host-connection examples for chat-first use when
host hook and session watcher observation is not being installed. Init uses the
shared, project-scoped host layout so generated host MCP configuration starts
`volicord mcp --stdio --connection <connection_id> --project <project_id>`
through `PATH` and does not embed a personal Runtime Home path.

`--profile` selects the public integration profile:

- `record` is the default. It writes MCP configuration, the managed `AGENTS.md`
  guidance block, and policy metadata without requiring host lifecycle hooks or
  a session watcher.
- `detective` writes MCP configuration, the managed `AGENTS.md` guidance block,
  `.volicord/policy.json` hook command policy, supported project-local host hook
  and rule files, and records the host-hook/session-watcher observation state.

Detective-aware setup, status, verification, and doctor text output report
`selected_profile` and an `observation_summary`. The corresponding JSON field is
`control_surface`. The summary includes
`host_hooks_active`, `session_watcher_active`,
`cooperative_pre_tool_warning_available`,
`cooperative_pre_tool_denial_available`,
`unrecorded_changes_detectable`, `actor_identity_provable`, and
`os_enforced`. Current Volicord output must report `os_enforced=false` and
`actor_identity_provable=false`.

Detective initialization requires the selected host adapter to declare and verify
support for every required lifecycle hook:
`session-start`, `pre-tool`, `post-tool`, `prompt-capture`, and `stop`. It also
requires session watcher snapshot support for the selected Product Repository.
`AGENTS.md` and `.volicord/policy.json` are not host hook configuration. If the
adapter does not know a reliable project-local hook schema or path for every
required phase, init fails with `DETECTIVE_HOOKS_UNSUPPORTED`. If the session
watcher cannot snapshot the selected repository, init fails with
`DETECTIVE_WATCHER_UNSUPPORTED`. The recovery is to use `--profile record` for
record-only setup or prepare a supported host, platform, and repository
configuration for detective before rerunning init. `record` does not require hook
installation or session watcher setup.

On native Windows, init rejects `--profile detective` with
`DETECTIVE_WINDOWS_UNSUPPORTED` before planning or writing detective host hook files.
Native Windows supports `--profile record`; use WSL2, Linux, or macOS for
detective only where the selected host hook and watcher contracts are supported
and tested.

Codex detective initialization additionally requires the selected Product
Repository to be a Git work tree root that supports cwd-independent wrapper
resolution from subdirectory host sessions. When that prerequisite is not met,
init fails with `DETECTIVE_HOOK_ROOT_UNSUPPORTED` instead of generating a bare
relative hook path. Claude Code detective initialization uses the host
project-directory placeholder described under
[detective host hook lifecycle commands](#guard-hook-commands).

For `detective`, init records `reload_required` when the host still needs restart
or reload to load generated detective host hooks, and `configured` when files are
installed but no matching host-hook event has been observed. Init does not mark
a detective installation record `active` merely because files were written.

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
  with `volicord mcp --stdio --connection <connection_id> --project
  <project_id>`
- writes or updates only the Volicord-managed block in `AGENTS.md`
- writes `.volicord/policy.json` with detective host hook commands that invoke
  the hidden internal hook namespace
- writes Volicord-managed hook wrapper scripts under `.codex/hooks/` or
  `.claude/hooks/` for required detective lifecycle phases
- writes supported host hook files such as `.codex/hooks.json` or
  `.claude/settings.json` that invoke those wrapper scripts
- writes supported host rule files such as `.codex/rules/*.rules` or
  `.claude/rules/volicord.md`
- records detective-profile hook observation status in the Runtime Home registry
- rejects `detective` initialization when required host hook configuration or
  session watcher support is missing
- rejects `detective` initialization on native Windows because Windows host-hook
  wrappers and watcher behavior are not implemented and tested
- reports the required host restart, reload, approval, or trust action when the
  host must load the new MCP or detective host hook configuration

Re-running init is idempotent for matching Volicord-managed content. It updates
managed blocks, policy files, host MCP entries, and detective installation records
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
| `volicord init` | Initializes Runtime Home and installation profile if needed, registers or reuses the selected repository project, creates or updates the shared project-scoped Agent Connection, ensures Connection Projects membership, and records detective-profile hook observation status. | Installs or updates managed project-local MCP configuration, `AGENTS.md` guidance, `.volicord/policy.json`, supported host hook wrapper scripts, and host hook and rule files for `codex` or `claude-code`. | Runs host-config, MCP startup, initialization, and `tools/list` checks where observable, then reports any host reload, restart, trust, or approval action. |
| `volicord connection add` | Registers or reuses the selected repository project, creates or updates the matching Agent Connection, records the connection intent and mode, and ensures the project is in Connection Projects. | Installs or updates managed host configuration for `codex` or `claude-code` according to the selected intent. | Runs host-config, MCP startup, initialization, and `tools/list` checks where observable. |
| `volicord connection list` | Reads matching Agent Connections and connected projects. | Does not launch the host and does not rewrite host configuration. | Reports stored and diagnostic verification state without refreshing host checks. |
| `volicord connection status` | Reads one selected Agent Connection. | Does not launch the host and does not rewrite host configuration. | Reports full stored verification status and required user actions. |
| `volicord connection verify` | Reads the selected Agent Connection and updates last-known verification status. | Inspects the managed target when the host integration owns an observable target. | Runs the observable checks and stores the resulting verification state. |
| `volicord connection mode` | Updates the selected connection mode. | Does not rewrite host configuration unless the host entry must be regenerated to reflect the mode. | Reports diagnostics after the mode change. |
| `volicord connection remove` | Removes selected Connection Projects membership and removes the Agent Connection when no owned membership remains. | Removes only matching managed host configuration when ownership and safety checks permit removal. | Does not delete projects, Core state, Runtime Home, artifact storage, or unrelated host configuration. |

Rules:

- `volicord connection add` must never connect every project in the Runtime Home by
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
blocked, and the next user action when one is required. For `action_required`
and degraded detective diagnostics, text output keeps the next action concrete:
reload or restart the host, approve host or project permission, repair managed
configuration, or run the shown `volicord connection verify ...` command after
the host-side action. JSON output must include top-level `status`, `checks`,
`actions`, and `summary_card` fields for diagnostic consumers.
Connection status and verification output must keep detective host hook file installation,
configuration health, runtime hook observation health, effective detective health,
host reload requirement, prompt-capture availability, and last host-hook event when
known as separate diagnostics. Text output reports `selected_profile`, an
`observation_summary`, cooperative pre-tool warning availability,
cooperative pre-tool denial availability, post-tool correlation availability,
unrecorded-change detection availability, prompt-capture availability, local web
consent availability, hook path safety, hook command cwd independence, hook
command subdirectory safety, watcher status, watcher baseline creation time,
watcher coverage start time, watcher coverage basis, and any watcher
partial-coverage warning as separate fields. Files installed or configured must
not be reported as an active observed hook or as active host-hook
observation before a matching observation exists. Incomplete session-watch
coverage must not be reported as full unrecorded-change detection.
Text and JSON Agent Connection outputs are diagnostic outputs. JSON output uses
`disclosure.guarantee_class=detective_observation` with stable
`non_guarantees` for OS sandboxing, network isolation, malware defense, full
write prevention, actor attribution proof, correctness proof, test sufficiency
proof, and human review replacement.

A successful `volicord mcp --check` startup check alone must not be described as a
`complete` Agent Connection. It is startup validation for the MCP process only.

<a id="generic-mcp-config-export"></a>
## Host MCP configuration

Volicord does not expose a public generic MCP config export command. Supported
host setup is performed through `volicord init` and `volicord connection add`.
Those commands write supported host configuration directly when the selected
host adapter owns a managed target. Generic or otherwise unsupported external
hosts remain user-managed configuration surfaces.

Rules:

- Supported managed host configuration is tied to an Agent Connection and starts
  a bound `volicord mcp --stdio` process.
- User-managed generic host configuration may name the installed `volicord`
  executable and the `mcp --stdio --connection <connection_id>
  [--project <project_id>]` arguments after a supported Agent Connection exists.
- Volicord must not claim that an arbitrary external host loaded, trusted,
  approved, initialized, or exposed a user-managed configuration.

<a id="guard-hook-commands"></a>
## Internal detective hook lifecycle commands

The hidden internal hook namespace is a local entry point for generated host
wrappers that can run a command during agent lifecycle events. It is not shown
in normal top-level help and is not a general user-facing command group. Hook
commands inspect registered project state, record host-observation events, and
return a machine-readable local decision. They do not replace Core methods,
user-owned judgments, write tickets, close-readiness checks, host trust, shell
approval, or OS-level sandboxing.

Each host-hook command reads one JSON hook event from stdin by default. `--file PATH`
reads that JSON event from a file for tests or host integrations that stage
events. The default `--output volicord-json` output includes `decision`,
`allowed`, `guard_event_id`, optional `session_id`, and a command-specific
`result`. `--output text` selects a concise human-readable line. Supported
decisions are `allow`, `deny`, `warn`, and `inject_context`.

`--host-output codex|claude-code` selects host-native hook rendering for
installed host hooks. In this mode stdout contains only host-recognized response
JSON or context, or is empty when the host expects no output; stdout does not
contain the Volicord wrapper JSON. Stored host-hook events keep the internal
decision and result details used by Volicord.
Volicord wrapper JSON includes `disclosure.guarantee_class=cooperative_host_decision`.
Host-native output must include a concise cooperative-decision disclosure in the
context or denial reason. These cooperative decisions are not OS-level enforcement, network
isolation, malware defense, actor attribution proof, full write prevention,
correctness proof, test sufficiency proof, or human review replacement.

Project selection uses `--repo PATH`, an event project or repository field when
present, or the current working directory. `--connection ID` supplies the
Agent Connection identity when the hook event does not contain `connection_id`.
`--session ID`, `--guard-installation ID`, `--host HOST`, and
`--integration-profile record|detective` can pin the recorded session,
installation, host kind, and integration profile. Host kinds use storage values
such as `codex`, `claude_code`, or `generic`. Public integration profiles are
`record` and `detective`.
`--policy-hash HASH` pins the expected `.volicord/policy.json` hash for
generated hook wrapper scripts; a mismatch prevents that hook event from
activating the detective installation record, while internal hook commands used
for tests or debugging may omit the option.

Generated Codex hook configuration must be cwd-independent and
subdirectory-safe. It does not invoke a bare `.codex/hooks/...` path. Each hook
entry runs a POSIX `sh` command with the shape:

```sh
root=$(git rev-parse --show-toplevel) || exit $?
exec "$root/.codex/hooks/volicord-dispatch.sh" PHASE
```

The generated `.codex/hooks/volicord-dispatch.sh` script is Volicord-managed. It
resolves the Git work-tree root again at runtime, requires an absolute root,
checks that the selected phase wrapper exists and is executable, and then execs
the phase wrapper under that root. If the Git root cannot be resolved, the
dispatch path fails instead of falling back to the host session cwd.

Generated Claude Code hook configuration must also be cwd-independent and
subdirectory-safe. It uses exec-form commands rooted at
`${CLAUDE_PROJECT_DIR}`, such as
`${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-pre-tool.sh`, with no args.

Generated wrapper scripts under `.codex/hooks/` and `.claude/hooks/` forward
stdin unchanged to the hidden internal hook namespace, preserve stdout, stderr, and the host-hook
exit code, and pass the expected host kind, host-native output mode, repository
selector, Agent Connection, host-hook installation, and policy hash. Users must not
replace generated hook commands with bare `.codex/hooks/...` or
`.claude/hooks/...` relative paths.

Detective-aware status, verification, and doctor diagnostics report
`hook_path_safety`, `hook_commands_cwd_independent`,
`hook_commands_subdirectory_safe`, and `generated_config_verified`. Hook path
safety can report values including `relative_path_unsafe`, `wrapper_missing`,
`wrapper_not_executable`, `absolute_path_stale`, `placeholder_unsupported`,
`host_output_mismatch`, and `policy_hash_mismatch`; the complete value set is
owned by [API Value Sets](api/schema-value-sets.md#state-and-blocker-values).
Any non-`ok` hook path safety value keeps detective host hooks inactive for that
view. The repair action is to regenerate the safe managed commands with
`volicord init --host HOST --repo PATH --profile detective`, then complete any
host trust, approval, reload, or restart action still reported.

When a `detective` internal hook command receives a valid event for the recorded
project, Agent Connection, host-hook installation, host kind, integration profile,
policy hash, and known hook phase, Volicord records observation metadata. The
observation can promote the detective installation record to `active` only when
required hook configuration is complete and the installation is not degraded,
stale, or broken. Invalid project, connection, host kind, integration profile,
policy hash, or hook phase data does not activate the installation. `active`
means Volicord observed a matching hook event for a currently usable detective
configuration; it does not claim OS-level enforcement, sandboxing, actor
identity proof, or write prevention.

The input event contract is host-neutral. Host-hook parsing is tolerant of common
field placements for host kind, session, tool name, command, prompt, result,
and changed paths, and preserves unknown fields in the stored host-hook event's
redacted subject. Prompt-like fields are hashed or omitted by default; prompt
capture records store the prompt hash and omit prompt text unless a future
owner-defined policy says otherwise.

Lifecycle behavior:

- `session-start` records or reuses the Agent Session and returns
  `inject_context` with concise project, active task, write-ticket, pending
  judgment, blocker, and unresolved-change context for host-session injection.
- `pre-tool` classifies read-only, clearly mutating, and uncertain tool
  attempts. Read and status commands are allowed without creating blockers. A
  product-file write attempt may return `deny` or `warn` when there is no active
  task, no current active write-ticket row, an attempted target is outside the
  selected Product Repository, an observed path is outside active write-ticket
  scope, the active ticket match is ambiguous, or policy blocks a clearly
  mutating shell command. These decisions are cooperative host decisions, not
  OS-level enforcement. Uncertain shell commands default to `warn` unless host-hook
  policy asks for `deny`. When pre-tool allows a clearly mutating product-file
  write with a concrete in-repository path set, active task, exactly one current
  active matching write ticket, and compatible project scope, it records an
  expected-write correlation row with project, connection, session, optional
  host invocation identity, tool kind, exact path policy,
  task/change-unit/write-ticket basis, and timestamp metadata. Read-only,
  uncertain, ambiguous, and ticket-out-of-scope commands do not create
  expected-write rows.
- `post-tool` records the observed tool outcome. When the event supplies
  changed Product Repository paths, post-tool first tries to match them to a
  prior expected-write row from the same project, connection, session, bounded
  time window, and exact path policy, using host invocation identity when the
  host supplies it. If no expected-write row matches, post-tool may correlate
  the changed paths to exactly one current active matching write ticket.
  Matched in-scope writes do not create unresolved unrecorded-change rows.
  Unmatched, ticket-out-of-scope, or ambiguous observed Product Repository
  changes record an unresolved unrecorded-change row and return `warn`.
  Post-tool observation and matching are host-observation records, not proof of
  product correctness, actor identity, or write prevention. It does not execute
  untrusted commands to discover changes.
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

`volicord changes reconcile [--repo PATH] [--task active|ID] [--dry-run] [--json]` is the local recovery command for unresolved unrecorded Product Repository change findings.

The command resolves the selected project from `--repo PATH` or the current working directory and selects the active Task by default. It calls the public `volicord.reconcile_changes` Core method with `actor_source=local_user` and `operation_category=local_recovery`, prints the compact summary card plus the number of resolved findings, pending user judgments, and remaining unresolved findings, and exits under the normal CLI exit-code model. Rejected Core responses remain rejected CLI results rather than successful reconciliation summaries.

With `--dry-run`, the command returns the Core dry-run preview instead of committing reconciliation effects. Text and JSON output report planned automatic resolutions, changes needing user judgment, pending judgment requests that would be created, projected close blockers, next actions, and the disclosure that the preview is not actor proof, intent proof, or correctness proof. The dry-run command does not advance `project_state.state_version`, write mutation or replay records, resolve close blockers, create user judgments, stage artifacts, or attach artifacts.

The non-dry-run command may resolve deterministic findings or create pending user-owned judgments. It does not record a user answer, accept a change on the user's behalf, prove actor identity, prove intent, prove correctness, prove review or test sufficiency, or complete close readiness. When it creates pending judgments, the user answers them through the `Judgment Inbox`, then reruns `volicord changes reconcile`.

## User Channel commands

<a id="user-channel-commands"></a>
<a id="user-interaction-commands"></a>

`volicord inbox` commands provide the local CLI path for a human user to list and
answer pending user judgments through the `User Channel`. They
do not create an Agent Connection, install MCP host configuration, or make an
Agent Connection eligible to act as the user.

When the initialized MCP client declares host prompt support, host prompt input
is the preferred User Channel input method for pending judgments created through
`volicord.request_user_judgment`. If host prompt input is unavailable and
chat command capture is `configured`, `observed`, or `active`, fallback guidance
may show exact chat commands such as `Volicord: answer J-3 1 #AB7K`
with the current verification code. If both host prompt input and chat command capture are
unavailable and the adapter can safely expose a local consent URL, fallback
guidance may show a loopback consent URL backed by a short-lived one-time token.
The terminal `volicord inbox` commands remain the CLI inbox input method and
manual-inspection path when host prompt input, chat command capture, or local consent URL is
unavailable, disabled, degraded, or inappropriate for the workflow.

Project selection uses `--repo PATH` or the current working directory's
repository root. Task selection uses the active task by default; `--task active`
is explicit and `--task ID` selects a named task.

The ordinary text-mode judgment flow centers on stable judgment identifiers and
choice identifiers printed by `volicord inbox`. Stored judgment references and
additional capture-path details remain available in JSON output.

Commands:

- `volicord inbox` lists pending `JudgmentInboxItem` entries for the selected
  task, including the judgment id, question, choices or answer constraints,
  required/optional status, preferred capture path, and fallbacks such as local
  web consent or the CLI answer command when available.
- `volicord inbox answer <judgment-id> --choice <choice>` records one selected
  Core-generated option through `volicord.record_user_judgment` with
  `actor_source=local_user`, `operation_category=user_only`, compatible User
  Channel provenance, and the selected option's stored machine action and
  outcome. `--note` is stored only as a note.
- `volicord inbox open <judgment-id>` attempts the local web consent/browser
  path when the CLI process has a usable consent URL. If no URL is available
  from the CLI process, it reports `action_required` text with `Result`, `Why`,
  `Next`, and `Does not prove` lines and names the CLI answer command instead.

Recording one judgment records only the addressed judgment. Final acceptance and
residual-risk acceptance remain separate judgment kinds and actions; this
command must not collapse one into the other.

Status and inbox list output expose selected owner state for the user's next
action, including a compact `summary_card` when the view can compute one.
When pending judgments are present, text output also summarizes available
answer paths so unavailable host prompt input does not hide chat capture, local
consent, or CLI inbox paths. They do not create evidence, final acceptance,
residual-risk acceptance, or close readiness. Only
`volicord inbox answer` mutates the addressed pending judgment, and it
does so only through the selected Core-generated option.

<a id="dry-run"></a>
## Dry run and JSON output

`--dry-run` performs planning, validation, conflict detection, host target
rendering, and output shaping without persistent changes.

Dry-run does not:

- create a `Volicord Runtime Home`
- create or modify SQLite databases
- create SQLite WAL or SHM files
- initialize or validate registry or project-state schemas
- register or update projects, Agent Connections, Connection Projects,
  installation profile rows, host-hook installation rows, or verification status
  rows
- create, modify, or remove host configuration files
- create, modify, or remove `Product Repository` files or directories
- invoke MCP startup checks, MCP initialization, or tool discovery

Text output must be human-readable and identify each resource action using
`created`, `reused`, `updated`, `removed`, `skipped`, `conflict`, or `planned`.

<a id="setup-output"></a>
JSON output is administrative CLI output, not a public Volicord API response
schema. Commands that report setup, connection, project, or user-channel
state must include enough structured status for noninteractive operators to
distinguish successful setup from required user action.

Required diagnostic JSON values:

- `status`: `complete`, `action_required`, `failed`, `not_verified`, or
  `dry_run`
- `checks[]`: ordered diagnostic checks with a stable check id, status, summary,
  and optional details
- `actions[]`: required or suggested user actions, each with a stable action id
  and human-readable command or instruction when one is available
- `summary_card`: stable compact summary data for status-like diagnostic or
  user-channel outputs that compute a summary card
- Detective-aware setup, doctor, connection status, and connection verification
  JSON must expose `selected_profile`, `control_surface`,
  `cooperative_pre_tool_warning_available`,
  `cooperative_pre_tool_denial_available`, `post_tool_correlation_available`,
  `bash_shell_mutation_coverage`, `hook_path_safety`,
  `hook_commands_cwd_independent`, `hook_commands_subdirectory_safe`,
  `prompt_capture_available`, and `local_web_consent_available` where host-hook
  diagnostics are reported. `control_surface.os_enforced` must be `false`
  unless Volicord implements OS-level enforcement. Host-hook health JSON may also
  expose `generated_config_verified`,
  `native_host_output_adapter_verified`, and
  `direct_file_write_matcher_coverage` to show the stricter host-hook
  prerequisites. When watcher diagnostics are reported, JSON must also expose
  `watcher_status`, `watcher_baseline_created_at`,
  `watcher_coverage_start_at`, `watcher_coverage_basis`, and
  `watcher_partial_coverage_warning`.

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
  approval, or write ticket does not substitute for the explicit CLI command
  path required by this administrative contract.
- Host trust, project trust, project MCP approval, OAuth, restart, and reload
  actions remain user-controlled host actions and cannot be supplied by the CLI.

## Administrative boundary

The administrative CLI can initialize, register, connect, export, and diagnose
local resources. It does not create public Volicord API methods and does not by
itself create Core authority, write-ticket compatibility, evidence sufficiency,
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
