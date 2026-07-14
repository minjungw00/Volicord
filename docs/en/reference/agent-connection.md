# Agent Connection Reference

This document defines Agent Connection and current connection context boundaries
for local MCP host integrations. It defines how an Agent Connection, its
connection intent, connected projects, connection mode, `actor_source`, and
`operation_category` are interpreted before a request enters Core.

## Owns / Does Not Own

This document owns:

- Agent Connection meaning and Connection Projects membership rules
- connection intent meaning: `personal`, `shared`, and `global`
- current connection context boundaries for MCP-host calls
- `actor_source` and `operation_category` provenance boundaries
- User Channel versus Agent Connection boundaries for authority-bearing
  user-action resolutions
- repository-root project selection and project availability boundaries at the
  Agent Connection layer
- agent context transfer rules between owner results and an Agent Connection
- managed final-output authority-disclosure capability and connection boundary
- fallback display when the selected Agent Connection or current connection
  context is unavailable, mismatched, stale, or insufficient

This document does not own:

- API request envelopes, response branches, schema shapes, or operation-category
  value names; see [API Schema Core](api/schema-core.md),
  [API Methods](api/methods.md), method owners, and
  [API Value Sets](api/schema-value-sets.md)
- `volicord mcp --stdio` startup, process environment, stdio framing,
  startup validation, response wrapping, or shutdown; see
  [MCP Transport](mcp-transport.md)
- administrative setup, connection, status, verification, mode, remove,
  project, and authority-bundle export commands; see
  [Administrative CLI](admin-cli.md)
- storage layout, artifact lifecycle, or staged-handle validation; see storage
  and artifact owners through [Reference Index](README.md)
- security guarantee meanings or access-boundary wording; see
  [Security](security.md)
- authority versus projected display rules; see
  [Projection and template display boundaries](projection-and-templates.md)
- rendered body wording, public display labels, or template phrasing; see
  [Template Bodies](template-bodies.md)

<a id="surface-stability"></a>
## Surface Stability

Labels follow the canonical vocabulary in
[Documentation Policy](../maintain/documentation-policy.md#surface-stability-labels).

| Surface | Stability | Notes |
|---|---|---|
| Agent Connection meaning, connection intents, Connection Projects membership, connection modes, and current connection context boundaries | `stable` | These are local integration contracts, not OS permissions or user authority. |
| Managed host lifecycle and verification observations | `beta` | The observations are supported but remain host- and capability-dependent. |
| Managed final-output authority projection and support-state evaluation | `beta` | Managed adapters may run a best-effort fixed-UI projection. Support remains feature- and evidence-specific, and only `support_status=verified` establishes a current support claim. |
| Stored identities, process-binding values, host configuration keys, and derived invocation metadata | `internal` | Public MCP inputs must not expose these details as caller-owned authority. |
| Human-readable status, verification, fallback, and guidance text | `diagnostic` | Exact fields are stable only where a focused owner explicitly defines them. |

## Agent Connection

An Agent Connection is a local MCP host connection unit stored under the
`Volicord Runtime Home` with `connection_internal_id`. Generated MCP startup
uses the `connection_id` process-argument spelling, but ordinary text-mode user
flows select the connection by host, connection intent, and repository root
through the commands owned by [Administrative CLI](admin-cli.md).

One `volicord mcp --stdio` process is bound to one Agent Connection. Generated
host configuration may contain a `connection_id` process-binding value derived
from the stored `connection_internal_id` so the host can start that process, but
that value is not a user authority token and is not required as a normal command
input.

The registry stores the connection's internal identity, host and intent,
configuration target, mode, enabled state, managed fingerprint, configuration
verification state, append-only host-capability verification history, and
related metadata. Exact record fields belong to
[Storage Records](storage-records.md) and [Storage DDL](storage-ddl.md). The
internal host configuration key `server_name` defaults to `volicord`.

<a id="lifecycle-and-state-boundaries"></a>
## Lifecycle And State Boundaries

An Agent Connection lifecycle spans several state surfaces. A command can change
one surface without changing the others.

- Installation profile: Runtime Home registry installation records store the
  selected Runtime Home identity and MCP command location. `volicord init`
  creates or reuses this required local configuration. It is not host trust, a
  user judgment, or a public API method.
- Agent Connection registry state: `agent_connections` stores management state.
  Init and connection commands create, update, verify, change, or remove it.
  Registry state is not the host configuration and does not prove that an
  external host loaded, trusted, approved, or exposed the MCP server.
- Connection Projects membership: `connection_projects` stores the explicit
  project allowlist. Init and connection add can add or validate membership;
  removal can delete it. `volicord project use` registers a project but does not
  add membership. Membership changes do not delete project or Core state.
- Host configuration: `config_target`, or a user-managed generic target, names
  the external host surface. Init and connection add install managed content;
  removal deletes only safely matched managed content. This configuration starts
  `volicord mcp --stdio` but is not registry state.
- Verification state: `last_verification_status` and the output owned by
  [Administrative CLI](admin-cli.md#agent-connection-result-states) record the
  latest checks. Host configuration, hook safety, MCP startup, initialization,
  and `tools/list` checks remain distinct from managed lifecycle observation and
  active-session tool exposure.
- Host-capability verification history: a bounded external live-host result may
  create an immutable verification row for one exact connection, capability,
  built-in host/client version, adapter profile, Volicord build, exact source
  revision, target, executable digest, managed fingerprint, evidence digest,
  and half-open validity interval. A later immutable row with
  `outcome=revoked` can become current and invalidate an
  earlier pass without changing history. Configuration verification does not
  create this history and cannot substitute for it.
  Every interval must satisfy
  `observed_at <= created_at` and
  `observed_at < expires_at <= observed_at + 86,400 seconds`; a pass also
  requires `created_at < expires_at`. Twenty-four hours is the maximum freshness
  window, not a default lifetime or attestation period; a publisher may choose
  a shorter expiry.
  For a passing built-in stdio row, `host_version` and `client_version` are not
  independent observations: both must equal the exact runtime
  `clientInfo.version` and the live artifact's installed-host version. The
  `source_revision` must be exact lowercase 40- or 64-hex; `unknown` cannot
  pass. If either equality cannot be proved, the result is not passing.
  An exact same-ID/same-content publication is idempotent and never moves a
  newer current pointer backward; same ID with different content conflicts.
- Invocation eligibility: the MCP adapter derives it at startup and for each
  public tool call. `enabled`, project availability, `connection.mode`, and
  `operation_category` affect it. Registry or project changes can make a call
  ineligible without rewriting host configuration.
- Removal: `volicord connection remove` can remove managed host content,
  membership, and sometimes the Agent Connection. It must not delete a Product
  Repository, project registration or state, Core records, Runtime Home,
  artifact storage, or unrelated host configuration.

Volicord-managed host configuration means Volicord owns and fingerprints
specific generated host configuration content. It is not the same as
the internal host-hook distribution state recorded by a host contract. That
state describes a verified source for hook-related implementation records; it
is not a public integration mode or security boundary.

Agent Connection verification keeps these layers distinct:

- host managed config identity: the managed server name, command, args,
  environment, scope, and fingerprint expected for the selected connection
- host trust, approval, or pending state: host-owned gates such as trust,
  project MCP approval, OAuth, pending approval, or rejection
- host policy overlay: host-owned approval or permission settings layered onto
  managed configuration without becoming Volicord configuration drift when the
  managed identity still matches
- CLI MCP preflight and handshake: terminal-side startup and protocol checks
  for the Volicord MCP server
- managed host startup: lifecycle evidence that a managed host process started
  the Volicord MCP server for the selected connection
- managed host `tools/list`: lifecycle evidence that the managed host process
  reached MCP tool discovery
- managed host tool call: lifecycle evidence that the managed host process
  called a Volicord tool
- active tool exposure: evidence that the active host session can see
  Volicord tools through a current host tool list, tool search, or another
  explicitly reliable source
- storage capability: whether the selected process binding can read registry
  and project state and, for workflow tools, write project state

CLI-side MCP preflight, `volicord mcp --check`, or a direct MCP handshake is a
process startup and protocol diagnostic. It is not, by itself, proof that
Codex, Claude Code, or another external host loaded, trusted, approved,
initialized, or exposed the project configuration.

For Codex project-scoped MCP configuration, the Volicord-managed identity is
the `volicord` server name and this exact portable process descriptor:
`command="volicord"`, arguments
`mcp --stdio --discover-repository --host codex`, and
`env_vars = ["VOLICORD_HOME"]`. The forwarding directive is part of the exact
managed identity but embeds no Runtime Home path. Connection IDs, project IDs,
an absolute command, Runtime Home literal paths, and every other environment
key are invalid in that repository-visible managed entry. The launching host
must provide the clone's init-selected nonempty, absolute `VOLICORD_HOME`;
repository discovery rejects an absent, empty, or relative value before
platform-default substitution. Codex
user-scoped configuration remains a local binding and can carry the selected
connection and project IDs plus managed-launch environment markers such as
`VOLICORD_MCP_LAUNCH=managed_host`, `VOLICORD_MCP_HOST=codex`,
`VOLICORD_MCP_CONNECTION_ID=<connection_id>`, and
`VOLICORD_MCP_PROJECT_ID=<project_id>` when a project binding is present.
Codex-owned tool approval subtables under that server entry are host policy
overlay, not Volicord-managed identity. Preserving an accepted
`tools.<tool>.approval_mode` overlay does not prove host trust, active tool
exposure, running-session approval, correctness, test sufficiency, human
review completion, sandboxing, or actor identity. A missing or changed
forwarding directive, any other deviation from the exact project descriptor,
or command, argument, or managed-marker drift in a local binding is
configuration drift.

Rules:

- An Agent Connection is agent-facing and cannot act as the local
  `User Channel`.
- A connection can be enabled, disabled, removed, or changed in mode without
  treating host configuration text as authority.
- Registering a connection does not automatically grant every project in the
  `Volicord Runtime Home`.
- A connection can address only projects explicitly present in its Connection
  Projects records or selected through an owner-defined repository-root
  registration path.
- `connection.mode=workflow` is the default Agent Connection mode. It exposes
  agent workflow operations as well as read/project discovery operations. It
  does not expose user-only User Action resolution.
- `connection.mode=read_only` exposes read/project discovery operations. It is
  not a workflow-write capability.
- `connection_internal_id`, a `connection_id` process binding, connection mode,
  connection intent, host configuration, or MCP server instructions are not OS
  permissions, host trust, secret isolation, filesystem ACLs, network policy, or
  user authority.

Storage record families and DDL belong to [Storage Records](storage-records.md)
and [Storage DDL](storage-ddl.md). Administrative creation, update,
verification, mode, and removal commands belong to
[Administrative CLI](admin-cli.md).

## Connection Intents

Connection intent describes where the host configuration is meant to be used. It
is not a security level and not an authority grant.

| Intent | Meaning | Must not infer |
|---|---|---|
| `personal` | User-owned host configuration for the current user's ordinary local flow. | It does not prove host trust, user identity, or access to every local project. |
| `shared` | Project-owned or project-shared primary host configuration stored as an explicit integration file in a selected `Product Repository`. | It is not Volicord runtime state, and it does not authorize arbitrary product-file edits. |
| `global` | User-wide host configuration for a supported host, with project access still constrained by repository-root registration and Connection Projects. | It does not connect every repository and does not bypass project or host trust. |

For `volicord init`, `personal` is the default and `--shared` explicitly
selects `shared`; init does not create a `global` connection. Connection
intent classifies the primary managed host target. Repository-local guidance,
local policy, and profile-dependent hook integration files applied by init
remain a separate administrative integration surface and do not change the
stored connection intent or host scope. In particular,
`.volicord/policy.json` is an intent-independent `local_overlay`, and generated
hook wrappers remain local even for `shared`.

For one Product Repository, `volicord init` keeps only one selected supported
host and one active repository-local `personal` or `shared` integration.
Selecting a different supported host or the opposite intent migrates the
managed host and hook projections and retires the prior Connection Project from
active use; it does not silently activate multiple host integrations or intents
against the singleton local policy.

For a host or intent migration that selects a different connection, the
requested project membership remains inactive while external host and guard
projections are applied. A newly registered requested connection is disabled;
an already-enabled requested connection can continue serving its other
projects but does not gain this project membership yet. A prior connection that
was eligible when migration began remains eligible until those projection
steps succeed; an explicitly disabled prior remains disabled. One Registry
transaction then adds the requested project membership, retires the
superseded project membership when that connection has other projects, enables
the requested connection, and records the requested guard installation. For a
superseded connection's last project, the transaction disables that connection
but retains its project membership as durable pending host-cleanup inventory. A
cleanup path revalidates that disabled marker in a short transaction, releases
the Registry write lock for host retirement, and then uses a final transaction
to revalidate the marker and remove the membership. Generic Agent Connection
and Connection Projects mutation APIs cannot forge or mutate this Store-owned
marker. If an external projection step fails before the switch,
the requested project membership remains inactive; a later cleanup failure
leaves the requested connection eligible and the superseded connection disabled
and discoverable for retry. Init reports either case as a partial-application
migration result with a stable migration ID and rerun arguments. The eligibility
switch is atomic; host-file retirement is not rolled back with cleanup storage,
and the surrounding multi-file and external-host migration is convergent rather
than one filesystem transaction.

A new host or intent migration includes older valid pending-cleanup markers for
the same project and rebinds them to its requested replacement before cleanup;
it does not strand an earlier failed migration. The connection named by the
validated prior local policy remains part of the superseded inventory even if
an operator disabled it, while unrelated disabled alternatives are preserved.

The durable marker is the exact `metadata_json.pending_host_cleanup` object on
the disabled superseded connection, with `project_id` and
`replacement_connection_id`. A disabled connection that retains membership but
does not carry a valid marker for that project is an ordinary disabled
connection and must not enter cleanup-resume or Doctor pending-cleanup
handling. Doctor reports an older valid replacement marker so a chained or
interrupted migration remains visible; init rebinds it before cleanup.

A `shared` primary host file contains a typed repository-discovery descriptor:
`volicord mcp --stdio --discover-repository --host codex` for Codex, or the
same command with `--host claude-code` for Claude Code. It also contains exactly
one host-native Runtime Home forwarding form: Codex uses
`env_vars = ["VOLICORD_HOME"]`, and Claude Code uses
`"env": {"VOLICORD_HOME": "${VOLICORD_HOME}"}`. It must not contain
`connection_id`, `project_id`, an absolute executable, a literal Runtime Home
path, or another environment entry. The descriptor bytes can therefore be
reused by another clone, but Agent Connection and project identities remain
local to each Runtime Home.

At repository-discovery startup, the MCP adapter requires forwarded
`VOLICORD_HOME` to be present, nonempty, and absolute and rejects any other
shape before platform-default Runtime Home selection. It then finds the
canonical Git worktree root from the host process current directory, looks up
that exact registered repository root, and requires exactly one enabled
`shared`, project-scoped Agent Connection for the descriptor host whose
Connection Projects include that project. It then narrows the session to that
one project.
An unregistered clone, no matching connection, or more than one matching
connection fails closed with an actionable init, verify, list, or duplicate
removal instruction. Repository metadata never supplies or derives an internal
ID.

Local policy and host overlays may retain connection/project IDs, absolute
commands, Runtime Home selection, and allowlisted local environment values;
they must not be treated as shareable MCP descriptors. A previously generated
shared entry with explicit local bindings is recognized only when its stored
managed fingerprint authorizes a safe migration. Re-running init replaces it
once with the portable descriptor, preserves unrelated host content, and is a
no-op after convergence. New shared projections never emit the legacy binding
shape.

The baseline directly managed host kinds are `codex` and `claude_code`.
Host-neutral MCP configuration is user-managed. User-managed configuration can
use internal registry state needed to start `volicord mcp --stdio` only after a
supported Agent Connection exists, but it is not a normal connection intent for
direct host installation.

## Connection Projects

Connection Projects are the explicit registry relationship between an Agent
Connection and registered projects. User-facing commands select projects by
repository root or project name; registry storage keeps `project_internal_id`
values for referential integrity and provenance.

Membership fields:

- `connection_internal_id`
- `project_internal_id`
- creation timestamp
- a composite primary key over `connection_internal_id` and
  `project_internal_id`

Rules:

- Project membership does not bypass project status, path separation, storage
  executability, Agent Connection mode, or method-owned invocation requirements.
- Invalid current project registrations must be rejected by Connection Projects
  listing and access resolution instead of returned as connected project
  records.
- Inactive or otherwise execution-ineligible valid projects remain unavailable
  at execution time even if membership exists.
- Removing a Connection Project or disabling the Agent Connection must take
  effect without requiring host configuration to be rewritten.
- An Agent Connection with no connected projects may remain stored, and host
  configuration may also remain on disk. That stored state does not mean a new
  `volicord mcp --stdio` process can start successfully.
- New MCP stdio startup and startup checks fail when the Agent Connection has
  zero connected projects.
- A `volicord mcp --stdio` process that already started while at least one project was
  connected can observe later membership changes without host configuration
  being rewritten. After the last membership is removed, project discovery may
  report no available projects, and project-routed public tools cannot proceed
  normally.
- The Agent Connection is executable again only after a project is connected and
  the startup or per-call project checks can validate the required project
  state.

## Host Configuration Inventory

A stored Agent Connection is management inventory for Volicord-managed host
configuration and verification state. The host configuration file remains the
operational source of truth for the external host. The registry record is
management inventory and last-known verification state, not a substitute for
the host configuration.

Rules:

- The registry stores `host_kind`, `connection_intent`, internal server name,
  configuration target, mode, enabled state, managed fingerprint, and last
  verification status.
- Host trust, project trust, project MCP approval, OAuth, or any comparable
  host-controlled approval cannot be bypassed by Volicord.
- A host configuration write can be successful as a file operation while the
  result state remains `action_required` because the host has not yet trusted,
  approved, loaded, initialized, or exposed the server.
- For Codex project-scoped configuration, project trust, host runtime
  observation, active-session Volicord tool exposure, and host MCP command
  launchability remain separate diagnostics. A Codex project can be `trusted`
  while Volicord still has not observed the Codex host process start the MCP
  server, and a PATH-resolved command such as `volicord` must be launchable in
  the environment that starts the MCP server.
- Codex can know the MCP server entry or log startup completion while the
  active session still has no cached tool snapshot or listed `volicord.*`
  tools. CLI-side MCP preflight, direct handshake, managed startup observation,
  manual or elevated probes, and managed `tools/list` observation do not replace
  managed tool-call evidence or another explicitly reliable
  active-tool-exposure source.
- Claude Code managed verification can inspect a project `.mcp.json` entry and
  `claude mcp get` output for matching command, args, environment, and scope,
  and can report connected, pending approval, rejected, missing, changed,
  unavailable, or unknown host state. Current Claude Code verification does not
  by itself prove active Claude Code session tool exposure, managed lifecycle
  startup, managed `tools/list`, managed tool-call evidence, or storage
  capability in a running Claude Code session.
- A host process may need a full restart, reload, resume, or new session after
  MCP configuration changes. The terminal that launched the host can have a
  different PATH or configuration snapshot than a terminal opened later inside
  the host.
- Human text status and verification output is a diagnostic summary for
  interactive users. For `volicord connection status` and
  `volicord connection verify`, read `Status`, `Checks`, `Next`, and
  `Diagnostics` first. Automation and full diagnostic inspection use the
  `--json` output owned by [Administrative CLI](admin-cli.md#setup-output).
- `last_verification_status=complete` may be stored only for an administrative
  verification result that satisfied the operational gates owned by
  [Administrative CLI](admin-cli.md#agent-connection-result-states). A direct
  Volicord-spawned MCP handshake is not enough by itself.
- `last_verification_status=action_required` is the expected state when Volicord can
  manage supported host configuration but a host-owned trust, approval, OAuth,
  reload, restart, command-link repair, or installation-profile repair remains.
- Rejected, missing, changed, unavailable, and unknown host states are not
  `complete` Agent Connection states.
- Product Repository guidance, including Volicord-managed `AGENTS.md` blocks,
  generated host instructions, host rule files, and MCP server instructions can
  improve tool selection, but they are not enforcement mechanisms and cannot
  guarantee that a model will choose Volicord tools.

<a id="host-feature-support-state"></a>
## Host feature support state

`HostFeatureSupportStatus` is the canonical support state for these six exact
managed-host features:

```text
native_user_action
local_web_user_channel
verified_tool_producer
registered_connection_observation
record_final_output
detective_final_output
```

Its values are exactly `verified`, `implemented_unverified`,
`unsupported_by_host`, and `temporarily_unavailable`. Configuration and file
checks are orthogonal facts: `configured=true` or
`configuration_verified=true` never promotes a feature to `verified`.

The centralized evaluator applies this order to a feature and all of its
required subcapabilities:

1. If the exact host, host version, platform, or host-owned surface does not
   provide any required capability, the result is `unsupported_by_host`.
2. Otherwise, if an implementation exists but exact, current, final-binary
   live evidence is absent, stale, expired, malformed, or mismatched, the
   result is `implemented_unverified`.
3. Otherwise, if the evidence matches but a current runtime prerequisite such
   as configuration, connection binding, host approval, listener readiness, or
   event delivery is down, the result is `temporarily_unavailable`.
4. Only when every required capability has matching fresh evidence and every
   current runtime prerequisite is ready is the result `verified`.

Aggregation uses the same precedence: any required
`unsupported_by_host` result wins; otherwise any
`implemented_unverified` result wins; otherwise any unavailable current
runtime prerequisite yields `temporarily_unavailable`; only an all-exact and
ready set yields `verified`. A default with no evidence is therefore
`implemented_unverified` for an implemented built-in feature and
`unsupported_by_host` for a generic or absent host feature. An exact replay
re-evaluates the current evidence, freshness, host identity, final Volicord
artifact, and runtime prerequisites; it cannot inherit an earlier
`verified` result.

The current baseline is:

| Host | Feature state |
|---|---|
| Codex | `native_user_action`, `local_web_user_channel`, `verified_tool_producer`, and `registered_connection_observation` are `implemented_unverified`. `record_final_output` is `unsupported_by_host` because the authenticated actual-host exact-replay entry point is absent. `detective_final_output` is `unsupported_by_host` because a safe block-only finalization surface is absent. |
| Claude Code | All six features are `implemented_unverified` pending exact live evidence bound to the final Volicord artifact and installed host version. |
| Generic | All six features are `unsupported_by_host`. |

`volicord connection status`, `volicord doctor`, and the release feature matrix
consume this one evaluator. They must not independently reinterpret
configuration findings, fixtures, direct-wrapper results, ignored tests, or
historical live results as feature support.

<a id="managed-final-output-authority-disclosure"></a>
## Managed final-output authority disclosure

The final-output-only authority-disclosure display may operate best-effort when
its `authority_display` implementation and configuration are available, but a
support or release claim requires the profile feature to have
`support_status=verified`. Record support requires `authority_display` and
`authenticated_exact_replay`. Detective support requires
`authority_display`, `authenticated_exact_replay`, and `block_finalization`;
replay remains required because every owner-defined delivery, including
Detective replay, refreshes current authority. An unsupported replay or block
surface keeps the aggregate unsupported even if the display runs. Only
profile-applicable subcapabilities are emitted, and best-effort output never
promotes their typed state. The display belongs to the host adapter, not to
model-authored final prose, and uses a host-owned fixed UI surface separate
from MCP tool context.

Before refreshing status, the adapter must read-only verify the enabled Agent
Connection, its selected-project membership, the pinned Product Repository,
host kind, and installed profile. Event text, model text, copied
`connection_id` values, and generated configuration cannot supply or repair that
binding. An eligible binding permits the current read-only status lookup; it
does not grant user authority or create a new authority record.
The adapter derives the controlled internal verification basis
`registered_host_stop_hook_connection_binding` only after those checks. That
value is not a public request field or a User Channel verification basis.
An unverified or directly invoked Detective Stop event may use the internal
`unregistered_host_hook_event` provenance only for a defensive read-only Stop
assessment. That provenance is not a managed binding, is never eligible for
the fixed-UI receipt projection, and cannot replace any check above.

Profile boundaries:

- `record` installs only the managed final-output handler needed for this
  disclosure. It does not install the other Detective lifecycle handlers, run a
  session watcher, activate Detective state, record a guard event, or gate the
  final output. The Codex handler uses Git work-tree root resolution; in a
  non-Git Product Repository, Codex `record` remains available but does not
  install or claim this managed disclosure capability and instead reports the
  applicable `volicord status` fallback. Claude Code does not have this Git-root
  prerequisite.
- `detective` uses the same disclosure projection in addition to its separate
  Stop decision and observation path. The persisted historical Stop decision is
  not the source of a displayed receipt.

Every delivery, including an exact replay, performs a new read-only status
refresh and uses the complete-receipt-or-fallback projection owned by
[Projection and template display boundaries](projection-and-templates.md#managed-final-output-authority-disclosure).
`generic`, user-managed, unsupported, missing, inactive, or degraded adapters do
not claim this managed display capability. Their diagnostic output must expose
the limitation and route an identified Task to
`volicord status --task TASK_ID --json`, or a no-active-Task case to
`volicord status --json`.

Writing or verifying generated adapter configuration proves only the managed
configuration state. It does not prove that the external host loaded the
adapter, delivered a final-output event, or showed the fixed UI disclosure.

<a id="current-connection-context"></a>
## Current Connection Context

Current connection context is the local invocation context derived for one MCP
tool call. It is derived by the local adapter from the bound Agent Connection,
the selected project, the method being called, and adapter-owned invocation
facts. It is not a public request payload.

An MCP session is bound at adapter startup to exactly one `connection_id`
process-binding value that names the stored `connection_internal_id`. Project
selection is resolved from the Agent Connection's registered repository roots
and host-provided project context where available. Public MCP tool input schemas
must not expose internal request envelopes, protocol metadata, `connection_id`,
`project_id`, `actor_source`, `operation_category`, or verification-basis fields
as caller-owned inputs.

Project selection for public MCP method calls is deterministic:

1. Use the project already bound by the selected Agent Connection when exactly
   one available project is eligible.
2. When the connection can see a host-provided repository root, match that root
   to one connected registered project.
3. Otherwise reject the call as ambiguous or unavailable with actionable text
   that names the repository-root setup or connection command needed to repair
   the state.

When explicit selection is needed, the MCP-visible selector is the
`project_selector` value returned by `volicord.list_projects`, not a caller-owned
Core envelope field.

The adapter must not guess a project from folder names, arbitrary process
current working directory values, host labels, or the first row returned by
storage. Host roots may be used only as host-provided repository-root evidence;
they do not bypass registration, Connection Projects, or path-separation
checks.

Before a public tool call enters Core, the MCP adapter must verify:

- the Agent Connection exists and is enabled
- the selected project is explicitly connected to that Agent Connection
- the selected project is active and executable
- the connection mode allows the method's `operation_category`

Connection modes and operation categories:

| Agent Connection mode | Allowed operation categories through MCP | MCP-visible public method tools |
|---|---|---|
| `workflow` | `read`, `agent_workflow` | `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.get_operation_result`, `volicord.prepare_write`, `volicord.prepare_evidence_capture`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_action`, `volicord.reconcile_changes`, `volicord.check_close`, `volicord.close_task` |
| `read_only` | `read` | `volicord.status`, `volicord.get_operation_result`, `volicord.check_close` |

The adapter-owned `volicord.list_projects` utility is visible in both
`workflow` and `read_only` modes. `volicord.check_close` is the read-only MCP
close-readiness tool mapped to the first-class Core read method.
`volicord.close_task` is the workflow-only MCP mutation tool and must not
appear in `read_only` tool discovery.

`volicord.prepare_evidence_capture` is likewise workflow-only and creates only
an intent. Receipt fulfillment is deliberately absent from MCP: the registered
local command runner, guard-event correlator, or session-watcher source must
fulfill the intent through the administrative source path, after which
`volicord.record_run` can finalize the producer. Connection registration and
source correlation remain cooperative local integration; they are not host or
local-principal attestation, actor-identity proof, or anti-forgery protection
against the same local principal.

The table above is the mode-based allowlist. Actual MCP `tools/list` output is
also constrained by the selected projects' readable and writable storage
capability; [MCP Transport](mcp-transport.md#tool-discovery-and-toolscall-response-wrapping)
owns the transport-level discovery and read-only-storage degradation rules.

<a id="operation-result-retrieval"></a>

`volicord.get_operation_result` is a read-only MCP tool in both connection
modes when the selected allowed project is readable. It pages the immutable
historical Core mutation response named by an `OperationResultRef`; it does not
replay the mutation or refresh current authority. The adapter and Core recheck
the enabled connection, Connection Projects membership, selected project, and
stored `actor_source` on every page. References and cursors are non-bearer
locators and never broaden connection access. Agent Connections cannot use the
tool to retrieve `user_only` results, including an exact
`volicord.resolve_user_action` response or private user text. Callers must
read `volicord.status` separately before treating historical facts as current.
The exact method and response contract is owned by
[`volicord.get_operation_result`](api/method-get-operation-result.md#volicordget_operation_result).

A read-only connectivity check combines administrative verification with
active MCP read calls: `volicord connection verify` from the terminal, then
`volicord.list_projects` and `volicord.status` from the active host session.
That path verifies configuration, project discovery, active read-tool exposure,
and readable project state. It must not require creating a `Task`.

A workflow write-path smoke check uses Agent Connection workflow tools and can
create Volicord state. A minimal path can include `volicord.intake`,
`volicord.update_scope`, `volicord.record_run`,
`volicord.request_user_action` when final acceptance is required for close,
and `volicord.check_close`. The resulting task can remain blocked by
`missing_final_acceptance` until the user records the required final judgment
through a supported `User Channel`.

The opt-in live Judgment harness in
[Testing Strategy](../architecture-guide/testing-strategy.md) exercises a
smaller connection round trip with an installed host: marker Task creation,
product-decision Judgment creation, a human answer through the host-native MCP
User Channel, consumption of the selected option from the default compact
result, a choice-mapped no-write Run, and the resulting Task-state refresh. It
requires the stored resolution basis `mcp_elicitation_user_channel`, the stored
`selected_option_id`, and the latest Run marker to agree. A pending CLI inbox
fallback is actionable recovery but is not counted as a successful native round
trip. The harness is ignored by default and is not a portable host-conformance
or security test.

`volicord.resolve_user_action` has `operation_category=user_only`. It is the
public Core API method for every User Channel resolution, including judgments
and target-bound evidence observation, but it is not exposed by Agent
Connections. The supported local fallback is the common `volicord inbox`
command group owned by [Administrative CLI](admin-cli.md#user-channel-commands).
An Agent Connection cannot substitute an ordinary `record_run` claim, staged
artifact, tool metadata, raw guard payload, or relayed text for a resolution.

Internal actor shape, not a public API schema:

```yaml
InvocationContext:
  actor_source: local_user | system | agent_connection:<connection_id>
  operation_category: read | agent_workflow | user_only | admin_local | local_recovery
  verification_basis: string
  assurance_level: string
```

Baseline `assurance_level` means cooperative local provenance, not
cryptographic human identity. Authority-bearing user-action resolution
requires `actor_source=local_user`, `operation_category=user_only`, compatible
User Channel provenance, and method-owned compatibility. An Agent Connection
cannot gain user authority by submitting copied user text or generated guidance.

Conditions:

- A public API request has exactly one derived `InvocationContext`.
- Internal project selection is constrained by the Agent Connection's connected
  projects. It is not caller authority and cannot grant access to an unlisted,
  inactive, or invalid project.
- MCP-visible public tool schemas do not expose `actor_source`,
  `operation_category`, `connection_id`, `project_id`, request metadata, or
  protocol envelope fields. If raw MCP arguments include those fields, the
  adapter rejects the call before Core execution.
- Nested payloads such as `ArtifactInput` or `StagedArtifactHandle` do not add
  a second invocation context.
- Authority-provenance fields for resolved authority-bearing user actions come
  from the derived `InvocationContext`, not caller text, labels, answer
  payloads, copied refs, generated Markdown, or Product Repository guidance.
- Protected reads, mutations, and artifact operations can rely on an invocation
  only when the method owner accepts the derived context.

Agent may:

- preserve derived invocation context when displaying or passing owner-result
  context
- expose absent or incompatible context as unavailable, mismatched, stale, or
  insufficient Agent Connection state

Agent must not:

- submit `InvocationContext` as a request payload
- assert `verified=true`
- submit `actor_source=local_user` or `operation_category=user_only` from an
  Agent Connection to satisfy user authority
- submit arbitrary verification-basis text as public request authority
- fabricate staged artifact provenance
- use copied identifiers, generated Markdown, chat text, projection text, or
  agent memory as substitutes for current connection context

Owner links:

- Exact request envelopes and response shapes belong to
  [API Schema Core](api/schema-core.md), [API Methods](api/methods.md), and
  method owners.
- `operation_category` value names belong to
  [API Value Sets](api/schema-value-sets.md).
- `volicord mcp --stdio` startup, connection binding, environment variables, stdio
  framing, startup validation, response wrapping, and shutdown belong to
  [MCP Transport](mcp-transport.md).

## User Channel And Agent Connections

Agent Connections are agent-facing connections. They are not the
`User Channel`, even when the model is relaying a user's words.

Conditions:

- The supported local CLI path for a human user to inspect pending user actions
  and submit the stored action-specific form is the `volicord inbox` command group
  owned by [Administrative CLI](admin-cli.md#user-channel-commands).
- When the initialized MCP client declares `capabilities.elicitation`,
  `volicord mcp --stdio` may use server-initiated elicitation as a User Channel
  path for a pending request created by `volicord.request_user_action`; the
  wire behavior is owned by [MCP Transport](mcp-transport.md#user-action-elicitation).
- A User Channel credential, bearer token, credential-bearing URL, complete
  request body, or `UserActionInboxForm` must never cross Agent Connection MCP
  `content`, `structuredContent`, compatibility or diagnostic text, full-detail
  output, resume replay, or operation-result bytes. An Agent Connection receives
  only the pending request ID, `status=pending`, and `next_actor=user` for the
  request itself.
- The exact boolean `true` at
  `params.capabilities.experimental["io.volicord/user-channel"].model_invisible_user_surface`
  is only the initialized client's cooperative delivery declaration. It does
  not grant local-web eligibility by itself.
- Local web is available only when one evaluator confirms all of the following:
  the current transport is the managed stdio host path; a loopback listener is
  ready; the exact declaration is `true`; and one
  `host_capability_verifications` row with `outcome=passed` is current at
  `observed_at <= now < expires_at` and exactly matches the Agent Connection,
  non-generic host kind, `clientInfo.name`, `clientInfo.version`, adapter
  profile and version, Volicord build, exact lowercase 40- or 64-hex source
  revision, target and executable digest, managed fingerprint, and live-host
  evidence digest. The evaluator must obtain the expected
  `evidence_artifact_sha256` from a separately verified exact-final-artifact
  release evidence manifest or receipt outside the executable. That manifest
  must bind the same capability, host/client, adapter, build, source, target,
  and executable digest, and the row's digest must exactly match its expected
  value. Missing, unknown, malformed, unverified, or mismatched manifest input
  is unavailable. The row's own digest and the build descriptor cannot supply
  the expected value. The row must also have
  `host_version == client_version == clientInfo.version`, with that same
  version bound as the artifact's installed-host version. Missing, false,
  wrong-typed, wrong-namespace, malformed, expired, revoked, ambiguous, corrupt, or
  mismatched input is unavailable and must not issue a token.
- The current adapter has no trusted acquisition path for that external
  manifest or receipt. Production local-web eligibility therefore remains
  fail-closed and uses CLI inbox; test-only injection of an expected value does
  not establish production availability.
- Manual stdio, CLI verification probes, Local HTTP transport, generic host
  connections, and unknown or invalid managed-launch markers are never
  eligible for this handoff. They use CLI inbox recovery even when the client
  declares the exact capability and a listener exists.
- A matching row records bounded evidence that the named host/profile delivered
  this handoff during a specific validation run. It is not host attestation,
  proof of host isolation, proof of current user identity, or a guarantee that
  a later external host preserves model invisibility. The host must still keep
  the namespaced tool-result `_meta` handoff outside model context and render it
  on a user-owned surface.
- The local-web URL may appear only in that model-invisible `_meta` handoff.
  It must not be copied into fallback text for the agent to relay. The consent
  page identifies the pending request, stored candidates, and non-guarantees;
  possession of its bearer credential is what opens that user-only page.
- When native elicitation and a negotiated model-invisible local-web surface
  are unavailable, agent-visible fallback identifies the pending request and
  routes the human user to the `volicord inbox` CLI path. Prompt-submit capture
  remains a separately verified User Channel integration; fallback text must
  not expose its complete form or a resolution credential.
- Public Agent status and close results use only the exact three-field pending
  summaries; they return no User Channel availability or capture-path facts. A
  complete `UserActionInboxItem` and its credential-free availability categories
  are fetched only through the separate internal Core projection used by a
  verified User Channel renderer. Unavailable host prompt input must not hide
  another available answer path, and the CLI inbox remains available when
  applicable. These projections do not let an Agent Connection resolve the
  action.
- A rich path is available for a particular action only on a User Channel
  surface and when its complete,
  untruncated request-bound presentation fits the transport or host-render
  budget owned by MCP Transport or Administrative CLI. A presentation-budget
  failure makes that path unavailable and must continue to the next compatible
  User Channel path.
- Every user-action resolution requires `actor_source=local_user`,
  `operation_category=user_only`, and compatible User Channel provenance.
- `actor_source=agent_connection:<connection_id>` cannot become `local_user`
  provenance by relaying text from a user.

Agent may:

- request a missing user action when a method owner supports that path
- display only the agent-safe pending request summary and current safe
  resolution projection
- route the human user to the supported `User Channel`

Agent must not:

- resolve any user action from an Agent Connection
- obtain, relay, open, or submit a User Channel bearer credential or complete
  user-only capture form from Agent Connection output
- treat Agent Connection tool arguments as MCP elicitation responses
- treat a natural-language approval, chat reply, generated Markdown status, or
  rendered projection as User Channel provenance
- broaden one selected option into final acceptance, residual-risk acceptance,
  sensitive-action approval, scope acceptance, or another judgment kind
- create evidence sufficiency, acceptance, residual-risk acceptance, close
  readiness, or security authority from displayed judgment text

Owner links:

- [Core Model](core-model.md) owns the authority meaning of user-owned
  judgments, final acceptance, residual-risk acceptance, evidence, and close
  readiness.
- [Resolve-user-action method](api/method-resolve-user-action.md) owns public
  method behavior for resolving one pending user action.
- [Projection and template display boundaries](projection-and-templates.md)
  owns generated display and projection authority boundaries.

## Agent Behavior Guidance

Agent behavior guidance has two layers:

- MCP server instructions are always supplied by the server during MCP
  initialization.
- Optional `Product Repository` guidance is installed only with explicit user
  authorization when an administrative command supports it.

Rules:

- MCP server instructions may describe cross-tool workflows, project selection
  rules, and limitations that apply across Volicord tools.
- Optional repository guidance may add a Volicord-managed `AGENTS.md` block or
  host-specific rule file inside a `Product Repository` only under the boundary
  owned by
  [Runtime Boundaries](runtime-boundaries.md#explicit-integration-files-in-product-repositories).
- Guidance can improve tool selection, but it is not authority, access control,
  user judgment, security enforcement, or proof that a model will choose
  Volicord tools.

## Agent Context Transfer

Agent context transfer gives the agent enough owner context for the next action
without turning the packet into an authority record.

Conditions:

- Agent context should contain only owner results needed for the next action and
  current connection-context limits that affect that action.
- A context packet is support context, not Core state, storage state, evidence,
  acceptance, residual-risk acceptance, or close output.

Agent may:

- pass compact context containing the current Task summary, current scope,
  `state_version`, pending user-owned actions, blockers, next safe action,
  evidence and artifact summaries, close-readiness and residual-risk summaries,
  owner-supported guarantee display, and source or limitation notes
- retrieve exact owner sections only when the next action needs them
- include both language versions for the same `doc_id` when bilingual
  maintenance requires semantic-parity review

Agent must not:

- inject full schemas, DDL, historical logs, artifact bodies, unrelated contract
  material, out-of-scope catalogs, exact template bodies, or both language
  versions for the same `doc_id` by default
- treat a stale or copied context packet as newer authority than the owner
  result or underlying record

Owner links:

- [Template Bodies](template-bodies.md) owns agent context packet wording.
- [Reference Index](README.md) routes exact owner sections.
- [Translation Policy](../maintain/translation-policy.md) owns bilingual
  semantic-parity review guidance.

## Fallback Boundary

Fallback display applies when current connection context or a required
connection mode is unavailable, mismatched, stale, or insufficient for the
requested operation.

Agent may:

- move to a suitable connection mode or a different connected project
- narrow the operation
- request the missing user-owned judgment
- continue outside Volicord only when the user explicitly chooses that mode

Agent must:

- expose the limitation in support or display text
- route machine-readable failure meanings to
  [API error codes](api/error-codes.md) and
  [API error details](api/error-details.md)
- route user-facing wording to [Template Bodies](template-bodies.md)

Agent must not:

- fabricate authority
- hide unavailable, mismatched, stale, or insufficient context states inside
  ordinary success text
- continue outside Volicord without the user's explicit choice
