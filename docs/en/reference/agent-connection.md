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
- release-cell execution or exact artifact evidence; see
  [Host Release Evidence](host-release-evidence.md);
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
  code: string | null
  summary: string
  details: object | null
  observed_at: UtcTimestamp | null

ConnectionAction:
  id: string
  instruction: string
  command: string | null
```

Every member shown above is required, including nullable members and arrays.
Unknown members, duplicate JSON keys, duplicate check IDs, duplicate action
IDs, noncanonical ordering, and unknown status values are invalid. Check IDs,
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

`dry_run` is an operation mode, never a connection or check status.
Configuration matching, executable availability, protocol and host versions,
capability observations, and observation timestamps belong in check facts;
they do not introduce another public or persisted status enum.

User instructions appear only in `actions` inside this report. Registry
storage does not keep an independent verification status or action array. A
connection with no completed persisted report is projected as a synthesized
`status=action_required` report containing one `verification_not_run` pending
check and one verification action. Reading that projection does not persist it.

Operational compatibility is reported from checks the adapter actually
performed and behavior it observed. `complete` does not mean exact-artifact
release certification, operating-system enforcement, actor identity proof,
correctness proof, or tamper-proof recording. Connection verification does not
issue a runtime authorization credential.

## Integration Revisions And Operational Sessions

The current Connection integration revision is a typed, domain-separated
canonical SHA-256 digest. Its basis is the Agent Connection identity, host
kind, intent, scope, mode, server name, configuration target, and current exact
managed-configuration fingerprint. That fingerprint covers the managed server
command and entry.

Revision construction excludes observed host version, executable path or
digest, support-catalog coordinates, release evidence, certified capability
sets, and MCP client name/version. Those values cannot change authorization.

Each MCP process start creates an opaque Registry runtime-session ID before
host thread metadata exists. `session_source` is exactly `managed_host` or
`cli_preflight`. Only `managed_host` can authorize an Agent Connection call.
The runtime session retains its owning Connection and Connection integration
revision.

The project integration revision extends the Connection revision with the
current project workflow-policy fingerprint and current Guard installation
identity/policy hash, or explicit absence of Guard ownership. A project Agent
Session retains that revision and cannot be rebound across a runtime session,
Connection, or project.

These records demonstrate locally observed cooperative protocol/session
ownership under current configuration. They do not identify a binary, host,
client, actor, operating-system user, or human. MCP client name/version and
observed host executable version accept arbitrary bounded future values and
remain diagnostics only.

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
4. the project session belongs to that runtime session, Connection, and
   project;
5. the runtime and project session revisions match current Connection and
   project integration revisions;
6. the Connection mode allows the requested operation category;
7. `ActorSource::AgentConnection` exactly names the validated Connection;
8. a project-scoped operation exactly names the validated project;
9. the runtime session has `session_source=managed_host`, never
   `cli_preflight`; and
10. client name/version and host version are ignored for authorization.

The adapter validates the authoritative runtime and project rows on every
project tool call before constructing Core invocation context. There is no
receipt path, release-evidence path, compatibility path, or fallback.

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

Runtime authorization does not hash the parent executable, compare a platform
release coordinate, consult an embedded support catalog, calculate a binding or
verifier-build digest, or issue/load/validate a host verification receipt. A
recognizable command name, process path, version string, environment value, or
local session is not actor identity. Managed launch context and authoritative
Store sessions establish only the cooperative ownership boundary above.

Repair does not overwrite unrelated Codex configuration or silently change the
selected project, Connection, intent, profile, or platform environment.
Uninstall removes only content whose current managed identity still matches
Volicord ownership.

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
- Exact Codex release artifacts and release-only capabilities:
  [Host Release Evidence](host-release-evidence.md).
- Runtime and repository path boundaries:
  [Runtime Boundaries](runtime-boundaries.md).
- Security guarantees and non-guarantees: [Security](security.md).
