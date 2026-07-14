# Administrative CLI reference

This document defines the local `volicord` administrative and bootstrap CLI.
The CLI prepares the `Volicord Runtime Home`, registers repository-root
projects, manages Agent Connections, provides the local `User Channel` command
path, and reports setup or connection diagnostics. Hidden lifecycle and
final-output commands exist only for generated host-integration wrappers. None
of these commands are public Volicord API methods.

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
- managed final-output authority-disclosure planning, rendering, and fallback
  behavior for generated host adapters
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

Labels follow the canonical vocabulary in
[Documentation Policy](../maintain/documentation-policy.md#surface-stability-labels).

| Surface | Stability | Notes |
|---|---|---|
| Supported administrative command names, options, stdout/stderr routing, process exit codes, dry-run behavior, and local User Channel command names | `stable` | These are local CLI contracts, not public Volicord API methods. |
| Managed final-output authority projection and typed support diagnostics | `beta` | Managed adapters may install and run a best-effort read-only projection. A host/profile support or release claim exists only when the shared evaluator reports `verified`; the current baseline does not make that claim for Codex or Claude Code final output. |
| `detective` profile setup, host-hook observation, session watcher observation, local consent availability reporting, and host-specific integration capability reporting | `beta` | These are supported cooperative observation surfaces with capability gates and owner-defined non-guarantees. |
| Hidden hook lifecycle namespace, generated wrapper details, conditional guard-integration staging and recovery sibling names, stored internal identities, host config keys, and process-binding values | `internal` | These details support generated host integrations and must not become normal user-facing command inputs or stable recovery-file names. |
| Human-readable init onboarding summaries, status summaries, doctor reports, connection verification reports, compact summary cards, action text, and diagnostic disclosures | `diagnostic` | JSON field presence and stable IDs are contracts only where this page explicitly requires them; text formatting is not a public API schema. |

## Command model

`volicord` is a local administrative/bootstrap executable. It is not a general
long-running server. The explicit `volicord serve` command is limited to the
local MCP transport process described in [MCP Transport](mcp-transport.md). The
`volicord inbox` command group is the user-facing user-action inbox and local
`User Channel` CLI adapter over selected Core methods; its command names remain
administrative CLI commands, not public Volicord API methods.

Supported baseline commands:

```text
volicord --help
volicord --version
volicord init --host codex|claude-code --repo PATH [--shared] [--profile record|detective] [--home PATH] [--mcp-command PATH] [--dry-run] [--json]
volicord status [--repo PATH] [--task active|ID] [--json]
volicord doctor [--json] [--privacy-footprint]
volicord diagnostics session [--session ID] [--json]
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
volicord mcp --stdio --discover-repository --host codex|claude-code
volicord mcp --check --connection <connection_id>
volicord mcp --check --connection <connection_id> --project <project_id>
volicord serve --transport local-http [--listen 127.0.0.1:8765 | --container-listen 0.0.0.0:8765] [--home PATH] [--connection <connection_id>] [--project PATH]... [--token-file PATH | --token TOKEN | --generate-token] [--allow-origin ORIGIN]
volicord export authority-bundle --output PATH [--repo PATH] [--json]
volicord changes reconcile [--repo PATH] [--task active|ID] [--dry-run] [--json]
volicord inbox [--repo PATH] [--task active|ID] [--json]
volicord inbox resolve <user-action-request-id> --choice <choice> [--repo PATH] [--note TEXT] [--json]
volicord inbox resolve <user-action-request-id> (--criterion ID | --claim ID) --artifact ID [--artifact ID ...] --summary TEXT [--contradicted] [--repo PATH] [--json]
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
- `volicord --version` writes `volicord <package-version> (build_id=<build-id>)`
  to stdout and does not require Runtime Home resolution. The package version
  remains SemVer. The separate build descriptor records Git commit and tree
  state, metadata source, target, exact builder-supplied profile or approximate
  Cargo profile class, optimization level, and debug state. It contains no
  build timestamp. A dirty descriptor labels the tree as dirty but does not
  identify the exact dirty contents.
- `--json` writes exactly one JSON document to stdout and does not mix human
  explanation into stdout. JSON output is the automation surface for result
  states, diagnostics, `summary_card`, `checks`, `actions`, and stable fields.
  Automation must not parse default human text output.
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
  transport process. Native runs accept only loopback addresses, and
  `--container-listen` is limited to Docker host-loopback publishing. Bearer
  authentication, Origin checks, and HTTP wire behavior belong to
  [MCP Transport](mcp-transport.md). This command is not a public network API,
  SaaS endpoint, multi-user server, or security boundary.

Not supported:

- The CLI has no general-purpose `server` or daemon command.
- `volicord serve` must not be treated as a public Volicord API service, SaaS
  endpoint, multi-user server, security boundary, or unauthenticated network
  service. `--container-listen` exists only for Docker host-loopback publishing;
  it is not a public host-interface or remote serving option.
- Administrative commands are not public Volicord API methods and must not be
  added to the public method list.
- Hidden Detective lifecycle commands are cooperative signals, not OS-level
  sandboxing or a security-enforcement proof. The hidden final-output command is
  a read-only display path, not a Detective signal. Neither is shown in normal
  top-level help.
- Text-mode user flows must not require users to type `project_internal_id`,
  `connection_internal_id`, host config keys, protocol envelopes, or stored
  registry fields.

### Output conventions

- `volicord init` may show a compact onboarding summary instead of a raw state
  dump. It names the host, profile, connection intent, host scope, repository,
  repository-file changes, Runtime Home, `Next:` checklist, limits, and JSON
  diagnostics command. A command under `Next:` appears on an indented line
  without prose punctuation.
- Status-like commands show a compact summary card or short sections before
  detailed diagnostics. These include `volicord status`, `volicord doctor`,
  connection status and verification, change reconciliation, and the inbox.
  When a command uses the common summary-card text renderer, its labels are
  `Task lifecycle`, `Volicord record effect for this command`, `Profile`,
  `Write Ticket`, `Evidence`, `Pending user actions`, `Unrecorded Product
  Repository changes`, `Close readiness`, `Transport`, and `Primary next
  action`.
- `Volicord record effect for this command` describes only whether the current
  command made a Core authority-state mutation. Human text renders the JSON
  value `read_only` as `none`, meaning `this command made no Core
  authority-state mutation`, and renders a committed record effect as
  `recorded`. The parenthetical explicitly says this does not describe
  product-file writes or Runtime Home write capability. Human text likewise
  renders `not_selected` as `not shown in this view` and a zero
  pending-user-action count as `pending (0)`.
- `Primary next action` names the compact card's immediate action when known and
  may include a follow-up verification command. It uses `none` only when the
  selected view has no known next action. Status and change-reconciliation text
  also shows the complete top-level `close_blockers` and `next_actions` counts,
  so the card action is not presented as the only action. `presentation_role`
  defines the public primary or additional display role; array order does not,
  and neither role grants authority. Stable blocker and action codes remain
  distinct from human sentences.
- Text summaries hide internal IDs unless the displayed action needs one. JSON
  retains the existing field names and stable values in `summary_card` and the
  top-level response; automation must not parse text formatting.
- Connection status and verification text starts with a title and compact
  `Status`, `Profile`, `Repository` or `Repositories`, `Checks`, `Next`,
  `Limits`, and `Diagnostics` sections. Connection add may also summarize host
  configuration or repository-file changes. Detailed hook state, CLI MCP
  preflight and handshake data, and host observations stay in JSON. Codex text
  may show separate checks for project trust, managed startup, managed
  `tools/list`, managed tool calls, active tool exposure, and the host MCP
  command.
- When judgments are pending, the user-only inbox text may show `Available
  answer paths:` for host prompt input, chat command capture, a local consent
  URL, and the CLI inbox. This line only routes the user; it does not record a
  judgment or let an Agent Connection act as the user. Inbox JSON uses
  `user_channel_availability` and its internal items use
  `answer_path_availability` for those facts. Public status text and JSON return
  only the exact three-field pending summaries and no User Channel availability
  or capture-path facts.
- Other diagnostic views may use concise `Result`, `Why`, `Next`, and `Does not
  prove` lines for `action_required` or degraded states. Connection status and
  verification use their section layout instead. In either layout,
  `action_required` is not a fatal CLI error. `Next` gives a concrete action,
  such as reloading or restarting the host, approving a host or project gate,
  repairing managed configuration, or running the displayed verification
  command afterward.

<a id="runtime-home-selection"></a>
## Runtime Home Selection

`volicord init` is the public first-run path for creating or reusing the local
installation profile. It creates or verifies the selected Runtime Home, records
the command paths later administrative, Agent Connection, and MCP process flows
use, registers the selected repository, and installs the selected host
connection. Init can select the Runtime Home path or MCP launch command while
performing repository setup and host connection. It cannot change the parent
shell's current environment.

The Runtime Home selected by init is part of every managed child-process
binding produced by that initialization. Personal and local MCP entries bind it
directly. Shared project entries keep the path clone-local by forwarding
`VOLICORD_HOME` through the host-specific portable form. Generated lifecycle
and final-output wrappers export the selected absolute Runtime Home and invoke
the installation profile's absolute `volicord_command`, and carry a versioned
managed-process binding marker. A managed `_hook` or `_final-output` child
validates that marker and requires its running executable to match the selected
profile command. A managed MCP or hidden child must not silently substitute the
host's platform-default Runtime Home. Init normalizes its selected Runtime Home
to an absolute, nonempty path before recording the profile or generating any of
these bindings.

When an existing installation profile's `volicord_command` is relative,
missing, or not executable, init replaces that unusable value with the
validated absolute executable running init before regenerating local wrappers.
An accessible executable already selected by the profile remains selected.
Doctor's `repair_volicord_command` action directs the user to invoke a working
Volicord executable and rerun init; `--mcp-command` changes the separate
`volicord_mcp_command` and is not the repair for this field.

The top-level setup status answers whether installation-profile preparation or
host connection still needs a named user action. Init may report
`action_required` after saving the Runtime Home and installation profile when
selected commands are not ready for future `PATH` lookup by shells or agent
hosts. JSON output must keep command-availability details and required actions
explicit. Default text output may present those actions through the compact
onboarding `Next:` checklist instead of a detailed diagnostic dump.

Arguments:

| Argument | Meaning |
|---|---|
| `--home PATH` | Selects the `Volicord Runtime Home`. Omission uses the platform default local runtime location. Init normalizes the selected path to an absolute, nonempty path before recording the installation profile or generating managed child bindings. The selected path must satisfy the Runtime Home/Product Repository separation contract before project state is used. |
| `--mcp-command PATH` | Stores the exact local MCP launch command in installation-profile `volicord_mcp_command` for personal/local connection bindings, verification, and other local MCP startup flows. Omission uses the MCP launch command selected by init. Generated local hook wrappers instead use the profile's separately recorded absolute `volicord_command`. A shared repository-visible MCP entry never embeds either path; it uses the fixed PATH-resolved `volicord` repository-discovery descriptor. |
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

`volicord doctor` is the read-oriented installation-profile diagnostic. Its
top-level status answers whether the current profile is usable.

It checks:

- the embedded package and build descriptor; unknown Git metadata, a dirty
  tree, an approximate profile, or incomplete compilation metadata is a
  warning. A dirty build remains usable but the descriptor does not identify
  its exact modified contents. Only known clean Git metadata with an exact
  builder-supplied profile and complete compilation dimensions passes this
  check
- Runtime Home access, registry schema, and installation-profile presence
- for repositories linked to a `personal` connection, whether Volicord's exact
  local-only paths are already in the Git index or exist without an effective
  ignore rule; this check reads path/index metadata only, never file contents
- stored command readiness and availability through `PATH`
- command-link or shim readiness when link metadata exists
- supported-host detection as a connection-verification concern
- for Detective profile records, hook-file installation, configuration health,
  runtime hook observations, effective detective health, and host reload needs

When stored command paths are executable, doctor may report `complete` while
also reporting availability warnings and `actions_recommended`. PATH or
command-link recommendations say when an existing agent host may need restart
or reload. Runtime-only capabilities such as session-watcher observation and
local web consent appear unavailable unless the reporting process owns that
runtime state. A connection-level `complete` result, successful CLI handshake,
or generated configuration does not create a host-capability verification.
Doctor may read and explain the bounded current verification basis, but it must
not create, refresh, or promote that basis. Without every invocation-bound
input, it reports local web consent as unavailable.

These checks do not prove OS enforcement, sandboxing, write prevention, product
correctness, or Close Status. Doctor does not create projects, install host
configuration, change connection mode, or answer user judgments. Human text may
summarize profile and observation limits; exact `selected_profile`,
`observation_summary`, and `control_surface` fields stay in JSON.

Text output includes the `build_id`. JSON includes top-level `build_id`, the
structured `build` object, a `checks[]` entry with `id=build_identity`, a
diagnostic disclosure, and a compact `summary_card`. The `build` object exposes
`package_version`, `git_commit`, nullable `git_dirty`, `metadata_source`,
`target_triple`, nullable `build_profile`, `profile_class`, `profile_exact`,
`opt_level`, nullable `debug`, and `build_id`. `metadata_source` is
`repository`, `environment`, or `unknown`; `profile_exact=false` means only the
Cargo `debug`/`release` profile class is known. The build object contains
neither the final executable digest nor a release evidence digest and is not a
release evidence manifest. Expected `evidence_artifact_sha256` for
host-capability evaluation must come from a separately verified external
exact-final-artifact release evidence manifest or receipt. JSON uses
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

`volicord project list` lists registered projects by user-facing name,
repository root, and status.

`volicord project rename NAME [--repo PATH]` changes the user-facing project
name for the selected repository. It does not change `project_internal_id`,
repository root, project home, or Core state.

`volicord project forget [PATH|NAME]` removes the selected project registration
only when doing so does not orphan active Agent Connection membership or project
state that an owner requires to remain addressable. Forgetting a project must
not delete the `Product Repository`, unrelated Runtime Home data, host
configuration, artifact storage owned by another remaining registration, or
Core authority rows that must be preserved.

<a id="evidence-capture-commands"></a>
## Evidence capture commands

The local registered-source fulfillment commands are:

```text
volicord evidence capture-command --intent ID [--repo PATH] [--json] -- PROGRAM [ARG...]
volicord evidence capture-tool --intent ID --pre-event ID --post-event ID [--repo PATH] [--json]
volicord evidence capture-connection --intent ID (--guard-event ID | --watch-observation ID) [--repo PATH] [--json]
```

All three commands select an active registered project, load the immutable
capture intent, and revalidate its current Task/Change Unit/scope/baseline,
target, Git workspace, requesting enabled Agent Connection and project access,
expiry, source selector, and exact canonical input or selector digest before
fulfillment. They reject an already fulfilled or consumed intent. The source observation must be
in `intent.created_at <= observed_at < intent.expires_at`, and receipt creation
must satisfy `observed_at <= receipt.created_at < intent.expires_at`.

`capture-command` is available on Linux and macOS. Other platforms reject it
before execution because Volicord does not have a bounded process-tree
termination primitive there. On supported platforms it hashes the complete
UTF-8 argument vector after `--`, checks it before execution, runs it with the registered repository root as cwd, and
records exit code plus stdout/stderr digest and byte counts without storing raw
arguments, environment, stdout, or stderr in the receipt. A process without a
numeric exit status creates no receipt. The runner streams both output channels
through hashing rather than accumulating them, applies one combined 16-MiB
output ceiling, and uses the intent expiry as its execution deadline. It kills
the isolated command process group and creates no receipt when the output
ceiling is exceeded or the expiry boundary is reached.

`capture-tool` requires exact registered `pre_tool` and `post_tool` guard-event
IDs for the same connection, session, guard installation, host invocation,
tool name, and canonical tool input. The pre event must not be denied, the post
must not precede it and must contain a complete tool response, and the tool name and input
digest must match the intent.

`capture-connection` accepts exactly one selected source and never auto-selects
among candidate facts. `--guard-event` requires the selected event's closed
event kind to match the intent selector, then hashes its redacted `raw_event`
for the receipt. `--watch-observation` requires the selected observation to
belong to the unique current active baseline for the intent-bound connection
and session, then hashes the safe selection of
observation/baseline/session/connection IDs, snapshot algorithm, digest and
entries, observed paths, change summary, and observation time for the receipt.
A missing or ambiguous current baseline rejects. The watch baseline and current
observation scans must both be complete and non-degraded with no skipped or
unreadable paths. The command strict-decodes the persisted baseline and current
snapshot entries, recomputes both snapshot digests and their diff, and requires
the stored observed paths and change summary to match that canonical result.
The exact selected source ID, observation time, and raw-event or watcher digest
are receipt facts; none is a caller-supplied intent field.

Fulfillment atomically reserves every underlying source fact with its receipt
and staging bytes. The same normalized host invocation, guard event, or watcher
observation cannot fulfill another capture intent or a different capture class
in the project.

Success creates one complete redacted receipt and one 24-KiB-bounded staging
handle that expires with the intent. Text and `--json` output expose the intent,
receipt, staging handle, completeness, kind, and safe observed outcome; they do
not expose raw source payloads. These commands record cooperative registered
local-source observations. They do not approve a command, grant permission,
provide OS isolation, attest the host or local principal, prove actor identity,
test sufficiency, correctness, final acceptance, or close readiness. They do
not advance Core state; `volicord.record_run` performs producer finalization.

<a id="connection-intents-and-hosts"></a>
## Connection intents and hosts

Agent Connection setup uses connection intents instead of low-level host config
scope names:

| Intent | Selected by | Meaning |
|---|---|---|
| `personal` | default | User-owned host configuration for the current user's ordinary local flow. |
| `shared` | `--shared` | Project-owned or project-shared host configuration stored as an explicit integration file in the selected `Product Repository`. |
| `global` | `--global` | User-wide host configuration for the selected host, with project access still constrained by registered repository roots and Connection Projects. |

`volicord init` defaults to `personal` and accepts `--shared` for an
explicit shared connection; it does not accept `--global`. Other connection
commands accept the intent options shown in their command syntax.
`--shared` and `--global` are mutually exclusive where both are supported.
When no intent flag is present, the intent is `personal`.

Connection modes:

- `workflow` is the default mode.
- `read-only` is explicit and exposes only read/project-discovery behavior
  through the Agent Connection.
- `volicord connection mode ... workflow|read-only` changes the stored mode for
  the selected connection without requiring users to edit host configuration.

The internal host configuration key `server_name` defaults to `volicord`.
Ordinary CLI flows do not expose a server-name option. A generated personal,
local, or user-wide host configuration may contain `connection_id` and, when
safely project-bound, `project_id` values derived from stored internal
identities so that the host can start `volicord mcp --stdio`; those values are
saved local process-binding details, not user authority tokens. A shared
repository-visible entry instead contains only the typed host-specific
repository-discovery descriptor. Text-mode command input uses the selected
host, intent, and repository root rather than internal IDs.

Ordinary `volicord connection add` commands use the saved profile in the resolved
Runtime Home instead of asking for an MCP command path or Runtime Home path.
Personal, local, or user-wide host configuration may carry that Runtime Home as
`VOLICORD_HOME`. Shared project host configuration must not embed a personal
Runtime Home path. It uses command `volicord` and exactly
`mcp --stdio --discover-repository --host codex` or
`mcp --stdio --discover-repository --host claude-code`. A generated Codex
`.codex/config.toml` entry also uses `env_vars = ["VOLICORD_HOME"]`; a generated
Claude Code `.mcp.json` entry uses
`"env": {"VOLICORD_HOME": "${VOLICORD_HOME}"}`. These are allow-forward forms,
not embedded Runtime Home paths. The host environment must resolve `volicord`
through `PATH` and provide the same nonempty, absolute `VOLICORD_HOME` selected
by init. Repository-discovery startup rejects a missing, empty, or relative
value before any platform-default substitution. It then resolves the canonical current Git
repository to the unique enabled shared connection and project registered for
that host in that Runtime Home. Repository metadata is never used to derive
local IDs.

<a id="agent-host-setup-and-init"></a>
### Host setup profiles

`volicord init --host codex --repo PATH --profile record` and
`volicord init --host claude-code --repo PATH --profile record` are the primary
first-run repository setup and host-connection examples for chat-first use when
host hook and session watcher observation is not being installed. Without an
intent flag, init creates a `personal` connection: Codex uses its user
configuration target, while Claude Code uses its repository-local CLI scope.
Adding `--shared` selects the project-scoped host layout in
`.codex/config.toml` or `.mcp.json`; that generated entry starts
`volicord mcp --stdio --discover-repository --host codex|claude-code` through
`PATH` and contains no local IDs, absolute command, or Runtime Home path. It
contains only the host-specific portable `VOLICORD_HOME` forwarding form:
Codex `env_vars = ["VOLICORD_HOME"]` or Claude Code
`"env": {"VOLICORD_HOME": "${VOLICORD_HOME}"}`.

Connection intent selects the managed Agent Connection target. Init separately
keeps its current repository integration inventory for both `personal` and
`shared`. The managed `AGENTS.md` block is repository guidance, while
`.volicord/policy.json` is an intent-independent local overlay. The policy
records `storage_scope=local_overlay` and the selected `connection_intent`; it
is never a shared repository projection. Shared Claude Code init also maintains
the repository `.mcp.json` projection; personal Claude Code init uses only the
host's local CLI target for MCP registration. The Detective profile adds the
supported repository hook configuration and rules plus local wrapper scripts.
Personal Claude Code detective settings use `.claude/settings.local.json`;
shared detective settings use `.claude/settings.json`. Those integration files
do not change the stored connection intent or its primary host scope.

For one Product Repository, init owns one selected supported host and one active
repository-local integration intent and profile. Re-running init with a
different supported host, the opposite `personal` or `shared` intent, or a
change between `record` and `detective`, is a managed migration rather than an
additional coexisting local integration. The migration must identify the prior
host, intent, and profile from the validated local policy, preserve unrelated
mixed-owner content, retire only matching Volicord-owned host entries, hook
handlers, managed blocks, and exact files, and fail on modified or unmanaged
lookalikes rather than deleting them. The prior Agent Connection may remain as
disabled history, but it must no longer be an enabled Connection Project for
that repository.

When the requested host or intent uses a different Agent Connection, init keeps
the requested project membership inactive during migration. A newly registered
requested connection uses `enabled=false`; an already-enabled requested
connection can continue serving its other projects but does not gain this
project yet. A prior connection that was eligible when migration began remains
eligible while init installs the requested host and guard projections; an
explicitly disabled prior remains disabled. Only after those steps succeed does one
Registry transaction revalidate the staged membership inventory, add the
requested project membership, enable the requested connection, and record the
requested guard installation. It removes a superseded project membership when
that connection still has other projects. When this is its last project, the
transaction disables the superseded connection but retains that one membership
as durable pending-host-cleanup inventory. A cleanup transaction revalidates
that the retained connection is still disabled with exactly that membership
and then releases the Registry write lock. After host retirement, a final
Registry transaction revalidates and removes the retained membership. External
host retirement cannot be rolled back if the final cleanup commit fails, but
the durable membership keeps the same init rerun discoverable and convergent.
Generic connection registration, enable/disable, and membership APIs cannot
create, overwrite, or remove this Store-owned marker. A fresh migration also
includes any older valid pending cleanup for the repository and rebinds it to
the new replacement, while unrelated disabled connections remain untouched.
The validated local policy connection is included even when it was explicitly
disabled. A profile-only migration reuses the current connection and does not
require that Registry transition. This staging boundary prevents a failed host
or intent migration from making both old and requested connections eligible for
the same project. A staging upsert never disables a target that another init
has concurrently enabled. Init classifies requested membership and exact
pending-cleanup state in one Registry transaction; if another attempt completed
the switch it resumes cleanup, and otherwise it fails without removing an
observed active requested membership.

For every init in a Git-backed Product Repository, Volicord recomputes one
managed block in the selected worktree's effective Git `info/exclude`. It
resolves a normal `.git` directory, a `.git` gitdir file, and a linked worktree
`commondir` to the actual common Git directory. The intent-independent part
always excludes `/.volicord/` and the dedicated Volicord wrapper scripts under
`.codex/hooks/` and `.claude/hooks/`; those files contain local process-binding
facts even for `shared` detective init. In a standalone worktree, `personal`
init additionally excludes the exact personal hook configuration and rule
paths. During a host, intent, or profile migration, init first keeps a fail-safe
union of the prior and requested local-only paths. It narrows the block to the new
intent only after the new projection is installed and every required retirement
succeeds. A failed or interrupted migration therefore keeps the broader local
protection, and rerunning the same init converges idempotently. A completed
`shared` migration removes the retired personal-only patterns; a completed
`personal` migration restores them.

The block does not exclude all of `.codex/` or `.claude/`, and it does not
exclude `AGENTS.md`, `.mcp.json`, `.claude/settings.json`, or shared Codex hook
configuration and rules. `.codex/hooks.json` is an exact-owned Volicord file for
this integration: different existing JSON is a conflict. Claude Code
`.claude/settings.local.json` is mixed-owner JSON whose unrelated settings are
preserved, but the host defines the entire file as a local surface, so a
personal init excludes the whole path. Init does not write or change the
tracked `.gitignore`.

A linked worktree's common `info/exclude` may contain only the
intent-independent paths because every sibling worktree reads it. Personal
Detective setup also needs personal-only hook configuration or rule paths that
cannot be hidden there without affecting a shared sibling. Init therefore
rejects that combination with
`LINKED_WORKTREE_PERSONAL_DETECTIVE_UNSUPPORTED` before applying repository or
Git-metadata files. Use `--profile record`, a `--shared` Detective integration,
or a standalone worktree.

`volicord doctor` reports local Git protection as
`checks[].id=personal_local_git_tracking`. It checks both the intent-independent
and known personal-only local paths for every connected Git repository,
regardless of the current policy intent. A missing or invalid policy is a
bounded audit error. A tracked local-only path or an existing local-only path
that is not ignored is a warning with bounded `tracked_paths` and
`unignored_existing_paths` details.

Doctor also reports `checks[].id=integration_intent_drift`. It compares the
validated local policy with enabled Connection Project inventory and inspects
the bounded, known prior-host and opposite-intent projection paths for
Volicord-owned content. It warns about a prior-host or opposite-intent
projection, simultaneous enabled integrations for multiple supported hosts or
intents, a policy intent or host that disagrees with enabled inventory, or
detective hooks left active under a record policy. A disabled opposite
host/intent connection that retains this project membership and the exact
Store-owned pending-cleanup marker is reported as
`findings[].kind=pending_host_cleanup`; rerunning the policy-selected init
finishes its host retirement and removes the durable cleanup membership. Doctor
reports a present but inexact reserved marker as
`findings[].kind=invalid_pending_host_cleanup_marker` rather than treating it as
resumable cleanup. That finding has
`actions[].id=restore_invalid_pending_cleanup_registry` with no command: init will not
mutate malformed Store-owned recovery state, so the operator must restore
`registry.sqlite` from a known-good Runtime Home backup or obtain
maintainer-assisted Registry repair first. Other intent-drift recovery reruns
init with the policy's exact host, intent flag, and profile. Doctor does not
change files or the Git index; index cleanup remains an explicit user action
that must not delete working-tree files.

`--profile` selects the public integration profile:

- `record` is the default. It writes MCP configuration, the managed `AGENTS.md`
  guidance block, policy metadata, and, for a managed Codex or Claude Code
  adapter with the implementation path, the minimal best-effort final-output
  handler used for authority disclosure.
  It supports cooperative Volicord workflow recording through MCP without the
  other host lifecycle hooks or a session watcher. Its final-output handler is
  read-only, does not activate Detective state, does not record a guard event,
  and does not gate final output.
- `detective` writes MCP configuration, the managed `AGENTS.md` guidance block,
  `.volicord/policy.json` hook command policy, supported project-local host hook
  and rule files, and records the host-hook/session-watcher observation state.

Detective-aware setup, status, verification, and doctor outputs report profile
and observation status. Human text may summarize this through command-specific
sections such as `Profile`, `Checks`, `Limits`, or `Diagnostics`; it is not a
raw diagnostic dump. JSON diagnostics carry the exact `selected_profile`,
`observation_summary`, and `control_surface` fields, including
`host_hooks_active`, `session_watcher_active`,
`cooperative_pre_tool_warning_available`,
`cooperative_pre_tool_denial_available`,
`unrecorded_changes_detectable`, `actor_identity_provable`, and
`os_enforced`. Connection status, Doctor, and every release-feature matrix cell
use the same exact six-key `host_feature_support` map:

```yaml
host_feature_support:
  native_user_action: HostFeatureSupportStatus
  local_web_user_channel: HostFeatureSupportStatus
  verified_tool_producer: HostFeatureSupportStatus
  registered_connection_observation: HostFeatureSupportStatus
  record_final_output: HostFeatureSupportStatus
  detective_final_output: HostFeatureSupportStatus
```

All six keys are required and no additional feature key is permitted. The
separate `final_output_authority_disclosure` object is selected-profile detail,
not a substitute for the map:

```yaml
final_output_authority_disclosure:
  support_status: HostFeatureSupportStatus
  configured: boolean
  configuration_verified: boolean
  required_subcapabilities: string[]
  subcapabilities: object<string, HostFeatureSupportStatus>
```

Only profile-applicable subcapabilities are present. Record requires
`authority_display` and `authenticated_exact_replay`. Detective requires those
two plus `block_finalization`. The retired `supported` and `verified` booleans
are not aliases. Configuration fields never promote `support_status`; that
field comes from the centralized exact-evidence and runtime evaluator. Current
Volicord output must report `os_enforced=false` and
`actor_identity_provable=false`.

Connection status always emits the six-key map at
`states.host_feature_support`. It emits selected-profile detail at
`states.final_output_authority_disclosure` only when the stored installation
profile is exactly `record` or `detective`. With `not_configured`, `mixed`, or
an unrecognized stored profile, it preserves that raw value in both
`states.selected_profile` and `states.control_surface.selected_profile`, emits
`states.final_output_authority_disclosure=null`, and never defaults to Record.
These typed fields are owned only by `states`; the `host_hook` config/audit
object does not duplicate them.

`connection add --dry-run --json` has no installed profile to select. Its
planned state therefore preserves `selected_profile=not_configured`, emits the
complete six-key map, emits `final_output_authority_disclosure=null`, and keeps
both typed fields out of `host_hook`.

Each terminal release-feature matrix cell, including passed, incomplete,
failed, and unavailable results, uses the unwrapped `host_feature_support` and
`final_output_authority_disclosure` field names. The map is always complete.
The detail is null without an exact profile and otherwise uses the exact
profile shape. A post-init result copies the product-produced init projection;
a preflight-unavailable result uses the centralized default projection with
configuration facts false. Unavailability does not erase or independently
reclassify static support status. `result=running` is the only non-terminal
recorder shape exempt from these fields. A terminal
`result=failed_before_completion` artifact uses the recorder's exact profile
hint and the centralized default projection; without an exact profile its
detail is null and it does not default to Record. Doctor emits
`states.host_feature_support_by_connection` as an array ordered by
`connection_id`; each element contains exactly:

```yaml
connection_id: string
host_kind: string
selected_profile: string | null
host_feature_support: HostFeatureSupportMap
final_output_authority_disclosure: FinalOutputAuthorityDisclosureDiagnostic | null
```

Doctor emits one row per readable stored Agent Connection and an empty array
when no connection row is readable. It sets both `selected_profile` and
`final_output_authority_disclosure` to null when the exact profile cannot be
selected; it does not invent a map from an unreadable connection. Exact shape
definitions remain in
[API State Schemas](api/schema-state.md#host-feature-support-diagnostics).

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
configuration for detective before rerunning init. `record` does not require
Detective lifecycle-hook installation or session watcher setup. Its implemented
best-effort final-output handler is a separate profile-independent display path;
its existence does not establish typed host support.

For Codex Detective setup, the generated `.codex/rules/volicord.rules` must be
loadable by the host: every `match` example must match its rule during host
rule validation. The prompt rule matches a three-token argv prefix: `sh`, `-c`,
and a closed choice of the exact generated Volicord hook script for
`session-start`, `pre-tool`, `post-tool`, `prompt-capture`, or `stop`. Because
Codex rules use prefix matching, trailing argv after those three tokens remains
inside the prompted set; generated Volicord hook calls supply no trailing argv.
Unrelated `sh -c` invocations and non-hook commands remain unmatched. This rule
is cooperative host prompting, not OS enforcement or proof that the host ran
the hook; those non-guarantees remain owned by [Security](security.md).

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
PATH` stores the exact MCP launch command in installation-profile
`volicord_mcp_command` when init must create or update that profile. Personal
host configuration uses the saved MCP command and Runtime Home as required by
the host adapter. Project-scoped
host MCP configuration selected by `--shared` still uses `volicord` from
`PATH` and forwards, but never embeds, the selected `VOLICORD_HOME`. Managed
local lifecycle and final-output wrappers instead pin both the selected
absolute Runtime Home and the installation profile's absolute
`volicord_command`.

Non-dry-run `volicord init`:

- initializes the Runtime Home if it is missing
- creates or updates the installation profile when needed
- registers or reuses the selected `Product Repository`
- creates or updates the matching Agent Connection and Connection Projects
  membership
- installs the primary managed host connection at the intent-selected target:
  the Codex user target or Claude Code local CLI target for `personal`, and
  project-scoped Codex `.codex/config.toml` or Claude Code `.mcp.json` for
  explicit `--shared`
- for an explicit shared connection, writes the exact portable
  `volicord mcp --stdio --discover-repository --host codex|claude-code`
  descriptor with the one host-specific portable `VOLICORD_HOME` forwarding
  form and no literal Runtime Home path or other environment entry
- writes or updates only the Volicord-managed block in `AGENTS.md`
- writes `.volicord/policy.json` as a `local_overlay` policy carrying the
  selected `connection_intent` and detective host hook commands that invoke the
  hidden internal hook namespace
- in a Git-backed repository, writes or updates the Volicord-managed local-path
  block in the effective Git `info/exclude` for both intents; the block always
  protects `.volicord/` and generated wrapper scripts, while a standalone
  `personal` init also adds personal-only hook configuration and rule paths
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
- rejects personal Detective initialization in a linked worktree when its
  personal-only paths cannot be isolated from sibling worktrees
- reports the required host restart, reload, approval, or trust action when the
  host must load the new MCP or detective host hook configuration

Re-running init is idempotent for matching Volicord-managed content and for a
partially completed host, intent, or profile migration. It updates managed
blocks, policy files, host MCP entries, and detective installation records without
duplicating them; an already retired matching projection is a successful no-op.
The prior `guard_installations.host_capability_json` is retirement authority
only when it satisfies the exact closed v2 contract in
[Storage Records](storage-records.md) and its profile, intent, host, adapter,
commands, and managed-script inventory match the owning installation row and
Agent Connection. If that prior capability is v1, malformed, inexact, or
owner-binding-invalid, rerunning init for the same host, intent, and profile is
a repair: init regenerates the current managed capability without decoding its
untrusted `files` inventory for retirement. Changing host, intent, or profile
cannot use that shortcut because it could retire files under contradictory
ownership. It fails before retirement with
`INTEGRATION_MIGRATION_INVENTORY_INVALID`. Recovery is to rerun init once with
the old host, intent flag, and profile to restore exact current inventory, then
rerun the requested migration.

Managed MCP entries or local wrappers generated before the Runtime Home binding
contract are stale managed projections. They receive no compatibility fallback
to a platform-default Runtime Home or a PATH-resolved wrapper command. The user
must rerun init with the same repository, host, intent, profile, and Runtime
Home; matching owned content is conditionally regenerated with the portable
forwarding directive or the pinned absolute wrapper bindings. This projection
refresh does not change Runtime Home DDL, the storage profile, or stored Core
records, so it requires no database or storage migration.
An older shared MCP entry with explicit connection/project bindings is eligible
for one conditional migration only when its current fingerprint matches the
stored managed fingerprint; successful migration replaces it with the portable
descriptor, and later reruns are no-ops.
If an existing target contains unmanaged content where Volicord requires
ownership markers or a managed fingerprint, or a previously managed retirement
target has changed, init must report a conflict instead of overwriting or
deleting it. Follow-up status and verification commands, dry-run diagnostics,
and repair commands preserve the selected intent: they include `--shared` only
for a shared init result.

If a migration fails after staging or file application begins, init exits with
a failed partial-application result rather than implying that nothing changed.
Text output names `Migration state: partial_application`, the stable migration
ID, observed requested connection and membership state, prior inventory state,
Registry transition state, host projection state, the cause, and rerun
arguments. JSON output has
`action=init`, `status=failed`, `retryable=true`, `retry_arguments`, `next`, and
a `migration` object with `migration_id`, `state=partial_application`,
`requested_connection_id`, `requested_connection_enabled`,
`requested_project_membership_active`, `prior_connection_ids`,
`prior_connection_inventory`, `prior_connection_states`,
`registry_transition`, and `host_projection`. Each
`prior_connection_states[]` entry has `connection_id` and its live `state`.
The two requested-state fields are live Registry reads: each is a Boolean when
readable and `null` when the failure also prevents that diagnostic read. Before
a connection Registry transition starts, `registry_transition=not_applied` and
`prior_connection_inventory=unchanged`. An error while the atomic transaction
is attempted conservatively reports both as `unknown`. A later verification or
verification-report failure reports `registry_transition=applied` and
`prior_connection_inventory=retired_for_project`. `host_projection` is one of
`partially_applied_or_pending_verification`,
`applied_registry_transition_unknown`,
`partially_applied_after_registry_transition`,
`cleanup_inventory_changed_after_registry_transition`, or
`applied_pending_verification` for those phases. While verified host cleanup
remains pending, `prior_connection_inventory=disabled_pending_host_cleanup`;
after it finishes, a later failure reports `retired_for_project`. Different
live states across prior connections report the aggregate value `mixed` and
remain distinguishable in `prior_connection_states`. Cleanup inventory that
fails its final revalidation reports `unknown`. The migration ID is stable for
the requested connection, Product Repository, intent, and profile, and the
retry arguments include the selected `--home` so a rerun returns to the same
Runtime Home. A profile-only migration reports
`registry_transition=not_required`. Host or guard files may already have been
created, updated, or retired, so the operator must resolve the named conflict,
inspect status or doctor when needed, and rerun the supplied init arguments.

On Unix, a newly created `.volicord/policy.json` uses user-only mode `0600`.
When a regular existing policy carries Volicord ownership metadata, a matching
init rerun repairs group or other permission bits by replacing it through the
same conditional managed-file path. An unmanaged policy file remains a
conflict. Before policy JSON is serialized, the MCP environment map accepts
only `VOLICORD_HOME`, `VOLICORD_MCP_LAUNCH`, `VOLICORD_MCP_HOST`,
`VOLICORD_MCP_CONNECTION_ID`, and `VOLICORD_MCP_PROJECT_ID`; secret-like and
all other environment keys are rejected without including their values in the
diagnostic. This allowlist is not a general secret-content scanner.
This is a local-overlay schema; none of these literal local-overlay environment
values is copied into a shared `.codex/config.toml` or `.mcp.json` Volicord
entry. The shared entry permits only the host-specific portable
`VOLICORD_HOME` forwarding form defined above.
The policy schema also requires `storage_scope=local_overlay`, the selected
`connection_intent`, a non-empty host/repository/connection identity set, an MCP
command with string arguments and string environment values, and a host-hook
enabled state that matches `selected_profile`. The top-level, `mcp`,
`host_hook`, required phase-command, and individual command objects use closed
field sets; unknown fields, phases, and non-string argument or environment
values are rejected. Audit treats a current recorded policy that violates this
shape or disagrees with its recorded intent as broken. The matching recorded
host capability must carry a string `connection_intent`; a missing or non-string
value is broken rather than a compatibility fallback.

The closed policy objects allow exactly these fields:

- top level: `schema`, `managed_by`, `storage_scope`, `connection_intent`,
  `host`, `repo_root`, `connection_id`, `guard_installation_id`,
  `selected_profile`, `mcp`, and `host_hook`
- `mcp`: `command`, `args`, and `env`
- `host_hook`: `enabled` and `commands`
- `host_hook.commands`: `session_start`, `pre_tool`, `post_tool`,
  `prompt_capture`, and `stop`
- each phase command: `command` and `args`

Applying a planned guard-integration managed file during init is a conditional
same-directory commit. This rule covers managed guidance, policy, hook, wrapper,
rule, and Git exclude files; host-adapter application of project-scoped MCP
configuration remains a separate boundary. Guard-integration planning captures a missing target
or a stable regular-file snapshot. Application pins the `Product Repository` and
each target-parent directory without following symbolic links, rejects a changed
or non-regular target, writes a sibling staging file, and uses the platform's
no-replace create or native replace/exchange operation. A create must not replace
a concurrently created target. An update is successful only after the installed
target matches the staged file, the displaced entry matches the planned
predecessor, and the displaced entry has been removed.

When a concurrent change or a native partial-failure state prevents that verified
result, the CLI attempts rollback only while every participating entry still
matches the state it inspected. A verified rollback removes its owned sibling
entries and reports failure. If automatic recovery cannot continue without
risking concurrent bytes, the CLI reports failure, names only recovery entries
that actually exist when inspected, and stops automatic deletion or replacement.
Internal sibling names have no stable naming or retention contract. Atomicity here
means the supported platform's same-directory namespace transition; it does not
make provisioning a transaction across multiple files or Runtime Home state, and
it is not a power-loss durability guarantee.

Git exclude planning rejects a symbolic-link `.git` marker, a malformed or
oversized gitdir/commondir control file, a non-directory Git target, or a Git
directory path that resolves through a symbolic link or non-canonical
component. For linked worktrees the pinned write root is the resolved common
Git directory rather than the Product Repository. The same no-follow parent,
stale-plan, conditional replacement, and recovery rules apply there.
The common block contains only intent-independent local overlay paths. A linked
personal Detective plan is rejected before managed files are applied because
its additional personal-only paths cannot safely be placed in common metadata.

The conditional-write checks cover changes to the managed target and its parent
path by ordinary concurrent writers. Implementation-private sibling names are
reserved to the active CLI attempt. A same-authority local process that discovers
and deliberately deletes or replaces those unpredictable names is outside this
cooperative-write guarantee. The CLI revalidates every state it can observe, but
these names are not an OS sandbox or an isolation boundary against another
process that already has write and delete authority in the directory.

Metadata handling for an existing guard-integration managed-file update is
platform-specific:

- On Linux and macOS, the sibling staging file remains at mode `0600` while its
  content is written. The CLI then reapplies and verifies the predecessor's POSIX
  mode, user ID, group ID, and all extended attributes exposed through the
  selected platform interface before commit, except that a Volicord-owned
  policy file uses mode `0600` as described above while retaining its user ID,
  group ID, and supported extended attributes. If it cannot read, reproduce, or
  verify that set, it rejects the update before reporting success. This covers an
  ACL only when the operating system represents that ACL through those extended
  attributes; it is not a guarantee for a separate metadata mechanism that the
  interface does not expose.
- On native Windows, the CLI denies new write sharing on the planned predecessor,
  reserves the backup name with a private create-new entry, and preserves a
  second hard link to the predecessor before using the default `ReplaceFileW`
  attribute and ACL merge behavior. It re-inspects the target, replacement,
  backup, and preserved predecessor after every native return. That native merge
  is not a portable metadata-equivalence guarantee.
- A newly created managed file receives the normal metadata of a new file in the
  selected directory. No cross-platform owner, ACL, extended-attribute, timestamp,
  alternate-stream, label, or other complete-metadata equivalence is implied.

<a id="volicord-agent-install"></a>
## Agent Connection commands

Connection selection uses host, intent, and repository root. When no intent
flag is present and a repository is selected, status, verify, mode, and remove
select the single matching connection for that host and repository across
intents. If more than one connection matches, the command reports an ambiguous
selector and the caller must add the matching intent flag. The command derives
or looks up the stored `connection_internal_id`.

- `volicord init`
  - Registry: prepares Runtime Home and the installation profile, registers the
    repository, creates or updates the default personal or explicit shared
    connection and membership, and records Detective profile observation state.
  - Host: installs the intent-selected managed MCP target plus the
    repository-local guidance, policy, and profile-dependent integration files
    for `codex` or `claude-code`.
  - Verification: runs observable host-configuration, MCP startup,
    initialization, and `tools/list` checks, then reports any host-controlled
    action.
- `volicord connection add`
  - Registry: registers the repository, creates or updates the matching
    connection, records intent and mode, and ensures project membership.
  - Host: installs managed configuration for the selected host and intent.
  - Verification: runs the observable host-configuration and MCP checks.
- `volicord connection list` reads matching connections, projects, and stored
  diagnostic verification state. It does not launch the host, rewrite
  configuration, or refresh host checks.
- `volicord connection status` reads one connection and reports its stored
  verification status and required actions. It does not launch the host or
  rewrite configuration.
- `volicord connection verify` inspects the managed target when available, runs
  observable checks, and stores the resulting verification state.
- `volicord connection mode` updates the stored mode. It rewrites host
  configuration only when regeneration is required and then reports diagnostics.
- `volicord connection remove` removes selected membership and, when no owned
  membership remains, the connection. It removes only matching managed host
  configuration and never deletes projects, Core state, Runtime Home, artifact
  storage, or unrelated host configuration.

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
| `complete` | Durable Agent Connection state exists, managed host configuration exists and matches the expected managed fingerprint, required host loadability and trust gates are satisfied, CLI MCP startup and initialization do not fail, and active Codex tool exposure is confirmed by managed host tool-call evidence or another explicitly reliable active-tool-exposure source. |
| `action_required` | Durable Agent Connection state and host configuration are present, but host trust, project approval, OAuth, reload, restart, command-link repair, installation-profile repair, or a comparable user-controlled action remains. |
| `failed` | The requested command or verification did not establish usable durable Agent Connection state, usable host configuration, or a required local prerequisite. |
| `dry_run` | The command reported the planned actions without persistent changes. |

Codex connection verification keeps these diagnostic concepts separate:

| Diagnostic concept | Text output surface | JSON diagnostic surface | Meaning |
|---|---|---|---|
| MCP configuration match | `MCP configuration` or `Current MCP configuration` | host check details and managed configuration fields, including `managed_config` | A personal entry matches the expected command, args, and local managed-launch markers. A shared entry matches the exact repository-discovery command and args plus only the host-specific portable `VOLICORD_HOME` forwarding form. Accepted tool-approval overlays do not change the match. Missing personal markers or missing shared forwarding may report `managed_config=unmanaged`; command, arg, or intent-specific managed-identity drift remains non-matching. |
| Codex tool approval policy | `Codex tool approval policy` when present | `verification.host.host_policy_overlay` and a `checks[]` entry with `id=codex_tool_approval_policy` when present | Codex-owned `tools.<known Volicord tool>.approval_mode` subtables appear as `kind=codex_tool_approval`. Diagnostics include `entries[].tool` and `entries[].approval_mode`. This does not prove host trust, active tool exposure, or running-session approval. |
| CLI MCP preflight and handshake | `CLI MCP preflight`, `CLI MCP handshake`, `Last CLI MCP preflight`, or `Last CLI MCP handshake` | `checks[]` entries with `id=cli_mcp_preflight` and `id=cli_mcp_handshake`, plus verification report fields | The CLI verification path directly launched and talked to Volicord's MCP server. This validates the CLI-observable MCP process, not active Codex tool exposure. |
| CLI MCP storage capability | `CLI MCP storage read`, `CLI MCP storage write`, and `CLI MCP effective tools` | `checks[]` entries with `id=cli_mcp_storage_read`, `id=cli_mcp_storage_write`, and `id=cli_mcp_effective_tools` when available | Storage capability observed through the CLI MCP verification process. This is separate from storage capability observed from a managed Codex host. |
| Codex project trust | `Codex project trust` | `verification.project_trust` and a `checks[]` entry with `id=codex_project_trust` when available | Codex user configuration marks the project `trusted`, `untrusted`, `unknown`, or otherwise leaves trust unconfirmed. |
| Managed Codex startup | `Managed Codex MCP startup` | `verification.host_runtime.managed_host_startup` and a `checks[]` entry with `id=managed_host_startup` when available | Volicord has or has not observed a managed Codex host process start the Volicord MCP server for the selected connection. |
| Managed Codex tool listing | `Managed Codex tools/list` | `verification.host_runtime.managed_host_tools_list` and a `checks[]` entry with `id=managed_host_tools_list` when available | Volicord has or has not observed a managed Codex host `tools/list` lifecycle event. This does not by itself confirm active tool exposure. |
| Managed Codex tool call | `Managed Codex tool call` | `verification.host_runtime.managed_host_tool_call` and a `checks[]` entry with `id=managed_host_tool_call` when available | Volicord has or has not observed a managed Codex host call a Volicord tool for the selected connection. This is the current completion evidence for active Codex tool exposure. |
| Active-session tool exposure | `Active Codex tool exposure` and `Next` action text when confirmation is required | `verification.active_tool_exposure`, `verification.host_runtime.active_tool_exposure`, `primary_next_action`, `actions[]`, and `connection.user_actions[]` | Whether active Codex tool exposure is confirmed, unconfirmed, or unknown. Manual probes, elevated probes, CLI preflight, direct handshakes, and source-less legacy observations do not confirm it. |
| Managed host storage capability | `Managed host storage read`, `Managed host storage write`, and `Managed host effective tools` | `verification.host_runtime.managed_host_storage` and `checks[]` entries with `id=managed_host_storage_read`, `id=managed_host_storage_write`, and `id=managed_host_effective_tools` when available | Storage capability observed from the managed Codex host lifecycle. This is separate from CLI MCP storage capability. |
| Host MCP command launchability | `Host MCP command` | `verification.host_mcp_command` and a `checks[]` entry with `id=host_mcp_command` when available | The configured MCP command is absolute, PATH-resolved, remote/executor-backed, unknown, or malformed, and can carry launch-risk details such as `host_path_unconfirmed`. PATH risk is a warning unless launch failure is proven. |
| Codex tool snapshot or listing issue | `Next` action text can direct the user to Codex MCP startup/tool-list logs | Codex host logs, not a Volicord-owned JSON field | Codex may know the MCP server exists or log `startup_complete` while the active session still has no cached tool snapshot or listed `volicord.*` tools. |

The accepted Codex tool approval policy overlay shape is:

```toml
[mcp_servers.volicord.tools."volicord.intake"]
approval_mode = "approve"
```

When a personal `volicord` server entry's command, args, and local managed
environment markers still match, or a shared entry's repository-discovery
command, args, and exact `env_vars = ["VOLICORD_HOME"]` forwarding still match,
that overlay alone must not make `managed_config` become `changed` or produce
the `mcp_config_changed` next action. A personal entry without its managed
markers or a shared entry without its exact forwarding directive may be
reported as unmanaged. Command, args, or intent-specific managed-identity drift
remains configuration drift.

Claude Code connection verification uses the runtime-facing Claude Code
adapter. For shared project setup, the managed identity is the project
`.mcp.json` `mcpServers.<server_name>` entry with the expected command, args,
environment, and managed fingerprint. For personal and global setup, Volicord
uses the Claude Code CLI target and compares `claude mcp get <server_name>`
output to the expected managed entry.

Claude Code verification can report these host states:

| Host state | Meaning |
|---|---|
| connected and matching | `claude mcp get <server_name>` reports a connected server whose command, args, environment, and scope match Volicord-managed configuration. |
| pending approval | Claude Code reports the MCP server is pending project approval; the result remains `action_required` until the user approves it in Claude Code. |
| rejected | Claude Code reports the MCP server was rejected. |
| missing | Claude Code does not report a configured MCP server with the expected name, or the project `.mcp.json` entry is missing. |
| changed or unmanaged | A server with the expected name exists, but command, args, environment, scope, fingerprint, or ownership does not match the Volicord-managed entry. |
| unavailable or unknown | The `claude` executable is unavailable, the command failed, or the output shape cannot be interpreted safely. |

This Claude Code verification proves only the managed configuration and the
host state that Claude Code exposes through `claude mcp get` or the project
configuration file. It does not by itself prove active Claude Code session tool
exposure, managed lifecycle startup, managed `tools/list`, managed tool-call
evidence, storage capability in the running host session, future tool choice,
or user approval beyond the reported host gate.

`verification.host_runtime` reports managed Codex lifecycle phase fields
`managed_host_startup`, `managed_host_tools_list`, and
`managed_host_tool_call`, each as `observed`, `not_observed`, or `unknown`.
It can also report `active_tool_exposure` and managed host storage diagnostics
when lifecycle evidence carries that data.
Only lifecycle events whose metadata has `host_kind=codex` and
`launch_origin=managed_host` for the selected connection and project count for
these fields; CLI preflight, direct handshake or probe launches, manual
launches, and source-less legacy observations do not satisfy them. A managed
`tools/list` event without a managed tool call leaves active tool exposure
unconfirmed.

### Verification output

- Connection status and verification text uses compact `Status`, `Checks`,
  `Next`, and `Diagnostics` sections. It shows the overall status, every
  attempted or blocked check, and a concrete next action when needed.
- For `action_required` or degraded Detective profile diagnostics, `Next` names
  the host reload, restart, approval, configuration repair, or follow-up
  `volicord connection verify ...` command. `action_required` remains a
  successful administrative result, not a fatal CLI error.
- JSON includes top-level `status`, `checks`, `actions`, and `summary_card`.
  Codex trust, runtime observation, command launch, and MCP handshake states
  remain separate.
- Detective profile diagnostics keep file installation, configuration health,
  runtime observation health, effective detective health, reload needs,
  prompt-capture availability, and the last known hook event separate. The
  exact JSON fields are listed under [Dry run and JSON output](#setup-output).
- Installed or configured files are not active hook observations until a
  matching event exists. Partial session-watch coverage is not full
  unrecorded-change detection.
- JSON uses `disclosure.guarantee_class=detective_observation` with stable
  `non_guarantees` for OS sandboxing, network isolation, malware defense, full
  write prevention, actor attribution proof, correctness proof, test
  sufficiency proof, and human review replacement.

A successful `volicord mcp --check` startup check, CLI MCP preflight, or direct
MCP handshake alone must not be described as a `complete` Agent Connection. It
is startup validation for the MCP process from the CLI-observable environment
only. It does not by itself prove that Codex, Claude Code, or another external
host has loaded, trusted, approved, initialized, or exposed the project
configuration. For Codex, it also does not prove that the active session
cached a tool snapshot or listed `volicord.*` tools.

<a id="authority-bundle-export"></a>
## Authority bundle export

`volicord export authority-bundle --output PATH [--repo PATH] [--json]`
exports an integrity-labeled copy of local Volicord records for one already
registered Product Repository. When `--repo` is omitted, the command resolves
the current directory to its Git repository root. `--output` names a directory
that must either not exist yet or already exist as an empty directory.

The command writes:

- `manifest.json`, describing the selected Runtime Home, registered project,
  exported record counts, artifact copy status, files, checksum path, and
  non-guarantees.
- `records.jsonl`, containing project `state.sqlite` storage rows as JSON
  Lines with `database`, `table`, and `row` fields.
- `artifacts/`, containing copied persistent artifact body files when the
  current local artifact store makes those bytes available. Artifact rows remain
  represented in `records.jsonl` and `manifest.json` even when a body is not
  copied.
- `checksums.sha256`, containing SHA-256 checksums for `manifest.json`,
  `records.jsonl`, `README.txt`, and copied artifact body files.
- `README.txt`, explaining the bundle contents and guarantee limits.

Rules:

- The authority bundle export reads the selected Runtime Home and project state
  without creating, registering, migrating, repairing, or updating Runtime Home
  records.
- `tool_invocations` rows retain their replay and audit metadata, but an
  `operation_category=user_only` row is exported with `response_json=null`.
  This export-only redaction does not mutate the source replay row. Private
  user text remains represented only by its canonical owner record when that
  record is part of the bundle; the duplicate exact user-only response is not
  an export projection of that text.
- The checksum manifest labels the exported copy. It is not proof that the
  Runtime Home was never modified before export.
- The bundle is not tamper-proof storage, cryptographic signing, an external
  audit log, correctness proof, test sufficiency proof, review completion
  proof, deployment proof, final acceptance, or residual-risk acceptance.
- JSON output reports the output path, bundle file paths, record count,
  artifact count, copied artifact count, and checksum-entry count.

<a id="external-host-configuration"></a>
## Host MCP configuration

The public `volicord export` surface is `volicord export authority-bundle`.
Volicord does not provide a public command that renders generic external MCP
host configuration. Supported host setup is performed through `volicord init`
and `volicord connection add`. Those commands write supported host
configuration directly when the selected host adapter owns a managed target.
Host-neutral or otherwise unsupported external hosts remain user-managed
configuration surfaces.

Rules:

- Supported managed host configuration is tied to an Agent Connection and starts
  a bound `volicord mcp --stdio` process.
- User-managed external host configuration may name the installed `volicord`
  executable and the `mcp --stdio --connection <connection_id>
  [--project <project_id>]` arguments after a supported Agent Connection exists.
- Volicord must not claim that an arbitrary external host loaded, trusted,
  approved, initialized, or exposed a user-managed configuration.

<a id="guard-hook-commands"></a>
## Internal host lifecycle and final-output commands

The hidden internal integration namespace is a local entry point for generated
host wrappers that can run a command during agent lifecycle or final-output
events. It is not shown in normal top-level help and is not a general
user-facing command group. Detective lifecycle commands inspect registered
project state, record host-observation events, and return a machine-readable
local decision. The final-output-only path performs the separately defined
read-only authority disclosure and records no observation. Neither path
replaces Core methods, user-owned judgments, write tickets, close-readiness
checks, host trust, shell approval, or OS-level sandboxing.

Generated final-output-only wrappers invoke the hidden `_final-output` command
with pinned repository, Agent Connection, guard-installation, host, profile,
policy-hash, and host-output arguments. These are generated process-binding
inputs, not a normal user command or public API request shape. The wrapper also
exports the init-selected absolute `VOLICORD_HOME` and invokes the installation
profile's absolute `volicord_command`; it does not use a bare PATH-resolved
command or an ambient host Runtime Home.

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

Every generated local lifecycle or final-output wrapper binds its managed child
before execution: it exports the absolute Runtime Home selected by init as
`VOLICORD_HOME`, exports the versioned `VOLICORD_MANAGED_PROCESS_BINDING`
marker, and invokes the installation profile's absolute `volicord_command`.
The generated value replaces rather than trusts an ambient host value. A
managed invocation of hidden `_hook` or `_final-output` requires that explicit
nonempty absolute Runtime Home, the exact current marker, and a running
executable that resolves to the profile command. It fails before
platform-default Runtime Home substitution or hidden-command handling when any
check does not match. A wrapper and stored capability from an older binding
shape are therefore not considered active merely because their stored hashes
agree; rerunning init regenerates the current owned wrapper.

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
redacted subject. Prompt-like fields are hashed or omitted by default. Prompt
capture records store the prompt hash and omit prompt text.

A host-reported `occurred_at` is observation metadata only. Volicord preserves
it as the owner-verified source fact when accepted; it is never an input to, or
an advance of, the persisted canonical Core UTC floor. It is not the authority
clock for current Task, write-ticket expiry, or UserAction effective-status
evaluation. Guard context, pending-action display, and prompt-command resolution
eligibility use the [canonical Core UTC clock](storage-versioning.md#canonical-core-utc-clock).
A delayed, replayed, future-dated, or clock-skewed host event cannot move those
current boundaries backward or forward. Store-generated transaction metadata
for recording the observation remains separate from the preserved host fact.

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
  chat user-action commands only when prompt-capture availability for the current
  host, project, and connection is `configured`, `observed`, or `active`, and
  the prompt contains exactly one explicit stored-form line. A choice form uses
  `Volicord: resolve A-3 --request USER_ACTION_REQUEST_ID --choice accept
  [--note "text"] #AB7K`. An Evidence observation form uses `Volicord: resolve
  A-4 --request USER_ACTION_REQUEST_ID (--criterion ID | --claim ID) --artifact
  ID [--artifact ID ...] --summary "text" [--contradicted] #AB7K`. The durable
  request ID is required; `A-N` remains a displayed task-local binding and is
  checked against the stored request's Task even when another Task later becomes
  active.
  Session-start and every other Agent-facing host context show an existing
  pending request only through `AgentSafeUserActionRequestSummary`: durable
  request ID, `pending`, and next actor/User Channel. They omit the request-bound
  `A-N`, verification code, command template, question, context, and complete
  Core-derived form. A complete form or verification credential may be rendered
  only by a separately verified User Channel surface that remains outside model
  context; prompt-capture availability by itself does not establish that
  surface. If such a user-only rendering exceeds its 32 KiB bound, the path is
  unavailable, no partial form is shown, and another User Channel path remains
  available. Agent-facing fallback text contains only generic `volicord inbox`
  guidance. The terminal `volicord inbox` path remains user-only and displays
  the complete canonical form. This routing check is not a general secret
  scanner or an OS security boundary.
  Unsupported, unconfigured, reload-needed,
  or degraded prompt capture returns structured non-recording output such as
  `prompt_capture_unsupported`, `prompt_capture_not_configured`, or
  `prompt_capture_reload_required`, with one next action. Non-command prompts
  proceed normally only when prompt capture is available. Malformed, ambiguous,
  unknown, missing-code, wrong-code, unresolved-stale, conflicting-duplicate,
  wrong-project, or wrong-connection user-action commands return `deny` without
  recording a resolution. An exact retry with the same request, connection,
  channel submission identity, and canonical resolution returns the original
  committed response without effects, including after a later basis change
  makes the resolved request effectively `stale`.
  A valid command resolves the addressed pending user action through the local
  `User Channel` with `actor_source=local_user` and
  `resolved_verification_basis=user_prompt_submit_hook`,
  omits the full prompt text from prompt-capture storage, and returns
  model-visible recorded-context output instead of treating the command as
  ordinary agent instruction. The `recognized_user_action_command` object
  reports the stored action type, safe selected identifiers, whether private
  `note` or Evidence-observation `summary` text was omitted, and
  `replayed=true` only when the same durable resolution was returned from
  idempotent replay. Local diagnostics count every successful recognized
  command as Core-reached, but count only a non-replay as Core-committed.
- `stop` checks whether the active task can safely be treated as complete.
  Before it can return `allow` for an active task, it obtains a current Core
  status response with close data. Only a response with
  `base.response_kind=result` and `base.effect_kind=read_only` is eligible for
  this decision. Its Core-owned `AuthorityReceipt` must match the refreshed
  status state version, project, Task, Task reference version, scope revision,
  current Change Unit, evidence gate, close state, close blockers, and the
  guard context captured for this hook event. A mismatch, non-result response,
  or response whose required status fields are missing or malformed returns
  `deny` with reason code `authoritative_refresh_failed`.
  For that denial,
  `result.close_status.authoritative_refresh` contains only the recognized
  `response_kind` value, or `null` when it is missing or malformed, and an
  `error_codes` list containing valid public `ErrorCode` values. It must not
  copy Core error messages, error details, or request or response bodies. A
  failure to call Core remains an internal hook-command failure on the existing
  error path. The refresh is read-only and a refresh-failure denial creates no
  Core state effect beyond ordinary hook-observation recording. A valid result
  injects the matched receipt as `result.close_status.authority_receipt` and
  returns `deny` when close-readiness blockers remain, user-owned judgments are
  pending, or unresolved unrecorded changes remain; otherwise it returns
  `allow`.

<a id="managed-final-output-authority-disclosure"></a>
### Managed final-output authority disclosure

Generated Codex and Claude Code adapters may install the implemented final-
output handler when their configuration prerequisites are available, but the
handler is claimed as supported only when the centralized evaluator returns
`support_status=verified` for the selected profile feature. Best-effort
operation while `implemented_unverified` retains that status and is never
reported as verified. The Codex handler requires a local Git work tree. A
non-Git Codex Record initialization remains successful, installs no handler,
and routes the user to the applicable `volicord status` fallback. In Record the
handler is a non-observing, non-gating read path. In Detective it runs alongside
the separate Stop decision; its receipt source is never the persisted guard
result.

For every delivery, including an exact event replay, the handler:

1. read-only verifies the enabled Agent Connection, selected-project
   membership, pinned Product Repository, host kind, and installed profile;
2. performs a new Core status read with close data;
3. uses the shared Core validator to require a result, `read_only`, non-dry-run
   response and to match the receipt's project, Task, Task reference version,
   `state_version`, scope revision, current Change Unit, evidence gate, close
   state, complete close-blocker set, and next action to that status result; and
4. projects the result under the complete-receipt-or-fallback rule in
   [Template Bodies](template-bodies.md#final-output-authority-disclosure-body).

The receipt branch places the complete validated `AuthorityReceipt` as
deterministic whitespace-free canonical JSON in the top-level `systemMessage`
fixed UI surface. The 8 KiB limit applies to the complete serialized host-native
JSON response after outer JSON escaping and including its terminating LF, not
merely to the inner message. Exactly 8,192 bytes is allowed; 8,193 bytes uses the
fallback. The fallback is independently measured against the same limit. No
branch truncates or partially emits receipt JSON.

An oversized receipt, failed refresh, rejected or malformed result, connection
or adapter mismatch, or unavailable/degraded adapter produces only a bounded
safe fallback. When a Task is identified, it reports safely available project,
Task, and `state_version` coordinates and the exact
`volicord status --task TASK_ID --json` command. When there is no active Task,
it explicitly says so and gives `volicord status --json`. It does not copy Core
error messages or details, request or response bodies, raw host event text, or
model-authored final prose.

The refresh and rendering path creates no Core state or version change, event,
replay row, guard event, Agent Session, installation activation, watcher state,
or host observation. An exact Detective replay reuses the immutable historical
Stop decision while this separate display refreshes current authority again.
The model's final prose, a mutation receipt, a cached Stop result, and generated
configuration are never current-authority inputs.

`systemMessage` is a separate host fixed UI surface. It is not model context and
does not inject text into the model's final prose. `generic`, user-managed,
unsupported, missing, inactive, or degraded adapters must report the capability
as unavailable and show the applicable status fallback; they must not claim
that a generated file proves the host displayed the receipt.

## Change reconciliation command

`volicord changes reconcile [--repo PATH] [--task active|ID] [--dry-run]
[--json]` is the local recovery command for unresolved unrecorded Product
Repository changes.

Selection and dispatch:

- `--repo PATH`, or the current directory when omitted, selects the project.
- The active Task is the default; `--task` can select another Task.
- The command calls `volicord.reconcile_changes` with
  `actor_source=local_user` and `operation_category=local_recovery`.
- Output includes the compact summary card and counts for resolved changes,
  pending user actions, unresolved changes, all top-level close blockers, and all
  top-level next actions. The counts and array order do not assign presentation
  roles; the structured `presentation_role` field does. Rejected Core responses
  remain rejected CLI results.

With `--dry-run`, text and JSON show planned automatic resolutions, changes that
need user judgment, judgment requests that would be created, projected close
blockers, next actions, and the preview disclosure. The preview does not prove
actor identity, intent, or correctness. It does not advance
`project_state.state_version`, write mutation or replay records, resolve close
blockers, create user judgments, stage artifacts, or attach artifacts.

Without `--dry-run`, the command may resolve deterministic changes or create
pending user-owned judgments. It does not answer a judgment, accept a change for
the user, prove identity, intent, correctness, review or test sufficiency, or
complete close readiness. When it creates a pending judgment, the user answers
  through the user-action inbox and reruns the command.

## User Channel commands

<a id="user-channel-commands"></a>
<a id="user-interaction-commands"></a>

`volicord inbox` commands provide the local CLI path for a human user to list
and resolve pending `UserActionRequest` records through the `User Channel`.
They do not create an Agent Connection, install MCP host configuration, or make
an Agent Connection eligible to act as the user.

When the initialized MCP client declares host prompt support, native host
prompt input is the preferred User Channel input method for a compatible
pending action created through `volicord.request_user_action`. Agent-facing
fallback guidance never shows a request-bound chat command, verification code,
complete form, or loopback bearer URL. Local web consent is selected only when
the shared evaluator confirms the managed non-generic stdio path, ready
listener, exact boolean declaration, and current unexpired exact-match
`outcome=passed` host-capability verification whose canonical UTC interval
satisfies
`observed_at <= created_at < expires_at <= observed_at + 86,400 seconds` and
whose current evaluation uses `observed_at <= now < expires_at`. Twenty-four hours is
the maximum freshness window, not a default lifetime or attestation period.
The verification's `evidence_artifact_sha256` must exactly match the expected
digest from the separately verified external exact-final-artifact manifest or
receipt bound to the same capability, host/client, adapter, build, source,
target, and executable digest. A missing, unknown, malformed, unverified, or
mismatched manifest fails closed. The current adapter has no trusted manifest
acquisition path, so production local-web selection uses CLI inbox fallback.
The short-lived URL is delivered
only in the namespaced top-level tool-result `_meta` handoff. Otherwise the
Agent receives generic CLI inbox guidance. The
terminal `volicord inbox` commands remain the complete-form CLI input and
manual-inspection path when another verified User Channel is unavailable,
disabled, degraded, or inappropriate for the action form.

Project selection uses `--repo PATH` or the current working directory's
repository root. Task selection uses the active task by default; `--task active`
is explicit and `--task ID` selects a named task.

The ordinary text-mode flow centers on the stable action-request identifier and
the Core-derived capture form printed by `volicord inbox`. Stored request refs,
candidate refs, and additional capture-path details remain available in JSON
output.

Commands:

- `volicord inbox` lists pending `UserActionInboxItem` entries for the selected
  task. Each item includes the request ID, action kind, question, Core-derived
  capture form, required/optional status, preferred capture path, and available
  fallbacks.
- `volicord inbox resolve <user-action-request-id> --choice <choice>` resolves a
  judgment-form request through `volicord.resolve_user_action`. It sends only
  the selected stored option ID and optional `--note`; Core derives the machine
  action, outcome, and kind-specific authority facts from the stored request,
  option, and current compatible basis.
- `volicord inbox resolve <user-action-request-id> (--criterion ID | --claim ID)
  --artifact ID [--artifact ID ...] --summary TEXT [--contradicted]` resolves an
  evidence-observation form. The target and every artifact must be selected
  from the stored capture form, `--artifact` must select a non-empty unique
  subset, and relevance defaults to `supported`. The resulting resolution is
  evidence provenance, not a judgment or final acceptance.
Each `resolve` call submits only the variant represented by the stored closed
capture form. The CLI derives `actor_source=local_user` and
`operation_category=user_only`, omits `expected_state_version` so Core can pin
the preflight version, and generates a request-bound `channel_submission_id`
for replay. It cannot use judgment flags for an observation form, observation
flags for a judgment form, caller-supplied options or candidates, or a
free-form direct observation without a pending request.

Resolving one action records only the addressed action. Final acceptance and
residual-risk acceptance remain separate action kinds, and an
`evidence_observation` resolution never substitutes for either.

Status and inbox list output expose selected owner state for the user's next
action, including a compact `summary_card` when the view can compute one.
When pending actions are present, text output also summarizes available capture
paths so unavailable host prompt input does not hide chat capture, local
consent, or CLI inbox paths. Listing and opening do not create evidence, final
acceptance, residual-risk acceptance, or close readiness. Only
`volicord inbox resolve` inserts the immutable resolution for the addressed
pending request through its stored capture form.

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
- init JSON reports the selected `connection.connection_intent` and
  `connection.host_scope`; dry-run and applied output use the same resolved
  values
- `summary_card`: stable compact summary data for status-like diagnostic or
  user-channel outputs that compute a summary card
- connection status and verification JSON can expose separate Codex diagnostics
  for CLI MCP preflight and handshake, `project_trust`, `host_runtime`,
  `active_tool_exposure`, `host_mcp_command`, CLI MCP storage capability, and
  managed host storage capability. Matching `checks[]` entries include
  `cli_mcp_preflight`, `cli_mcp_handshake`, `codex_project_trust`,
  `managed_host_startup`, `managed_host_tools_list`,
  `managed_host_tool_call`, `active_tool_exposure`, `host_mcp_command`,
  `cli_mcp_storage_read`, `cli_mcp_storage_write`,
  `cli_mcp_effective_tools`, `managed_host_storage_read`,
  `managed_host_storage_write`, and `managed_host_effective_tools` when
  available. These diagnostics distinguish trust state, CLI MCP startup,
  managed host runtime observation, active tool exposure, command launch risk,
  and storage capability from CLI MCP handshake success.
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
  `native_host_output_adapter_config_verified`, and
  `direct_file_write_matcher_coverage` to show the stricter host-hook
  prerequisites. `local_web_consent_available` must remain `false` when the
  command cannot observe the current invocation's exact client declaration,
  launch origin, listener readiness, and persisted verification tuple; stored
  connection verification alone must not set it to `true`. When watcher
  diagnostics are reported, JSON must also expose
  `watcher_status`, `watcher_baseline_created_at`,
  `watcher_coverage_start_at`, `watcher_coverage_basis`,
  `watcher_partial_coverage_warning`, and `watcher_scan_summary`.
  `watcher_scan_summary` reports `files_scanned`, `files_skipped`,
  `unreadable_paths_count`, `degraded_reasons`,
  `degraded_reason_counts`, `skipped_paths_sample`,
  `skipped_paths_truncated`, `default_excluded_paths`,
  `max_file_size_bytes`, `max_file_count`, `follows_symlinks=false`,
  and `not_full_filesystem_monitoring=true`.

`volicord doctor --privacy-footprint` is a read-only diagnostic report for
the selected `Volicord Runtime Home`. Text and JSON output summarize the
categories and counts of stored Runtime Home data and list non-proofs such as
actor attribution, write prevention, tamper-proof audit, full filesystem
monitoring, OS enforcement, correctness, test sufficiency, review completion,
final acceptance, and residual-risk acceptance. The command must not print
stored row bodies, Product Repository file contents, or prompt text.
When `diagnostics.sqlite` is present, the stored-category summary includes its
bounded identifier, categorical, counter, byte-size, and latency footprint and
states that the excluded diagnostics content categories are not stored.

Setup and doctor JSON must include `status_meaning` so diagnostic consumers can
distinguish setup action status from installation-profile health.
Doctor JSON must separate blocking local repairs in `actions_required[]` from
warning-only follow-up in `actions_recommended[]` when the top-level status
remains `complete`.

<a id="session-diagnostics"></a>
## Session diagnostics

`volicord diagnostics session [--session ID] [--json]` reads bounded local
operability aggregates from the selected Runtime Home. Without `--session`, it
selects the session with the latest diagnostic update. A Runtime Home with no
diagnostic database or no matching session returns `status=no_data`; the read
does not create the database. Normal setup preconditions can read the
installation profile, but the report itself reads only `diagnostics.sqlite`
and does not open a project `state.sqlite`.

Safe aggregate collection is enabled by default for MCP tool-call boundaries
and supported guard-hook observations. Diagnostic persistence is best effort:
failure, corruption, or write denial in `diagnostics.sqlite` must not change an
MCP result, guard decision, User Channel result, Core commit, or CLI authority
result. The command is observational and must not change `state_version`,
evidence or assurance, close readiness, a `UserActionRequest` or
`UserActionResolution`, or any authority row.

JSON output has these top-level fields:

- `schema_version`, `status`, and `scope`
- `storage`, including `database_file`, `retention_days`, `max_sessions`, and
  `max_events_per_session`
- `redaction`, declaring the excluded content categories
- `authority_isolation`, declaring that the report does not open project state
  or change authority categories
- `current_build`, with this CLI's package version and `build_id`
- nullable `session`

An available `session` includes bounded session, connection, project,
transport, host-kind, producer-build, and timestamp identifiers; `tools[]`;
`totals`; `user_channel_counts`; and `fallback_counts`. Per-tool aggregates
report call count, total/maximum/average latency in microseconds, request and
response byte counts, schema-validation failures, retries after a validation
failure, Core-reached count, Core-committed count, and replay count. Totals also
report observed product-file-write count and authoritative-refresh failures.
These are local diagnostic observations, not evidence, actor attribution,
write authority, host conformance, or a correctness claim. A fallback count
means that path was selected for a pending answer; it does not mean the user
  resolved the user action through that path.

The default store accepts no prompt body, Product Repository path or file
content, error body, secret text, user-action question or capture form,
choice note, or Evidence-observation summary.
It stores only allowlisted identifiers and categorical outcomes plus counters,
byte sizes, latency, and build identity. Content-bearing or detailed trace mode
is not supported. Any future detailed trace must be a separately documented,
explicit opt-in surface with its own retention and redaction contract; it must
not silently widen this default collection. Exact placement and retention are
owned by [Storage Records](storage-records.md#local-diagnostics-store).

<a id="noninteractive-approval-behavior"></a>
## Noninteractive behavior

Noninteractive commands must not prompt for missing user input or
host-controlled action. They must report the missing condition through the
normal result model: recoverable user or host action as `action_required`,
usage mistakes as exit code `2`, and conflicts or runtime failures as exit
code `1`.

Rules:

- Project-scoped host MCP configuration requires the explicit `--shared`
  command path. Init's separate repository-local guidance, policy, and
  profile-dependent integration files are authorized by the explicit init
  command for both supported init intents and remain limited to the managed
  files that command previews.
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
