# Agent Connection Reference

This document defines the first-release Agent Connection contract. It owns the
exact `host_kind=codex` Record connection surface, the canonical connection
verification report, managed configuration ownership, integration revisions,
and the validated operational-session boundary between the Codex adapter and
Core.

<a id="owns-and-does-not-own"></a>

## Owns / Does Not Own

This document owns:

- the accepted `host_kind`, integration profile, connection intents, transport,
  user-action delivery path, modes, and platform-environment values;
- the canonical `ConnectionVerificationReport`, its closed status values,
  deterministic aggregation, strict encoding, and missing-report projection;
- Connection and project integration revisions;
- authoritative managed-host runtime, negotiated MCP profile, and project
  session ownership;
- `ValidatedAgentSession` and the checks required before Core consumes it; and
- Codex adapter discovery, installation, verification, repair, and uninstall
  responsibilities.

This document does not own:

- stdio framing, MCP initialization, tool routing, or shutdown; see
  [MCP Transport](mcp-transport.md);
- administrative command syntax, output, or exit codes; see
  [Administrative CLI](admin-cli.md);
- exact database tables or storage effects; see
  [Storage Records](storage-records.md) and
  [Storage Effects](storage-effects.md);
- ordinary build, package, platform, and release validation; see
  [Validation](../maintain/validation.md);
- operating-system topology and filesystem prerequisites; see
  [System Requirements](system-requirements.md);
- Core `UserActionRequest` and `UserActionResolution` schemas; see
  [API User Action Schemas](api/schema-user-action.md); or
- product-wide failure-category and security meanings; see
  [Failure Model](failure-model.md) and [Security](security.md).

<a id="surface-stability"></a>

## Surface Stability

Labels follow the canonical vocabulary in
[Documentation Policy](../maintain/documentation-policy.md#surface-stability-labels).

| Surface | Stability | Contract |
|---|---|---|
| First-release value sets, `ConnectionVerificationReport`, integration revisions, authoritative operational sessions, and `ValidatedAgentSession` | `stable` | These are exact boundary contracts. |
| Codex discovery, managed installation, verification, repair, uninstall, and drift result semantics | `stable` | Implementations may change without changing the observable contract. |
| Adapter modules, filesystem helpers, the hidden launcher, and Store lease/query helpers | `internal` | They preserve the stable boundary but are not public surfaces. |
| Human-readable verification guidance and client/host version observations | `diagnostic` | Machine-readable categories, reasons, and typed fields remain authoritative. |

<a id="first-release-surface"></a>

## First-Release Surface

The first release accepts only this Agent Connection surface:

| Dimension | Exact value |
|---|---|
| Host | `host_kind=codex` |
| Integration profile | `integration_profile=record` |
| Connection intent | `personal` or `shared` |
| Connection mode | `read_only` or `workflow` |
| Transport | Volicord-managed stdio MCP entered through the hidden host launcher; public manual stdio remains `volicord mcp serve` |
| Production MCP revisions | `2024-10-07`, `2024-11-05`, `2025-03-26`, `2025-06-18`, or `2025-11-25` |
| User-owned action delivery | CLI inbox |
| Platform environment | `linux`, `macos`, `native_windows`, or `wsl2` |

A `personal` connection installs user-owned local Codex configuration. A
`shared` connection installs project-owned Codex configuration inside the
selected `Product Repository`. A personal managed launch identifies one
registered Connection; the Connection's allowed projects remain its
authoritative Store-owned memberships. A shared managed launch resolves its
Connection and project through repository discovery.

The Connection owns its host kind, scope, mode, managed-configuration
fingerprint, project membership, immutable Store-owned integration-instance ID,
and Store-owned integration generation. Connection and project integration
revisions derived from those current owner facts are local lifecycle and
correlation coordinates.

A genuinely new Agent Connection defaults to `workflow`; a new Connection
created by `volicord connection add --read-only` instead starts in
`read_only`. Setup replay and repair use the persisted mode of a matching
current Agent Connection exactly. In particular, omitting `--read-only` from
add is not a `workflow` transition request, and supplying it for an established
`workflow` Connection does not authorize an implicit transition. Setup never
infers mode from the Record profile. `volicord connection mode` is the only
command that changes an established mode and increments the integration
generation.

An Agent Connection is a stored local integration record in the
`Volicord Runtime Home`. It does not grant operating-system permission,
establish user identity, or prove that Codex loaded the managed entry. One
managed stdio MCP process is bound to one current Agent Connection.

User-owned actions are delivered through the CLI inbox. An MCP agent may
request an owner-defined action, but it cannot act as the local user channel or
resolve the action on the user's behalf.

For the production revision set, an exact requested `protocolVersion` selects
the same session profile. Another string in the initialization-based request
shape receives the server's preferred `2025-11-25` counter-offer; support is
never inferred from date ordering or configured by the user. The pinned
pre-release `2026-07-28` revision belongs to the discover-based generation and
does not enter initialize negotiation. Exact parameter and response behavior
is owned by [MCP Transport](mcp-transport.md#protocol-revision-negotiation).

<a id="managed-mcp-launch-contract"></a>

## Managed MCP Launch Contract

One typed managed MCP launch contract is the canonical source for the hidden
launcher command and arguments, static and forwarded Runtime Home bindings, the
personal/shared distinction, strict launch-shape validation, canonical
projection, and deterministic managed-fingerprint inputs. Managed provenance
begins only with successful one-time launch-lease consumption.

A personal connection requires the selected canonical absolute Runtime Home
and selected absolute `volicord` executable. It invokes the hidden host-owned
launcher, stores only the Runtime Home as process configuration, and forwards
no parent-environment name:

```toml
[mcp_servers.volicord]
command = "/absolute/path/to/volicord"
args = ["_host-launch", "codex", "--connection", "<connection_id>"]

[mcp_servers.volicord.env]
VOLICORD_HOME = "/absolute/runtime/home"
```

A personal entry carries no project selector. Its arguments contain no
`--project`, and its static environment contains no project marker. The Agent
Connection's authoritative Product Repository associations remain Store-owned
Connection Project memberships rather than process-launch state.

A shared connection contains only the clone-portable repository-discovery
launch. It uses `volicord` from `PATH`, forwards only `VOLICORD_HOME`, and has
no static environment table:

```toml
[mcp_servers.volicord]
command = "volicord"
args = ["_host-launch", "codex", "--discover-repository"]
env_vars = ["VOLICORD_HOME"]
```

The shared launch contains no absolute executable, Runtime Home, Connection ID,
project ID, or other machine-local lifecycle coordinate. A personal entry with
a project argument or project environment marker, a static/forwarded
environment collision, a blank or duplicate forwarded name, an incomplete
personal binding, or a mixed personal/shared argument or environment shape is
invalid.

Generated Codex configuration is an adapter projection of this contract. The
configuration contains no launch lease, nonce, reusable secret, or raw operating-
system handle. CLI verification derives its
public preflight and manual stdio probe commands from the same binding facts;
neither probe is a managed-host launch.

The Codex adapter parses the current TOML shape and validates a managed entry by
reconstructing this same typed contract. Unknown launch keys, malformed values,
and noncanonical shapes are drift rather than a second accepted form. The
adapter preserves only a valid `tools.<known-tool>.approval_mode` overlay and
excludes it from launch identity. The managed fingerprint covers the canonical
launch projection together with host kind, scope, and server name. Formatting
differences do not change it; a launch-semantic difference does.

The hidden launcher strictly reloads the current canonical entry, verifies the
enabled Connection and exact integration revision, and verifies that the
entry's fingerprint equals the current stored managed fingerprint. It then
creates one short-lived Registry launch lease and transitions in memory to the
stdio adapter. The lease ID is not written to Codex configuration, process
arguments, logs, or a public environment variable. MCP bootstrap atomically
consumes the lease and creates the `managed_host` runtime session. Consumption
requires the exact Connection, `codex` host kind, integration revision, and
managed fingerprint captured by the launcher. A lease is single-use; replay,
expiry, mismatch, and cancellation fail closed, and a normal launcher failure
terminalizes any still-unused lease.

<a id="connection-verification-report"></a>

## `ConnectionVerificationReport`

One small report is the canonical serialized connection-verification state:

```yaml
ConnectionVerificationReport:
  status: complete | action_required | failed
  checked_at: UtcTimestamp
  checks: ConnectionCheck[]
  root_cause_ids: DiagnosticFindingId[]
  actions: ConnectionAction[]

ConnectionCheck:
  id: ConnectionCheckKind
  status: passed | pending | failed | blocked | not_applicable
  depends_on: ConnectionCheckKind[]
  cause_finding_ids: DiagnosticFindingId[]
  code?: string
  summary: string
  details?: object
  observed_at?: UtcTimestamp

ConnectionAction:
  id: ConnectionActionKind
  instruction: string
```

`status`, `checked_at`, `checks`, `root_cause_ids`, `actions`, and each check or action's
non-optional members are required. Optional `code`, `details`, `observed_at`,
members are omitted when absent rather than serialized as null. Unknown
members, duplicate JSON keys, duplicate check kinds, duplicate action kinds,
noncanonical ordering, explicit null for an optional member, and unknown status,
check-kind, or action-kind values are invalid. Non-null check codes are 1
through 128 ASCII bytes and match `[a-z][a-z0-9_]*`. `summary` and
`instruction` values are 1 through 4,096 UTF-8 bytes and contain no NUL. A
non-null `details` value is a JSON object whose serialized form is at most 16
KiB. A check contains at most 16 dependency edges and 32 root-finding
references. A report contains at most 64 checks and 32 actions, and its
serialized form is at most 64 KiB.

Checks are sorted by the stable snake-case spelling of `ConnectionCheckKind` in
ascending UTF-8 byte order. Actions use the same ordering by the stable
snake-case spelling of `ConnectionActionKind`. Strict decoding rejects another
order rather than silently normalizing it; enum declaration order is not the
wire-order contract.

`ConnectionCheckKind` is the closed current-product vocabulary:
`connection_removal`, `diagnostic_lookup`, `guard_files`, `guard_hook_execution`,
`guard_observation`, `guard_verification`, `host_executable`, `host_session`, `managed_config`,
`mcp_server`, `mode_transition`, `process_startup`, `project_trust`,
`required_tools`, `runtime_session_lookup`, `setup_plan`, `tool_round_trip`, and
`verification_not_run`.
Operational verification uses the applicable checks in
the table below. Missing-report and administrative command planning use the
remaining named kinds. `diagnostic_lookup` and `runtime_session_lookup` are
used only by their bounded administrative diagnostic operations; arbitrary
adapter-defined check IDs are not accepted.

`ConnectionActionKind` is the closed current-product vocabulary:
`apply_removal`, `apply_setup`, `host_trust_required`,
`inspect_codex_protocol`, `install_or_repair_codex`, `observe_codex`,
`reload_host`, `repair_guard`, `repair_managed_config`, `repair_mcp_server`, and
`run_verification`. Host plans, host effects, verification reports, and command
reports use the canonical `ConnectionAction` contract directly. Within that
direct contract, `observe_codex` and `inspect_codex_protocol` remain distinct
from `reload_host`.

A Connection action expresses semantic work through its stable kind and user
instruction. It contains no executable shell text. JSON consumers use the
action ID and instruction as report facts rather than executing action content.
When complete current selector coordinates are available, the human renderer
constructs executable follow-up guidance from its typed current CLI invocation
context. That renderer-owned guidance is not copied into action JSON or the
persisted report.

Every check in the report is required for that report. The top-level status is
derived and cannot disagree with the checks:

1. any `failed` or `blocked` check produces `status=failed`;
2. otherwise any `pending` check produces `status=action_required`;
3. otherwise the report contains only `passed` and `not_applicable` checks and
   produces `status=complete`.

The five check statuses have exactly these meanings:

- `passed`: the check completed successfully;
- `pending`: its required external observation or user-triggered event has not
  occurred and no failed prerequisite currently prevents it;
- `failed`: the check itself observed a failure;
- `blocked`: the check could not run or be observed because a prerequisite
  check failed; and
- `not_applicable`: the check does not apply to this Connection or profile.

`depends_on` is the canonical explicit dependency edge set for the check kind.
Operational verification uses these chains:

```text
managed_config -> process_startup -> host_session
required_tools -> tool_round_trip
managed_config -> mcp_server
guard_files -> guard_hook_execution -> guard_observation -> guard_verification
```

`host_session` is the managed-host `initialize` check, `required_tools` is the
managed-host `tools/list` check, and `tool_round_trip` is the canonical
verification-role tool-call check. `ToolVerificationRole::ManagedHostRoundTrip`
is bound at compile time to `AgentToolId::LIST_PROJECTS`; its persisted and MCP
wire-name projection is `volicord.list_projects`. The CLI probe, MCP runtime,
Store observation, and verification report comparison use that same typed
identity. `process_startup` and `host_session` use the fixed `latest_attempt`
role: the newest current-revision `managed_host` runtime. `required_tools` and
`tool_round_trip` use the fixed `latest_complete_proof` role: the newest such
runtime that completed initialize, the initialized notification, `tools/list`,
required-tool validation, and the canonical verification-tool call in that
one runtime. No check combines milestones from different sessions. When no
managed-host attempt exists, all four checks are `pending`. A terminal latest
attempt makes current managed-session health fail; an older complete proof can
still pass the capability checks, but it is reported under its distinct role
and cannot hide that current failure. Without a complete proof, capability
readiness remains pending or fails from the latest attempt's own observation.
A managed-configuration failure blocks `mcp_server` and the current-attempt
process/session chain. A Guard-file integrity failure blocks hook execution,
phase observation, and the correlated integration verification.

Only `failed` and `blocked` checks may carry `cause_finding_ids`. A `blocked`
check carries the canonical sorted union of the independent root-finding IDs on
its failed or blocked prerequisites. `root_cause_ids` is the sorted,
deduplicated union for the complete check graph. A blocked check whose causes
do not match its failed prerequisites, a dependency cycle, or noncanonical
dependency edges makes the report invalid.

The current Codex connection report contains these operational checks:

| Check ID | Successful observation | Waiting or applicability rule | Self failure |
|---|---|---|---|
| `managed_config` | The selected target contains the canonical managed entry. | Applies to every managed Connection. | The required entry is missing, malformed, owned by another entry, changed, or unavailable to inspect. |
| `host_executable` | `codex` was discovered on `PATH` and its version command succeeded. | It waits when the read-only status path has no prior active probe. | Discovery or the version command failed. |
| `mcp_server` | The CLI self-test passed preflight and the complete MCP exchange. | It waits for active verification and is blocked by failed managed configuration. | The self-test itself observed a process, Store, or protocol failure. |
| `process_startup` | A current managed host started the configured MCP process. | It waits for managed-host use and is blocked by failed managed configuration. | No managed-host startup failure is claimed without a typed host observation; absence remains waiting. |
| `host_session` | The `latest_attempt` completed `initialize` and its initialized notification. | It waits for the newest current-revision managed-host attempt and is blocked by `process_startup`. | That latest attempt has a linked terminal protocol finding. |
| `required_tools` | The `latest_complete_proof` records one actual `tools/list` inventory and same-session required-tool validation. | It waits when no complete proof exists; it has no dependency on the current-attempt session check. | With no complete proof, the latest attempt returned a missing required set or terminally failed. |
| `tool_round_trip` | The same `latest_complete_proof` records `verification_tool_name=volicord.list_projects` and its observation time after required-tool validation. | It waits for a complete proof and is blocked only by `required_tools`. | With no complete proof, the latest attempt's canonical call failed or used a different current canonical owner. |
| `project_trust` | Project trust is satisfied. | A normal trust or reload action is `pending`; scopes with no separate trust check are `not_applicable`. | Trust configuration is malformed or contradictory. |
| `guard_files` | Every current Guard manifest file expectation matches. | Applies when Guard is part of the Connection profile. | A managed file, manifest, wrapper, ownership, or executable-integrity check failed. |
| `guard_hook_execution` | A current managed Guard hook executed. | It waits for current hook activity and is blocked by `guard_files`. | Hook execution itself recorded a failure. |
| `guard_observation` | Every required current typed hook phase was observed. | It waits for remaining phases and is blocked by `guard_hook_execution`. | A current event reports an incompatible hook contract. |
| `guard_verification` | One bounded run correlated MCP acknowledgement with prompt, pre-tool, and post-tool observation in the same current managed runtime, native session, and turn. | It waits for a completed current run and is blocked by `guard_observation`. | The newest run no longer matches current runtime, Guard Installation, policy, revision, or hook-contract ownership. |

CLI MCP preflight is read-only and creates no runtime session. The manual
stdio self-test creates `session_source=manual_cli` only in a disposable
per-command Runtime Home; neither preflight nor that disposable evidence
satisfies `process_startup`, `host_session`, `required_tools`, or
`tool_round_trip`. Guard uses `guard_files`, `guard_hook_execution`,
`guard_observation`, and `guard_verification` as top-level operational checks.
The strict Guard manifest owns the current policy hash, integration revision,
typed runtime commands, complete Volicord-managed artifact expectations, and
required hook phases. It also names the exact `host_contract_profile` and
deterministic `host_contract_digest`; the current values select
`codex-hooks-v1` and its reviewed contract identity. Policy and runtime
commands are distinct projections of one typed invocation. Audit rejects a
manifest whose profile or digest differs from that exact selection, compares
every managed artifact with its canonical current expectation, and requires
compatible current events for every required phase.

A recorded verification-tool name mismatch never passes `tool_round_trip`.
The check fails with `tool_round_trip_designation_mismatch`, and active
verification persists
`mcp.tool_verification.designation_mismatch`. Its bounded facts expose the exact
`expected_tool_name` and `observed_tool_name`; JSON check details and verbose
output likewise show the exact expected and observed names. A prior revision,
non-managed runtime row, missing milestone, or a pair split across sessions cannot
substitute for the current exact pair.

Any bounded Codex version proceeds through these behavioral checks. The PATH
executable version does not select or disqualify a managed runtime session.

`dry_run` is an operation mode, never a connection or check status.
Configuration matching, executable availability, protocol and host versions,
capability observations, and observation timestamps belong in check facts;
they do not introduce another public or persisted status enum.

User instructions appear only in `actions` inside this report. They are ordered
and deduplicated by stable ID and are derived from root findings and current
check state. A blocked downstream check does not emit an observation action;
its blocker's repair action comes first. Equivalent actions from several
symptoms collapse to one stable action. Reload and first-use actions require actual Codex activity to be
observed. A passing `guard_files` check does not request Guard file
reinstallation. Registry storage does not keep an independent verification
status or action array. A
connection with no completed persisted report is projected as a synthesized
`status=action_required` report containing one `verification_not_run` pending
check and one verification action. Reading that projection does not persist it.

The administrative CLI projects init, add, status, verify, mode, and remove
through the current schema-2 `DiagnosticReport`. It carries the canonical
checks, bounded findings and cause edges resolved from the current evaluation
overlay and Store APIs, derived
root IDs, one deduplicated typed action per root, Connection context,
operation-specific result details, and report limits. Concise, verbose, and
JSON output are projections of this same report and identify the same roots.
No renderer derives a cause or remediation category from summary prose, and no
projection re-exposes a fact redacted by `DiagnosticFinding`.

A current Connection report selects findings from the exact IDs referenced by
its `failed` and `blocked` checks and then resolves only their bounded cause
chains. Resolution uses an inline finding from the current evaluation before
an explicitly persisted Store seed, while retaining explicit provenance for
each reference. The combined graph may contain inline current findings,
persisted immutable occurrences, and persisted active current-state findings.
An independent current finding appears only when the operation
deliberately selects it; the report never treats every stored finding on the
same integration revision as current. CLI-owned current-state operational
findings bind each closed diagnostic value to one immutable definition and use
typed subjects for the exact managed-config target, Product Repository trust,
Guard managed artifact, Guard phase, Guard Installation, Guard event,
integration revision, or verification tool. Each subject owns its scope,
typed versioned canonical identity encoding and opaque subject identity, and a
separate safe display projection. Path-bearing subjects canonicalize filesystem
aliases before deriving the opaque identity and do not persist the canonical
path bytes. Each `CurrentDiagnosticKey` includes the complete Connection scope,
full code, domain, stage, source, and opaque subject identity. Its stable ID is
the full fixed digest of that complete key, so the same diagnostic code on two
artifacts or phases remains two findings while re-observing one subject
refreshes only its snapshot, including its safe display projection.

Active verification reconciles each complete CLI owner observation set. It
activates or refreshes the conditions still observed and explicitly resolves
previously active conditions omitted after a successful repair, compatible
revision, or fresh observation. Resolved current findings remain available by
exact ID but are not reportable current findings and do not reappear through a
failed or blocked check's current projection.

The JSON projection includes `generated_at` as the report time and the exact
current integration revision in Connection context when one exists. The
persisted verification `checked_at` remains the observation time for that
verification and is not repeated as a competing top-level time. Status builds
a complete in-memory current evaluation from stored active-probe facts and
current observations, but that read does not persist the projection or modify
any timestamp. It needs no verification run to make an inline cause
reportable. Only a reference explicitly classified as persisted whose Store
row is absent is represented as the typed
`diagnostics.finding_record_missing` observation; rendering does not fabricate
the missing domain facts. An inline finding is returned as the actual cause
and never receives missing-record substitution or
`action.diagnostics.rebuild_current_observations` guidance.

Optional active verification captures the exact typed Connection integration
revision before it plans or probes. Store persists the resulting report only
when that same revision is still current, using one immediate Registry
transaction for the comparison and report replacement. The write changes only
`verification_report_json` and the ordinary row update timestamp. A revision
conflict leaves the existing report and every owner field unchanged and
requires verification to be rerun. Verification observes managed
configuration; it never applies, adopts, or records a newly planned managed
fingerprint.

A persisted action with any member other than `id` and `instruction` is a
malformed current report and strict reads reject it. Active verification may
replace such a malformed report through the same revision-guarded replacement
boundary; it does not modify unrelated Connection owner state. No in-place JSON
rewrite or alternate decoder applies.

Operational compatibility is determined from the current managed configuration
and the protocol, tool-list, required-tool, safe-call, and Guard behavior the
adapter actually observed. `complete` means every applicable required check
passed and every other check is `not_applicable`. It does not establish
operating-system enforcement, actor or human
identity, correctness, future behavior, or tamper-proof recording. Core
invocation authorization is evaluated separately for each managed MCP call.

## Integration Revisions And Operational Sessions

The current Connection integration revision is a typed, domain-separated
canonical SHA-256 digest. Its basis is the Agent Connection identity, immutable
Store-owned integration-instance ID, host kind, intent, scope, mode, server
name, configuration target, and current exact managed-configuration
fingerprint, plus the nonnegative Store-owned integration generation. That
fingerprint covers the managed server command and entry and identifies the
Volicord-managed host configuration that a setup owner last successfully
applied or adopted. Only setup, repair, staged activation, or another explicit
managed-configuration mutation may replace it. Replacing it changes the
integration revision and clears the prior verification report atomically;
compatible replay with the same fingerprint may retain that report.

Store generates a new opaque integration-instance ID only when it inserts a
new physical `agent_connections` row. Compatible registration replay,
enabled-state and verification updates, staged activation and cleanup recovery,
and mode transitions preserve it. Physical deletion removes the instance with
the row. Recreating the same deterministic Connection identity therefore gets
a new integration-instance ID even when every caller-visible target and
configuration input is unchanged.

The integration generation begins at zero and increments exactly once for each
successful real mode transition within that physical instance; a same-mode
no-op leaves it unchanged. Therefore the generation distinguishes revisions
within one physical instance, while the integration-instance ID distinguishes
deletion and recreation. Returning to a previously used mode still creates a
new revision and cannot make evidence from that earlier mode generation current
again.

The observed executable path, host version, and MCP client name/version remain
diagnostic facts. Authorization uses the current Connection, revisions, and
authoritative runtime/project session bindings.

The actual MCP peer is the bounded `clientInfo.name` and `clientInfo.version`
observed on that runtime session. The PATH probe is the separately observed
Codex executable path and version. Reports and findings never substitute one
for the other. When both versions exist and differ,
`host.codex.peer_version_differs_from_path_probe` records warning evidence with
both fact objects; the mismatch alone is not a fatal Connection failure.
The explicitly selected `codex-mcp-2025-06-18-v1` profile owns MCP
session/thread/turn metadata. Malformed MCP metadata, inconsistent nested and
top-level thread coordinates, and registered-session mismatch use
`host.codex.metadata_malformed`,
`host.codex.session_thread_turn_inconsistent`, and
`host.codex.registered_session_correlation_mismatch` respectively.

Each MCP process start creates an opaque Registry runtime-session ID before
host thread metadata exists. `session_source` is exactly `managed_host`,
`manual_cli`, `cli_preflight`, or `integration_probe`. Only atomic successful
launch-lease consumption can create `managed_host`, and only `managed_host` can
authorize an Agent Connection call. Public `volicord mcp serve` always
records `manual_cli`; preflight creates no session, and integration probes
never count as managed host activity. The runtime session retains its owning Connection and Connection
integration revision.

After a valid initialize request, that runtime owns one session-scoped typed
MCP selection. Its `McpSessionMilestones` retain the runtime, source,
Connection, integration revision, process start, actual peer `clientInfo`,
requested/selected/negotiated protocol revisions, initialize and initialized-
notification completion, `tools/list` time and exact deterministic returned
tool identities, required-tool validation time, canonical verification-tool
identity/time, terminal finding, and last observation. Invalid combinations
are rejected: negotiation requires completed initialization, required-tool
success requires an actual list observation, verification-tool success
requires same-session required-tool validation, and a managed capability proof
requires `session_source=managed_host`. Reconnection creates a new runtime and
new milestones; profiles and milestones are not shared across processes.

Connection report context gathers session IDs from check evidence as well as
finding correlation. Each entry preserves `latest_attempt` and/or
`latest_complete_proof` roles; when one session has both roles it appears once
with both roles. Human and JSON projections expose the same role assignment.

The project integration revision extends the Connection revision with the
current project workflow-policy fingerprint and current Guard installation
identity/policy hash, or explicit absence of Guard ownership. `host_sessions`
retains the revision-scoped local session ID, Connection, exact native session,
and observation times. `host_turns` retains turns shared by both contract
sources. `host_tool_invocations` retains hook tool-use IDs and canonical tool
names. Store derives the local session ID with a domain-separated digest over
the Connection internal ID, exact project integration revision, and exact
native session only after resolving the project and validating current Guard
ownership. Callers cannot supply the complete local ID, and the stored project
revision is immutable.

The `codex-hooks-v1` parser yields `CodexHookPromptCorrelation` for
`UserPromptSubmit` and `CodexHookToolCorrelation` for `PreToolUse` and
`PostToolUse`. Prompt correlation requires only session and turn. Tool
correlation additionally requires tool-use ID and canonical tool name. No hook
phase has a thread coordinate. The `codex-mcp-2025-06-18-v1` parser instead
yields `CodexMcpCorrelation`, for which session, thread, and turn are required.
Store phase checks and SQL discriminators reject cross-source or incomplete
combinations.

Guard correlation and Guard policy are separate steps. A compatible hook
correlation may reach policy and produce `Continue`, `ContinueWithContext`,
`ContinueWithWarning`, or `Deny`. An incompatible hook contract instead
records an observation failure when possible, produces no policy decision, and
does not satisfy the phase. In the Codex `record` profile, the adapter continues
that host action with bounded context and exit `0`; it does not convert the
observation failure into denial. Event persistence unavailability follows the
same non-synthetic-denial rule. Only an explicit compatible `PreToolUse`
policy `Deny` becomes a Codex permission denial. `PostToolUse` output can warn
about or require reconciliation of an already-completed action, but cannot
claim it prevented the action.

The host-neutral boundary is `GuardHookOutcome`: observation outcome, optional
policy decision, bounded diagnostics, and safe feedback. The Codex adapter,
not Core or Store, owns stdout hook JSON, stderr, process exit, context,
warning, and denial projection.

A Guard observation may create normalized host, turn, and tool rows but never
creates the MCP-only `managed_mcp_sessions` row. The first actual managed MCP
tool call validates the current managed runtime without mutation, establishes
or validates that exact MCP anchor, revalidates current owner inputs while
reserving the cross-database binding, and only then attaches its runtime in a
final project transaction. Connection, project, Guard Installation, revision,
native-session, thread, or existing-runtime conflicts detected against the MCP
anchor leave no new Registry binding. The anchor may remain unbound if a later
Registry reservation fails, but it is not authorization. A Registry
reservation left by interruption before final attachment is also not
authorization; exact replay under unchanged owner state reuses it and finishes
the attachment. An attached MCP session cannot be rebound across a runtime
session, Connection, project, host session, or host thread.

Runtime rows are historical process observations, not launch leases or
liveness claims. Launch leases exist only to authorize one bootstrap
transition and do not turn runtime rows into liveness records. A crashed
process may leave an apparently open row, and multiple
cooperative Codex processes may be current concurrently. Neither condition
blocks Guard correlation: no runtime is guessed from open rows, and different
host sessions bind independently.

A real Connection mode transition is one Registry transaction over the
Connection and the strict manifest of every owned Guard Installation. The
transaction changes only the mode, integration generation, stored verification
report, manifest integration revisions, and affected Registry timestamps. It
does not change Guard commands, managed-file inventories, policy hashes, host
configuration, or Product Repository files. All candidates must be complete,
current, and owner-matched before any write; otherwise no transition is
committed.

These records establish locally observed cooperative protocol/session ownership
under current configuration. They do not establish client, host, actor,
operating-system-user, or human identity. MCP client name/version and observed
host executable version accept arbitrary bounded future values and remain
diagnostics only.

The integration-instance ID, integration generation, and derived integration
revisions are local lifecycle and correlation coordinates. Store owns their
lifecycle inputs; callers cannot select them.

### Managed in-chat integration verification

`GuardIntegrationVerificationRun` is the durable, bounded proof unit for the
first-party in-chat workflow. It records its verification ID, Connection,
project, managed MCP runtime, native host session and turn, Guard Installation,
integration revision, policy hash, hook-contract digest, expected probe tool,
creation and expiry, lifecycle status, probe acknowledgement, completion,
matched prompt/pre/post event IDs, and terminal finding. Status is exactly
`active`, `passed`, `failed`, or `expired`. One Connection/runtime/turn/revision
coordinate has at most one active run, and begin replay under unchanged
ownership returns the same active or passed run.

Only an actual current `managed_host` call can begin, probe, or read a run.
Manual stdio, CLI preflight, and integration probes cannot create success. A
pass requires the prompt, pre-tool, and post-tool records to belong to the same
run session and turn; pre/post must share the tool-use ID, generated exact probe
name, and verification-ID input. The current Guard Installation, policy hash,
integration revision, hook-contract digest, and managed runtime must still
match. Prompt is no later than pre-tool, and pre-tool is earlier than post-tool.
No historical event search or cross-run phase assembly is allowed.

`guard_verification` is the final Connection check for this workflow. Its check
details expose the verification, runtime-session, host-turn, and matched Guard
event IDs so concise, verbose, and JSON reports share the same evidence
coordinate. `complete` requires this correlated check to pass; unrelated
prompt/pre/post observations do not substitute. The run is cooperative local
evidence only. It does not automate or bypass Codex project trust, modify MCP
trust configuration, establish user or host identity, or create Core workflow
authority.

<a id="validated-agent-session"></a>

## `ValidatedAgentSession`

Core accepts Agent Connection invocation authority only through this
non-serializable typed boundary:

```rust
struct ValidatedAgentSession {
    connection_id: AgentConnectionId,
    project_id: ProjectId,
    runtime_session_id: AgentRuntimeSessionId,
    project_session_id: AgentSessionId,
    integration_revision: IntegrationRevision,
}
```

It is created only after validating all of the following current facts:

1. the Agent Connection exists and is enabled;
2. the project exists and is currently a Connection Project;
3. the runtime session belongs to that Connection;
4. the project session has a non-null runtime binding;
5. the Registry binding exactly matches the runtime, Connection, project,
   project session, and host session;
6. the project session belongs to that runtime session, Connection, and
   project;
7. the runtime and project session revisions match current Connection and
   project integration revisions;
8. the Connection mode allows the requested operation category;
9. `ActorSource::AgentConnection` exactly names the validated Connection;
10. a project-scoped operation exactly names the validated project;
11. the runtime session has `session_source=managed_host`, never `manual_cli`,
   `cli_preflight`, or `integration_probe`.

The adapter validates the authoritative runtime and project rows on every
project tool call before constructing Core invocation context. Executable path,
host version, and client version remain diagnostics outside this authorization
decision.

Core derives the audit basis deterministically:

```text
connection:<connection_id>/session:<project_session_id>/revision:<project_integration_revision>
```

This basis is the deterministic local lifecycle and correlation coordinate for
the validated operational ownership recorded in the audit event.

## Codex Adapter Responsibilities

The Codex adapter owns host-specific configuration inspection and mutation:

- discover the Codex configuration target;
- install only the managed entry selected by current Connection inputs;
- project the canonical managed MCP launch contract into Codex TOML and parse
  it strictly back into the same contract;
- validate that exact current entry again before issuing a one-time launch
  lease and entering managed stdio;
- detect missing, modified, or extra managed configuration as drift;
- report executable availability and bounded host version diagnostics;
- repair owner-defined managed state from current canonical inputs; and
- uninstall only matching Volicord-managed state.

The Codex adapter does not classify native Linux or WSL2 and does not validate
the process target or filesystem restrictions. Those observations and checks
belong to the platform filesystem boundary under
[System Requirements](system-requirements.md).

Setup and repair plan and validate first, apply or adopt host configuration,
then commit the resulting managed fingerprint and owner-coherent Registry and
Guard state. Verification begins only from that final Connection record and
persists its report against that record's exact integration revision. Report
persistence never performs a second fingerprint update.

Runtime authorization validates the current enabled Connection, project
membership, allowed mode, managed runtime session, revision-scoped project
session, and exact Registry/project binding. Command names, executable paths,
version strings, Runtime Home configuration, and local session metadata are
diagnostic or routing facts and do not establish actor or human identity. The
launch lease is an evidence-integrity transition coordinate, not an operating-
system actor credential.

Repair does not overwrite unrelated Codex configuration or silently change the
selected project, Connection, intent, profile, or platform environment.
For both `workflow` and `read_only`, repair may restore missing owned Codex
configuration, Guard files, and the current Guard Installation while preserving
the Connection mode and integration generation. If repair applies a different
canonical managed configuration, it records the new managed fingerprint,
changes the integration revision, and invalidates the prior verification
report before verification under the new revision. If the fingerprint is
unchanged, the current revision is preserved.
Uninstall removes only content whose current managed identity still matches
Volicord ownership.

An explicit connection-removal command also retires connection-owned Registry
integration state. Removing one membership deletes its Registry project-session
bindings and Guard Installation. A multi-project personal Connection remains,
with its connection-wide runtime sessions and other projects' owned rows, until
its last membership is removed. Last-membership removal deletes all remaining
Registry project-session bindings, Guard Installations, runtime sessions, and
the Agent Connection. Project-local Agent Sessions, Guard observations,
workflow history, evidence, and other authority records remain historical
Product Repository state. They cannot authorize a later call without a current
Registry membership and a current validated runtime/project session.

Connection migration uses the same project-scoped Registry retirement order.
For a superseded multi-project Connection, it removes only the selected
project's runtime/project bindings, Guard Installation, and membership in the
atomic replacement activation transaction. For a superseded last-project
Connection, it keeps the old Connection disabled with that complete project
inventory and a pending-host-cleanup marker until external cleanup succeeds.
The final Registry transaction revalidates the replacement and exact retained
inventory before deleting the bindings, Guard Installation, and membership and
clearing the marker. Cleanup or revalidation failure leaves the inventory
intact for retry; successful cleanup retains the disabled zero-membership
historical Connection and its connection-wide runtime sessions.

## Threat Model

Trusted:

- the same operating-system user account;
- the `Volicord Runtime Home` owned by that account; and
- that account's Store write access.

Untrusted:

- external host and client input;
- a manual, integration-probe, stale, closed, or wrong-revision session;
- a session for another project, runtime, or Connection;
- manually modified configuration; and
- client/host version and process metadata as identity claims.

Tampering with Runtime Home by a malicious process running with the same user
permissions is outside the first-release threat model. The local records are
therefore cooperative and are not tamper-proof against another process with the
same account access.

## Adjacent Owners

- Managed stdio MCP behavior: [MCP Transport](mcp-transport.md).
- Install, verify, repair, and uninstall commands:
  [Administrative CLI](admin-cli.md).
- Platform cells and WSL2 topology:
  [System Requirements](system-requirements.md).
- Ordinary build, package, platform, and release validation:
  [Validation](../maintain/validation.md).
- Runtime and repository path boundaries:
  [Runtime Boundaries](runtime-boundaries.md).
- Security guarantees and non-guarantees: [Security](security.md).
