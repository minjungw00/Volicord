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
| Adapter modules, filesystem helpers, generated launch markers, and Store query helpers | `internal` | They preserve the stable boundary but are not public surfaces. |
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
| Transport | Volicord-managed stdio MCP started with `volicord mcp --stdio` |
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

One typed managed MCP launch contract is the canonical source for the
executable command, stdio arguments, static and forwarded environment bindings,
the personal/shared distinction, managed provenance, strict launch-shape
validation, canonical projection, and deterministic managed-fingerprint inputs.

A personal connection requires the selected canonical absolute Runtime Home
and selected absolute `volicord` executable. It stores the Runtime Home and
managed host, launch, and Connection markers as static environment values and
forwards no parent-environment name:

```toml
[mcp_servers.volicord]
command = "/absolute/path/to/volicord"
args = ["mcp", "--stdio", "--connection", "<connection_id>"]

[mcp_servers.volicord.env]
VOLICORD_HOME = "/absolute/runtime/home"
VOLICORD_MCP_CONNECTION_ID = "<connection_id>"
VOLICORD_MCP_HOST = "codex"
VOLICORD_MCP_LAUNCH = "managed_host"
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
args = ["mcp", "--stdio", "--discover-repository", "--host", "codex"]
env_vars = ["VOLICORD_HOME"]
```

The shared launch contains no absolute executable, Runtime Home, Connection ID,
project ID, or other machine-local lifecycle coordinate. A personal entry with
a project argument or project environment marker, a static/forwarded
environment collision, a blank or duplicate forwarded name, an incomplete
personal binding, or a mixed personal/shared argument or environment shape is
invalid.

Generated Codex configuration is an adapter projection of this contract. CLI
verification materializes both preflight and the stdio self-test from the same
contract. Neither consumer adds a project selector, platform identity, or
WSL-specific field.

The Codex adapter parses the current TOML shape and validates a managed entry by
reconstructing this same typed contract. Unknown launch keys, malformed values,
and noncanonical shapes are drift rather than a second accepted form. The
adapter preserves only a valid `tools.<known-tool>.approval_mode` overlay and
excludes it from launch identity. The managed fingerprint covers the canonical
launch projection together with host kind, scope, and server name. Formatting
differences do not change it; a launch-semantic difference does.

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
`guard_observation`, `host_executable`, `host_session`, `managed_config`,
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
managed_config -> process_startup -> host_session -> required_tools -> tool_round_trip
managed_config -> mcp_server
guard_files -> guard_hook_execution -> guard_observation
```

`host_session` is the managed-host `initialize` check, `required_tools` is the
managed-host `tools/list` check, and `tool_round_trip` is the canonical
verification-role tool-call check. `ToolVerificationRole::ManagedHostRoundTrip`
has exactly one canonical owner, currently `volicord.list_projects`. When no managed-host attempt exists, the four checks
from `process_startup` through `tool_round_trip` are `pending`. An initialize
failure makes `host_session` `failed` and makes `required_tools` and
`tool_round_trip` `blocked` by that same root finding. A managed-configuration
failure blocks both `mcp_server` and the process/protocol chain. A Guard-file
integrity failure blocks hook execution and phase observation.

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
| `host_session` | A current, host-version-fresh managed-host session completed `initialize` and its initialized notification. | It waits for a qualifying attempt and is blocked by `process_startup`. | The current attempt observed an initialization or protocol failure. |
| `required_tools` | A qualifying `tools/list` observation contains every required tool. | It waits for tool discovery and is blocked by `host_session`. | Tool discovery completed with missing or invalid required tools. |
| `tool_round_trip` | A qualifying current, host-version-fresh session records both `verification_tool_name=volicord.list_projects` and `verification_tool_observed_at`. | It waits for the canonical role-owner call and is blocked by `required_tools`. | The call itself observed a protocol or contract failure, or the recorded tool name differs from the current canonical owner. |
| `project_trust` | Project trust is satisfied. | A normal trust or reload action is `pending`; scopes with no separate trust check are `not_applicable`. | Trust configuration is malformed or contradictory. |
| `guard_files` | Every current Guard manifest file expectation matches. | Applies when Guard is part of the Connection profile. | A managed file, manifest, wrapper, ownership, or executable-integrity check failed. |
| `guard_hook_execution` | A current managed Guard hook executed. | It waits for current hook activity and is blocked by `guard_files`. | Hook execution itself recorded a failure. |
| `guard_observation` | Every required current typed hook phase was observed. | It waits for remaining phases and is blocked by `guard_hook_execution`. | A current event reports an incompatible hook contract. |

The CLI MCP self-test creates only `session_source=cli_preflight`; it never
satisfies `process_startup`, `host_session`, `required_tools`, or
`tool_round_trip`. Guard uses `guard_files`, `guard_hook_execution`, and
`guard_observation` as top-level operational checks.
The strict Guard manifest owns the current policy hash, integration revision,
typed runtime commands, complete Volicord-managed artifact expectations, and
required hook phases. Policy and runtime commands are distinct projections of
one typed invocation. Audit compares every managed artifact with its canonical
current expectation, and Guard observation requires compatible current events
for every required phase.

A recorded verification-tool name mismatch never passes `tool_round_trip`.
The check fails with `tool_round_trip_designation_mismatch`, and active
verification persists
`mcp.tool_verification.designation_mismatch`. Its bounded facts expose the exact
`expected_tool_name` and `observed_tool_name`; JSON check details and verbose
output likewise show the exact expected and observed names. A prior revision,
CLI preflight row, missing timestamp, or stale host-version observation cannot
substitute for the current exact pair.

Any bounded Codex version proceeds through these behavioral checks. A changed
version makes the current host observation pending until Codex is reloaded and
its operational behavior is observed again.

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
checks, bounded findings and cause edges loaded through Store APIs, derived
root IDs, one deduplicated typed action per root, Connection context,
operation-specific result details, and report limits. Concise, verbose, and
JSON output are projections of this same report and identify the same roots.
No renderer derives a cause or remediation category from summary prose, and no
projection re-exposes a fact redacted by `DiagnosticFinding`.

A current Connection report selects findings from the exact IDs referenced by
its `failed` and `blocked` checks and then loads only their bounded cause
chains. An independent current finding appears only when the operation
deliberately selects it; the report never treats every stored finding on the
same integration revision as current. CLI-owned current-state operational
findings identify the exact managed-config target, Product Repository, Guard
managed artifact, Guard phase or event, Guard Installation, or runtime session
in their typed subject. Stable IDs include a bounded digest of that canonical
subject, so the same diagnostic code on two artifacts or phases remains two
findings and re-observing one subject refreshes only its snapshot.

The JSON projection includes `generated_at` as the report time and the exact
current integration revision in Connection context when one exists. The
persisted verification `checked_at` remains the observation time for that
verification and is not repeated as a competing top-level time. Status may
rebuild an in-memory current projection from stored active-probe facts and
current observations, but that read does not persist the projection or modify
any timestamp. A referenced finding row that is absent is represented as the
typed `diagnostics.finding_record_missing` observation and directs the operator
to rebuild current observations; rendering does not fabricate the missing
domain facts.

Active verification captures the exact typed Connection integration revision
before it plans or probes. Store persists the resulting report only when that
same revision is still current, using one immediate Registry transaction for
the comparison and report replacement. The write changes only
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
diagnostic facts. A host-version change renews operational observation, while
authorization uses the current Connection, revisions, and authoritative
runtime/project session bindings.

The actual MCP peer is the bounded `clientInfo.name` and `clientInfo.version`
observed on that runtime session. The PATH probe is the separately observed
Codex executable path and version. Reports and findings never substitute one
for the other. When both versions exist and differ,
`host.codex.peer_version_differs_from_path_probe` records warning evidence with
both fact objects; the mismatch alone is not a fatal Connection failure.
Malformed native metadata, inconsistent session/thread/turn coordinates,
registered-session mismatch, and managed-marker mismatch use
`host.codex.metadata_malformed`,
`host.codex.session_thread_turn_inconsistent`,
`host.codex.registered_session_correlation_mismatch`, and
`host.codex.managed_marker_mismatch` respectively.

Each MCP process start creates an opaque Registry runtime-session ID before
host thread metadata exists. `session_source` is exactly `managed_host` or
`cli_preflight`. Only `managed_host` can authorize an Agent Connection call.
The runtime session retains its owning Connection and Connection integration
revision.

After a valid initialize request, that runtime owns one session-scoped typed
MCP selection. It retains the requested protocol string, selected production
profile, exact-match or server-counter-offer outcome, client capabilities,
bounded attempted client name/version, and whether the initialized notification
completed the handshake. The selected profile generates the initialize result
revision and capabilities and governs later lifecycle validation. Selection is
not negotiation completion: only the required valid initialized notification
marks the profile negotiated and records its revision as the authoritative
runtime-session protocol observation. Reconnection creates a new runtime and a
new selection; profiles are not shared or inherited across processes.

The project integration revision extends the Connection revision with the
current project workflow-policy fingerprint and current Guard installation
identity/policy hash, or explicit absence of Guard ownership. A project Agent
Session retains that revision, a deterministic revision-scoped session ID,
Connection, host session/thread/latest turn, and first/last observation times.
The Store derives the internal ID with a domain-separated digest over the
Connection internal ID, exact project integration revision, and exact
host-native session ID only after resolving the project and validating current
Guard ownership. Callers cannot supply the complete internal ID. The stored
project revision is immutable: a later Connection mode generation, physical
Connection recreation, project-policy revision, or Guard ownership revision
creates a different project Agent Session row for the same native session and
leaves the earlier row as history.
A Guard observation may create it with a null runtime binding. The first actual
managed MCP tool call for that host session validates the current managed
runtime without mutation, establishes or validates the exact project Agent
Session anchor, revalidates the current owner inputs while reserving the exact
cross-database binding, and only then attaches its runtime in a final project
transaction. Connection, project, Guard Installation, revision, native-session,
thread, or existing-runtime conflicts detected against the project anchor
leave no new Registry binding. The project anchor may remain unbound if a later
Registry reservation fails, but it is not authorization. A Registry reservation
left by interruption before final attachment is also not authorization; exact
replay under unchanged owner state reuses it and finishes the attachment. An
attached session cannot be rebound across a runtime session, Connection,
project, host session, or host thread.

Runtime rows are historical process observations, not leases or liveness
claims. A crashed process may leave an apparently open row, and multiple
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
11. the runtime session has `session_source=managed_host`, never
   `cli_preflight`.

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
version strings, environment values, and local session metadata are diagnostic
or routing facts and do not establish actor or human identity.

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
- a CLI-preflight, stale, closed, or wrong-revision session;
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
