# MCP transport reference

This document owns the first-release local MCP process boundary: managed stdio
startup, strict binding, JSON-RPC lifecycle, tool discovery, public argument
projection, response wrapping, and shutdown. Core methods, Codex configuration,
connection verification, and storage effects remain with their focused owners.

<a id="surface-stability"></a>
## Surface Stability

Labels use [Documentation Policy](../maintain/documentation-policy.md#surface-stability-labels).

| Surface | Stability |
|---|---|
| `volicord mcp --stdio`, initialization, `tools/list`, `tools/call`, and response wrapping | `stable` |
| Authoritative runtime-session lifecycle milestones | `stable` |
| Pre-1.0 additions not listed in the stable process and method set | `beta` |
| Managed launch markers and generated configuration details | `internal` |
| Host executable version, MCP client name/version, and best-effort protocol metrics | `diagnostic` |

## Process Model

`volicord mcp --stdio` is a child process started by managed Codex
configuration. It exchanges line-delimited JSON-RPC through stdin and stdout
and opens no TCP, HTTP, Unix-domain socket, or other network listener.

```text
volicord mcp --stdio --connection <connection_id> [--project <project_id>]
volicord mcp --stdio --discover-repository --host codex
volicord mcp --check --connection <connection_id> [--project <project_id>]
```

The bound form uses exact stored identifiers from the generated managed entry.
Repository discovery is only for the canonical shared Codex entry and resolves
the Connection and project from the exact Runtime Home and canonical Git work
tree. It does not infer a connection from cwd
alone, scan nearby repositories, or accept another host selector. `--check`
performs preflight without entering the stdio loop.

## Environment And Startup

`VOLICORD_HOME` selects the Runtime Home according to
[Runtime Boundaries](runtime-boundaries.md). The canonical managed launch
contract stores the selected absolute value in personal configuration and
forwards the parent value only in shared configuration, which embeds no
machine-local path. Exact generated shapes and strict parsing belong to
[Agent Connection](agent-connection.md#managed-mcp-launch-contract).

Before reading MCP requests, the adapter resolves the exact registered
Connection from the Volicord-generated managed launch/configuration context and
validates that it is enabled, its selected projects are current members, the
Runtime Home and Product Repository are separated, the `StorageManifest` is
current, and required storage is readable. Managed launch markers classify the
cooperative process source but do not prove client, host, actor, or human
identity. Corrupt records, ambiguous selection, and unavailable storage use the
[Failure Model](failure-model.md).

After startup resolves that managed Connection, it immediately records a
Registry runtime session with the current Connection integration revision and
the `managed_host` or `cli_preflight` source. Executable path, host version, and
client version remain diagnostics; managed-call authorization is established
from the current session and project bindings described below.

## MCP Wire Behavior

Each non-empty stdin line is one complete UTF-8 JSON-RPC 2.0 request. Malformed
JSON returns `-32700`; invalid requests return `-32600`; unknown methods return
`-32601`; invalid parameters return `-32602`; and internal protocol failures
return `-32603`. Responses preserve the request `id`.

`initialize` precedes `tools/list` and `tools/call`. The process negotiates only
the supported MCP protocol version and then accepts
`notifications/initialized`. Calls before initialization, repeated initialize,
batch input, and unsupported versions fail before Core.

## Authoritative Lifecycle Recording

After resolving the Agent Connection, the process creates a Registry runtime
session before validating thread metadata or reading a protocol message. The
row identifies this Volicord-generated process launch, its Connection,
`managed_host` or `cli_preflight` source, current connection integration
revision, process ID, and process-start time. A CLI preflight row never
satisfies a managed-host operational check.

The adapter durably records successful `initialize` before returning its
response, records a valid `notifications/initialized` before entering ready
state, and records each actual `tools/list` response before returning it. The
discovery fact says whether that generated response contained every tool
required by the current Connection mode. Duplicate initialized notifications
are idempotent after the first valid observation.

Successful `volicord.status`, `volicord.get_operation_result`,
`volicord.check_close`, and `volicord.list_projects` completions update the
safe/read-only milestone before the tool result is emitted. Observable fatal
transport failure and EOF-driven graceful close record their terminal facts.
An authoritative Store failure withholds the corresponding protocol success.
Bounded writes to `diagnostics.sqlite` remain best effort and are never
consulted for these facts.

Connection verification starts a separate `cli_preflight` process and calls
`volicord.list_projects` as its designated safe read-only round trip. That
process validates the server surface, but its lifecycle facts cannot satisfy a
`managed_host` operational check or authorize a Connection call.

Runtime rows are durable process-launch observations, not liveness records.
A process that exits before recording a terminal failure or graceful close may
leave a row that appears open. Such a row remains historical evidence and is
never selected to correlate a later Guard event. Concurrent managed processes
may coexist and bind different host sessions.

The negotiated protocol version is authoritative protocol data. `clientInfo`
name/version and an observed host executable version are diagnostic fields;
they accept bounded future values. Compatibility is determined from the current
managed configuration and the initialization, tool-list, required-tool,
safe-call, and Guard behavior observed for the current revision. These records
are cooperative and do not establish client, host, actor, or human identity.

## Per-Call Session Authorization

A valid Guard observation may create or update a project `agent_sessions` row
before any MCP runtime is known. That row stores no fabricated or sentinel
runtime coordinate. Before an actual project is selected, MCP runtime state
retains only the exact native session, thread, and turn metadata; it does not
derive or search for a Connection-only internal session coordinate. After
project selection, the Store resolves the current project integration revision
and derives the project Agent Session ID from the Connection, that exact
revision, and the native session. For the first actual managed `tools/call`,
the Store first validates the current managed runtime without mutation. It then
establishes or validates an unbound project Agent Session anchor with the exact
Connection, native session, thread, project revision, and current Guard
ownership. Only after that project transaction commits does the Store
revalidate the current owner facts and reserve the exact Registry
runtime/project/revision/host-session binding. A final project transaction
attaches that runtime to the same anchor. A deterministic project ownership
conflict therefore creates no Registry reservation. An unbound anchor is not
authority, and a Registry reservation without the matching project attachment
is not authority. If the final project write is interrupted, an identical call
under unchanged owner state reuses the reservation and finishes the attachment.
CLI preflight never performs this binding.

Project Agent Session validation therefore precedes Registry runtime
reservation. Authorization requires both the completed project attachment and
the exact completed Registry binding.

Before constructing Core invocation context for a project tool, the adapter
validates the authoritative current Registry runtime session, the exact
`mcp_runtime_project_session_bindings` row, and the project `agent_sessions`
row. The Connection must exist and be enabled; the project must exist and
remain a Connection Project; the runtime session must be a current
`managed_host` session owned by that Connection; and the project session must
be non-null-bound to the same runtime, Connection, project, and host session.
The Registry binding revision and project row revision must be identical, and
both integration revisions must match their current Connection and project
inputs. The current Connection mode must allow the requested operation
category. An unbound Guard-only session retains Guard history but cannot
authorize a tool call. Every real mode transition advances the Store-owned
Connection integration generation, so runtime sessions, project Agent Sessions,
and Guard events from every earlier generation remain historical even if the
Connection later returns to the same mode value.

Core receives one non-serializable `ValidatedAgentSession`. Its Connection ID
must exactly match `ActorSource::AgentConnection`, and its project ID must
match every project-scoped invocation. The audit `verification_basis` is
derived locally as
`connection:<connection_id>/session:<project_session_id>/revision:<project_integration_revision>`.
This value is the deterministic local lifecycle and correlation coordinate for
the validated operational ownership recorded in the audit event.

## Tool Discovery

| Mode and storage | MCP-visible tools |
|---|---|
| `workflow`, writable | `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.get_operation_result`, `volicord.prepare_write`, `volicord.prepare_evidence_capture`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_action`, `volicord.reconcile_changes`, `volicord.check_close`, `volicord.close_task`, `volicord.list_projects` |
| `workflow`, readable only | `volicord.status`, `volicord.get_operation_result`, `volicord.request_user_action` (resume only), `volicord.check_close`, `volicord.list_projects` |
| `read_only`, readable | `volicord.status`, `volicord.get_operation_result`, `volicord.check_close`, `volicord.list_projects` |
| no readable allowed project | `volicord.list_projects` |

Task state and previous calls do not dynamically add tools. A withheld mutation
fails without Core effects. `volicord.resolve_user_action` is a public Core API
method but is never an MCP tool.

## Public Argument Projection

`tools/call` uses string `params.name` and optional object `params.arguments`.
Public schemas hide Core envelopes, internal connection/project IDs, protocol
metadata, idempotency fields, actor source, operation category, and verification
basis. Hidden fields are rejected before Core. Compact discovery schemas never
relax the complete owner-defined request validation.

<a id="mutation-authority-receipt-projection"></a>
## Response Wrapping

Read-only tools return the public method result as structured content.
Mutations return the selected `summary`, `workflow`, or `full` projection with
one fresh `AuthorityReceipt`, exact effect identity, replay facts, and bounded
recovery information. Text is a human rendering, not another authority source.

A delivery failure after a committed Core effect preserves operation-result
coordinates. The adapter does not retry a mutation merely because response
serialization or transport failed.

## UserAction Requests

An MCP agent may create a pending request through
`volicord.request_user_action` or use its explicit read-only resume branch. It
may later observe a safe snapshot of current status and the immutable CLI
resolution identity. It never receives the private inbox form, note, submission
identity, or credentials.

The adapter never answers or resolves the request and sends no server-initiated
resolution request. The user resolves only with `volicord inbox resolve`.
Guard prompt observations, when present, remain non-authority observations.

## Shutdown And Reconnection

EOF closes the loop after in-flight response handling and records graceful
close. A new process repeats
startup validation and MCP initialization; it inherits no connection, project,
session authorization, or current state from the previous process.

## Related Owners

- [Agent Connection](agent-connection.md)
- [Administrative CLI](admin-cli.md)
- [API Methods](api/methods.md)
- [API User-Action Schemas](api/schema-user-action.md)
- [Storage Effects](storage-effects.md)
- [Security](security.md)
