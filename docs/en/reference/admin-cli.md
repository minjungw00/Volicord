# Administrative CLI reference

This document owns the local `volicord` administrative and bootstrap CLI
contract. The CLI establishes the `Volicord Runtime Home`, registers projects
from repository roots, manages Agent Connections without requiring users to
handle internal identities, provides the local `User Channel` command path,
exports generic MCP configuration, and reports setup or connection diagnostics.
These commands are not public Volicord API methods.

It does not define public API method behavior, API schemas, storage record
layout, security guarantees, Core authority semantics, or MCP stdio transport
behavior.

## Owns / does not own

This document owns:

- `volicord` command names, command-line arguments, defaults, stdout/stderr
  routing, and process exit codes
- setup-time Runtime Home, installation profile, executable-link, and MCP
  command selection
- repository-root project detection and administrative project commands
- Agent Connection command behavior for supported host integrations
- generic MCP config export behavior
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
- MCP process startup, stdio framing, wire behavior, response wrapping, and
  shutdown; see [MCP Transport](mcp-transport.md)
- storage record layout, SQLite DDL, general storage migration definitions,
  Core authority semantics, and security guarantee meanings

## Command model

`volicord` is a local administrative/bootstrap executable. It is not a
long-running server. The `volicord user` command group is the local
`User Channel` CLI adapter over selected Core methods; its command names remain
administrative CLI commands, not public Volicord API methods.

Supported baseline commands:

```text
volicord --help
volicord --version
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
- Errors remain stderr diagnostics under the CLI exit-code model.

Not supported:

- The CLI has no `serve`, `server`, or daemon command.
- Administrative commands are not public Volicord API methods and must not be
  added to the public method list.
- Text-mode user flows must not require users to type internal project
  identities, Agent Connection identities, host config keys, protocol envelopes, or stored
  registry fields.

<a id="runtime-home-selection"></a>
## Setup and Runtime Home

`volicord setup` establishes the local installation profile. It creates or
verifies the selected Runtime Home and stores the command paths later
administrative, Agent Connection, export, and MCP process flows use. Setup is
the only baseline command that directly selects the Runtime Home path or MCP
command location.

Arguments:

| Argument | Meaning |
|---|---|
| `--home PATH` | Selects the `Volicord Runtime Home`. Omission uses the platform default local runtime location. The selected path must satisfy the Runtime Home/Product Repository separation contract before project state is used. |
| `--link-bin PATH` | Installs or updates user-selected command links for both `volicord` and `volicord-mcp` when feasible. The command reports each target path and refuses unsafe replacement. |
| `--mcp-command PATH` | Stores the command that managed host configuration and generic exports should use to start `volicord-mcp`. Discovery order is explicit `--mcp-command PATH` when supplied, then a sibling `volicord-mcp` next to the running `volicord` executable, then a command on `PATH`. |
| `--json` | Selects machine-readable output. |

Setup effects:

- creates or validates the Runtime Home registry
- records Runtime Home identity and installation profile metadata
- records the selected `volicord` and `volicord-mcp` command locations for
  later `connect`, `doctor`, export, and MCP startup flows
- may update the command links named by `--link-bin` for both executable roles
- reports a `PATH` action when a link directory is not visible to the current
  process; it cannot permanently modify the parent shell environment
- does not register a project unless a separate project or connection command
  selects a repository
- does not create a public Volicord API method or record a user-owned judgment

`volicord doctor` is the read-oriented diagnostic command for the setup profile.
It verifies Runtime Home access, registry schema, installation profile presence,
stored command readiness, and command-link or shim readiness when link metadata
is present. It reports supported host detection as a connection-verification
concern. It does not create projects, install host configuration, change
connection mode, or answer user judgments.

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
Registration creates an internal project identity, a user-facing project name, a
project home under the Runtime Home, and project-local state as needed. The
default project name is derived from the repository directory and made unique
inside the Runtime Home when needed.

`volicord project current` reports the project detected from the current working
directory. It does not create a project registration.

`volicord project list` lists registered projects by user-facing name, repository
root, status, and diagnostic availability.

`volicord project rename NAME [--repo PATH]` changes the user-facing project
name for the selected repository. It does not change the internal project
identity, repository root, project home, or Core state.

`volicord project forget [PATH|NAME]` removes the selected project registration
only when doing so does not orphan active Agent Connection membership or project
state that an owner requires to remain addressable. Forgetting a project must
not delete the `Product Repository`, unrelated Runtime Home data, host
configuration, artifact storage owned by another remaining registration, or
Core authority rows that must be preserved.

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
configuration may contain an internal connection identity, server name, and command
arguments so that the host can start `volicord-mcp`; those values are not user
authority tokens and are not required as text-mode command inputs.

Ordinary `volicord connect` commands use the saved profile in the resolved
Runtime Home instead of asking for an MCP command path or Runtime Home path.
Personal, local, or user-wide host configuration may carry that Runtime Home as
`VOLICORD_HOME`. Shared project host configuration must not embed a personal
Runtime Home path; it uses `volicord-mcp` as a command name that the future host
environment must resolve through `PATH`.

<a id="volicord-agent-install"></a>
## Agent Connection commands

Connection selection uses host, intent, and repository root. The command derives
or looks up the internal connection identity.

| Command | Runtime Home registry effect | Host configuration effect | Verification effect |
|---|---|---|---|
| `volicord connect` | Registers or reuses the selected repository project, creates or updates the matching Agent Connection, records the connection intent and mode, and ensures the project is in Connection Projects. | Installs or updates managed host configuration for `codex` or `claude-code` according to the selected intent. | Runs setup, host-config, MCP startup, initialization, and `tools/list` checks where observable. |
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
| `action_required` | Durable Agent Connection state and host configuration are present, but host trust, project approval, OAuth, reload, restart, command-link repair, setup repair, or a comparable user-controlled action remains. |
| `failed` | The requested command or verification did not establish usable durable Agent Connection state, usable host configuration, or a required local prerequisite. |
| `dry_run` | The command reported the planned actions without persistent changes. |

Verification output must make checks and user actions first-class diagnostics.
Text output must show the overall status, each check that was attempted or
blocked, and the next user action when one is required. JSON output must include
top-level `status`, `checks`, and `actions` fields for diagnostic consumers.

A successful `volicord-mcp` startup check alone must not be described as a
`complete` Agent Connection. It is startup validation for the MCP process only.

## Generic MCP config export

`volicord export mcp-config [--output PATH] [--repo PATH] [--read-only] [--json]`
exports host-neutral MCP configuration. It is a separate export flow, not a
normal host connection intent.

Rules:

- The command resolves or registers the selected repository project by root.
- It uses the setup profile's stored MCP command unless the setup is invalid,
  in which case it reports an `action_required` setup diagnostic.
- It may create or update internal registry state needed for the exported
  command to start a bound `volicord-mcp` process.
- Omission of `--read-only` uses workflow mode.
- When `--output` is omitted, the configuration is written to stdout. When
  present, `--output` names the exact output file.
- Exported configuration remains user-managed after export. Volicord must not
  claim that an arbitrary external host loaded, trusted, approved, initialized,
  or exposed it.

## User Channel commands

<a id="user-channel-commands"></a>
<a id="user-interaction-commands"></a>

`volicord user` commands provide a local CLI path for a human user to inspect
task status and answer pending user judgments through the `User Channel`. They
do not create an Agent Connection, install MCP host configuration, or make an
Agent Connection eligible to act as the user.

Project selection uses `--repo PATH` or the current working directory's
repository root. Task selection uses the active task by default; `--task active`
is explicit and `--task ID` selects a named task.

The ordinary text-mode judgment flow uses the numbered indexes printed by
`volicord user judgments` and `volicord user judgment show`. Stored judgment or
option identifiers are reference and JSON details, not required ordinary inputs.

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
- register or update projects, Agent Connections, Connection Projects, setup
  profile rows, or verification status rows
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

<a id="noninteractive-approval-behavior"></a>
## Noninteractive behavior

Noninteractive commands must fail instead of prompting when required user input
or host-controlled action is missing.

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
