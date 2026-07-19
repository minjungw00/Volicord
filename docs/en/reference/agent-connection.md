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
- authoritative managed-host runtime and project session ownership;
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
| User-owned action delivery | CLI inbox |
| Platform environment | `linux`, `macos`, `native_windows`, or `wsl2` |

A `personal` connection installs user-owned local Codex configuration. A
`shared` connection installs project-owned Codex configuration inside the
selected `Product Repository`. Both identify one registered Connection and its
allowed projects through the Volicord-generated managed launch/configuration
context.

A genuinely new Agent Connection defaults to `workflow`. Setup replay and
repair use the persisted mode of a matching current Agent Connection exactly;
they never infer mode from the Record profile or perform a mode transition.
`volicord connection mode` is the only command that changes an established
mode and increments the integration generation.

An Agent Connection is a stored local integration record in the
`Volicord Runtime Home`. It does not grant operating-system permission,
establish user identity, or prove that Codex loaded the managed entry. One
managed stdio MCP process is bound to one current Agent Connection.

User-owned actions are delivered through the CLI inbox. An MCP agent may
request an owner-defined action, but it cannot act as the local user channel or
resolve the action on the user's behalf.

<a id="connection-verification-report"></a>

## `ConnectionVerificationReport`

One small report is the canonical serialized connection-verification state:

```yaml
ConnectionVerificationReport:
  status: complete | action_required | failed
  checked_at: UtcTimestamp
  checks: ConnectionCheck[]
  actions: ConnectionAction[]

ConnectionCheck:
  id: ConnectionCheckId
  status: passed | pending | failed
  code?: string
  summary: string
  details?: object
  observed_at?: UtcTimestamp

ConnectionAction:
  id: string
  instruction: string
  command?: string
```

`status`, `checked_at`, `checks`, `actions`, and each check or action's
non-optional members are required. Optional `code`, `details`, `observed_at`,
and `command` members are omitted when absent rather than serialized as null.
Unknown members, duplicate JSON keys, duplicate check IDs, duplicate action
IDs, noncanonical ordering, explicit null for an optional member, and unknown
status values are invalid. Check IDs,
action IDs, and non-null check codes are 1 through 128 ASCII bytes and match
`[a-z][a-z0-9_]*`. `summary`, `instruction`, and non-null `command` values are
1 through 4,096 UTF-8 bytes and contain no NUL. A non-null `details` value is a
JSON object whose serialized form is at most 16 KiB. A report contains at most
64 checks and 32 actions, and its serialized form is at most 64 KiB.

Checks are sorted by `id` in ascending UTF-8 byte order. Actions use the same
ordering by `id`. Strict decoding rejects another order rather than silently
normalizing it.

Every check in the report is required for that report. The top-level status is
derived and cannot disagree with the checks:

1. any `failed` check produces `status=failed`;
2. otherwise any `pending` check produces `status=action_required`;
3. otherwise `status=complete`.

The current Codex connection report contains these operational checks:

| Check ID | `passed` | `pending` | `failed` |
|---|---|---|---|
| `managed_config` | The selected target contains the canonical managed entry. | Never used after an active inspection. | The required entry is missing, malformed, owned by another entry, changed, or unavailable to inspect. Details name the target and the precise cause. |
| `host_executable` | `codex` was discovered on `PATH` and its version command succeeded. | The read-only status path has no prior active probe to project. | Discovery or the version command failed. Path and version are diagnostic only. |
| `mcp_server` | The Volicord CLI self-test passed preflight, `initialize`, `tools/list`, the current required-tool set, and a safe read-only `volicord.list_projects` call. | Never used after an active self-test. | Process startup, storage preflight, initialization, tool discovery, required-tool validation, or the safe call failed. |
| `host_session` | At least one `managed_host` session for the current integration revision and current host-version observation completed `initialize`. | No qualifying managed-host use was observed, only an older revision was observed, initialization is still absent, or the Codex version changed after the observation. | When no qualifying success exists, the newest current attempt recorded an actual initialization or protocol failure. |
| `required_tools` | At least one current, host-version-fresh managed-host `tools/list` observation contains every tool required by the current mode. | No qualifying tool-list observation exists. | When no qualifying success exists, the newest current managed host actually omitted required tools or returned invalid tool-list data. |
| `tool_round_trip` | At least one current, host-version-fresh managed-host session completed a safe read-only Volicord tool call. | No such qualifying current observation exists. | When no qualifying success exists, the newest current attempt recorded an actual protocol or contract incompatibility. |
| `project_trust` | Project trust is satisfied, or no separate project trust applies. | A normal Codex trust or reload action remains. | The trust configuration is malformed or contradictory and cannot be resolved by the normal action. |
| `guard_files` | Every current Guard manifest file expectation matches, including canonical content, owner fields, markers, wrapper runtime commands, and required executable behavior. | A newly applied configuration still needs the normal host reload step. | A required managed file is missing, malformed, content- or ownership-mismatched, or lacks required executable behavior. |
| `guard_observation` | Every required typed hook phase was observed for the manifest's exact policy hash and integration revision. Prompt capture is reported as a detail of this check. | Files are valid, but one or more current required phases have not yet been observed. Older policy-hash or integration-revision events do not satisfy the check. | A current Guard event reports a malformed or incompatible hook contract. |

The CLI MCP self-test creates only `session_source=cli_preflight`; it never
satisfies `host_session`, `required_tools`, or `tool_round_trip`. Guard uses
only `guard_files` and `guard_observation` as top-level operational checks.
The strict Guard manifest describes Volicord-managed files, runtime commands,
policy ownership, and required phases; it does not certify host capability or
store a Guard installation lifecycle status.

Any bounded Codex version is eligible for these behavioral checks. A changed
version makes the current host observation pending until Codex is reloaded and
observed again; it is not an unsupported-host or failed-artifact result.

`dry_run` is an operation mode, never a connection or check status.
Configuration matching, executable availability, protocol and host versions,
capability observations, and observation timestamps belong in check facts;
they do not introduce another public or persisted status enum.

User instructions appear only in `actions` inside this report. They are ordered
and deduplicated by stable ID and are derived directly from pending or failed
checks. Reload and first-use actions require actual Codex activity to be
observed. A passing `guard_files` check does not request Guard file
reinstallation. Registry storage does not keep an independent verification
status or action array. A
connection with no completed persisted report is projected as a synthesized
`status=action_required` report containing one `verification_not_run` pending
check and one verification action. Reading that projection does not persist it.

The administrative CLI projects the report's checks and actions directly into
the top-level `ConnectionCommandReport`. It does not nest this report, repeat
its aggregate status, or expose `checked_at` as a second command-output time.
Status may rebuild an in-memory current projection from the stored active-probe
facts and current observations, but that read does not persist the projection
or modify any timestamp.

Active verification captures the exact typed Connection integration revision
before it plans or probes. Store persists the resulting report only when that
same revision is still current, using one immediate Registry transaction for
the comparison and report replacement. The write changes only
`verification_report_json` and the ordinary row update timestamp. A revision
conflict leaves the existing report and every owner field unchanged and
requires verification to be rerun. Verification observes managed
configuration; it never applies, adopts, or records a newly planned managed
fingerprint.

Operational compatibility is reported from checks the adapter actually
performed and behavior it observed. `complete` does not certify executable
identity or provenance and does not mean operating-system enforcement, actor
identity proof, correctness proof, or tamper-proof recording. Connection
verification does not issue a runtime authorization credential.

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

Revision construction excludes observed host version, executable path or
cryptographic identity, allowlist coordinates, claimed capability sets, and
MCP client name/version. Those values cannot change authorization.

Each MCP process start creates an opaque Registry runtime-session ID before
host thread metadata exists. `session_source` is exactly `managed_host` or
`cli_preflight`. Only `managed_host` can authorize an Agent Connection call.
The runtime session retains its owning Connection and Connection integration
revision.

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
managed MCP tool call for that host session reserves the cross-database binding
and attaches its runtime. An attached session cannot be rebound across a
runtime session, Connection, project, host session, or host thread.

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

These records demonstrate locally observed cooperative protocol/session
ownership under current configuration. They do not identify a binary, host,
client, actor, operating-system user, or human. MCP client name/version and
observed host executable version accept arbitrary bounded future values and
remain diagnostics only.

The integration-instance ID and integration generation are local lifecycle
coordinates only. Neither is host identity, actor identity, release
certification, a security credential, or caller-controlled input.

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
   `cli_preflight`; and
12. client name/version and host version are ignored for authorization.

The adapter validates the authoritative runtime and project rows on every
project tool call before constructing Core invocation context. No alternate
authorization, compatibility, or fallback path exists.

Core derives the audit basis deterministically:

```text
connection:<connection_id>/session:<project_session_id>/revision:<project_integration_revision>
```

This basis names local operational ownership. It is not a certificate,
receipt, identity proof, bearer token, host attestation, or trusted host
digest.

## Codex Adapter Responsibilities

The Codex adapter owns host-specific configuration inspection and mutation:

- discover the Codex configuration target and platform environment;
- install only the managed entry selected by current Connection inputs;
- generate the command, arguments, Runtime Home forwarding, and managed launch
  markers used to select the Connection and optional project at startup;
- detect missing, modified, or extra managed configuration as drift;
- report executable availability and bounded host version diagnostics;
- repair owner-defined managed state from current canonical inputs; and
- uninstall only matching Volicord-managed state.

Setup and repair plan and validate first, apply or adopt host configuration,
then commit the resulting managed fingerprint and owner-coherent Registry and
Guard state. Verification begins only from that final Connection record and
persists its report against that record's exact integration revision. Report
persistence never performs a second fingerprint update.

Runtime authorization does not hash the parent executable, derive a platform
executable identity, consult an exact-host allowlist, calculate an executable
identity digest, or issue or validate an executable attestation. A recognizable
command name, process path, version string, environment value, or local session
is not actor identity. Managed launch context and authoritative Store sessions
establish only the cooperative ownership boundary above.

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
permissions is outside the first-release threat model. This contract adds no
binary attestation, operating-system keystore, signing, key rotation, or
revocation.

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
