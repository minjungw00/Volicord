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

The `--connection` process form accepts an explicit `--project` for current
manual or preflight selection. The canonical personal Codex entry uses this
form without `--project`; its current project associations remain Store-owned
Connection Project memberships. Repository discovery is only for the canonical
shared Codex entry and resolves the Connection and project from the exact
Runtime Home and canonical Git work tree. It does not infer a connection from
cwd alone, scan nearby repositories, or accept another host selector.
`--check` performs preflight without entering the stdio loop.

## Environment And Startup

`VOLICORD_HOME` selects the Runtime Home according to
[Runtime Boundaries](runtime-boundaries.md). The canonical managed launch
contract stores the selected absolute value in personal configuration and
forwards the parent value only in shared configuration, which embeds no
machine-local path. Exact generated shapes and strict parsing belong to
[Agent Connection](agent-connection.md#managed-mcp-launch-contract).

Connection preflight and the CLI stdio handshake materialize their process
launches from that same contract. Materialization preserves ordinary inherited
process variables, removes Volicord-owned managed-MCP variables, applies static
values, resolves every forwarded name from explicit verification input, and
then applies the CLI-only diagnostic marker. Personal verification therefore
uses the static Runtime Home already in its contract. Shared verification uses
the operation-selected Runtime Home as the forwarded `VOLICORD_HOME` while the
repository-visible configuration remains portable. Shared repository discovery
runs from the canonical Product Repository root; personal verification uses
its bound identifiers without a repository-discovery working-directory
dependency.

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

`initialize` precedes `tools/list` and `tools/call`. Calls before initialize,
repeated initialize, and invalid lifecycle operations fail before Core. A
selected initialization profile does not become negotiated until the required
`notifications/initialized` step completes.

## Protocol Revision Negotiation

The production-supported initialization revisions are exactly:

- `2024-10-07`
- `2024-11-05`
- `2025-03-26`
- `2025-06-18`
- `2025-11-25`

The request's string `protocolVersion` is the requested revision. An exact
member of this closed set selects the same profile and the initialize result
returns the same revision. Any other string that belongs to the
initialization-based protocol shape receives the preferred server counter-offer
`2025-11-25`; as required by the pinned specification, a client that cannot
support the returned revision disconnects. Selection uses exact registry
membership, not lexical or date-range comparison, and the supported set is not
user-configurable.

Missing or non-string `protocolVersion`, non-object `capabilities`, and
malformed `clientInfo` remain `-32602` invalid parameters with bounded error
data. The pinned pre-release `2026-07-28` revision belongs to the discover-based
generation, so an initialize request carrying it fails as a typed method or
generation mismatch instead of receiving an initialization counter-offer.

After valid parameter decoding, the active MCP connection owns one typed,
session-scoped selection. It retains the exact requested string, selected
profile, exact-match or counter-offer outcome, client capabilities, bounded
attempted client name/version, and initialized-notification completion fact.
The selected profile generates the initialize response `protocolVersion` and
capabilities and governs later lifecycle validation. The profile is selected
after a successful initialize request, but its revision is negotiated only
after the valid initialized notification completes the handshake.

Volicord advertises only the `tools` server capability, and only when that
field is permitted by the selected profile; all five supported profiles permit
it. The profile also controls JSON-RPC batching exactly as follows:

| Selected profile | Batch requests and responses |
|---|---|
| `2024-10-07` | disallowed |
| `2024-11-05` | disallowed |
| `2025-03-26` | allowed |
| `2025-06-18` | disallowed |
| `2025-11-25` | disallowed |

These are reviewed profile facts, not chronology inferred from revision names.
A disallowed batch, including one containing `initialize`, is rejected before
initialization state changes or Core execution. An allowed batch is processed
sequentially in input order. Notifications have no response entry, and a batch
containing only notifications produces no response.

## CLI Conformance And Host Compatibility Probes

Connection verification runs a server-conformance matrix over every profile in
the production registry, in its reviewed order. Each profile gets a separate
stdio process and exact requested revision. The probe completes `initialize`,
`notifications/initialized`, `tools/list`, validation against that revision's
pinned schema, current-mode required-tool validation, exactly one
`volicord.list_projects` call, and graceful EOF/shutdown. It records the
requested and negotiated revisions, returned tools, completed stages, and a
typed failure per revision. The aggregate server check passes only if every
production revision passes; one failed revision does not prevent the remaining
revisions from being probed.

Host compatibility is a separate, host-owned fixture list rather than a
projection of the protocol registry. The current `codex` fixture uses the
reviewed Codex initialize request shape with `clientInfo.name` set to
`codex-mcp-client`, the `Codex` title, an empty current capability object, and
the independently pinned revision `2025-06-18`. Its one tool call carries valid
Codex native thread/session/turn correlation metadata. It executes
`tools/list` and `volicord.list_projects`, and it never derives its requested
revision from the server's preferred or newest profile. Multiple independently
pinned `codex` fixtures may coexist when deployed client families require
different revisions.

Both matrices are CLI probe evidence. A passing host-compatibility fixture
shows that the reviewed request shape works against this server; it does not
show that a managed Codex process ran. Only lifecycle observations recorded by
an actual process with source `managed_host` can satisfy managed-host
operational checks.

## Authoritative Lifecycle Recording

After resolving the Agent Connection, the process creates a Registry runtime
session before validating thread metadata or reading a protocol message. The
row identifies this Volicord-generated process launch, its Connection,
`managed_host` or `cli_preflight` source, current connection integration
revision, process ID, and process-start time. A CLI preflight row never
satisfies a managed-host operational check.

As soon as it parses bounded `clientInfo.name`, `clientInfo.version`, and
`protocolVersion`, the adapter durably records them as the attempted client and
requested revision even if later initialize validation fails. On successful
initialize it records completion and the server-selected profile revision
before returning that revision. The selected value becomes the negotiated
revision only when a valid `notifications/initialized` fully completes the
handshake. The adapter records each actual `tools/list` response before
returning it; the discovery fact says whether that generated response contained
every tool required by the current Connection mode. Duplicate initialized
notifications are idempotent after the first valid observation and cannot
change the negotiated revision.

Successful `volicord.status`, `volicord.get_operation_result`,
`volicord.check_close`, and `volicord.list_projects` completions update the
safe/read-only milestone before the tool result is emitted. Observable fatal
transport or protocol failure creates one bounded shared `DiagnosticFinding`
and atomically links its finding ID as the runtime session's terminal finding.
EOF-driven graceful close remains a distinct mutually exclusive terminal fact.
An authoritative Store failure withholds the corresponding protocol success.
Bounded writes to `diagnostics.sqlite` remain best effort and are never
consulted for these facts.

Connection verification starts a separate `cli_preflight` process and calls
`volicord.list_projects` as its designated safe read-only round trip. That
process validates the server surface, but its lifecycle facts cannot satisfy a
`managed_host` operational check or authorize a Connection call. Successful
CLI verification does not fabricate managed `host_session`, `tools/list`, or
tool-round-trip observations.

Runtime rows are durable process-launch observations, not liveness records.
A process that exits before linking a terminal finding or recording graceful close may
leave a row that appears open. Such a row remains historical evidence and is
never selected to correlate a later Guard event. Concurrent managed processes
may coexist and bind different host sessions.

The requested revision is received from the client, the selected revision is
returned or selected by the server, and the negotiated revision is that
selected value only after the handshake fully completes. Only the negotiated protocol version is
authoritative runtime-session protocol data. `clientInfo` name/version and an
observed host executable version are diagnostic fields; they accept bounded
future values. Compatibility is determined from the current managed
configuration and the initialization, tool-list, required-tool, safe-call, and
Guard behavior observed for the current revision. These records are cooperative
and do not establish client, host, actor, or human identity.

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

One canonical tool registry owns each public tool name, description, compact
input schema, compact output schema, annotations, and any currently populated
optional presentation or metadata values. `tools/list` projects that model
through the selected session profile; Volicord does not maintain a separate
tool registry or server implementation for each revision. Connection mode and
storage capability may withhold tools as listed above, but protocol revision
does not rename or substitute the tools that remain visible.

| Selected profile | Emitted fields for each current Volicord tool |
|---|---|
| `2024-10-07`, `2024-11-05` | `name`, `description`, `inputSchema` |
| `2025-03-26` | `name`, `description`, `inputSchema`, `annotations` |
| `2025-06-18`, `2025-11-25` | `name`, `description`, `inputSchema`, `outputSchema`, `annotations` |

The later profiles also permit `title` and `_meta`, and `2025-11-25` permits
`execution` and `icons`. Those fields are emitted only when the canonical
registry owns populated values; the current Volicord registry owns none, so
they are absent rather than fabricated.

## Public Argument Projection

`tools/call` uses string `params.name` and optional object `params.arguments`.
Public schemas hide Core envelopes, internal connection/project IDs, protocol
metadata, idempotency fields, actor source, operation category, and verification
basis. Hidden fields are rejected before Core. Compact discovery schemas never
relax the complete owner-defined request validation.

<a id="mutation-authority-receipt-projection"></a>
## Response Wrapping

Every successful Core call first produces one canonical public method-result
object. The selected profile then chooses its MCP carrier without changing that
object or its Core meaning:

| Selected profile | Authoritative result carrier |
|---|---|
| `2024-10-07` | `toolResult`; the revision has no standardized `isError` field |
| `2024-11-05`, `2025-03-26` | JSON text in the first `content` item, with `isError` |
| `2025-06-18`, `2025-11-25` | `structuredContent`, with compatibility `content` and `isError` |

For the text-only profiles, the first text item is the authoritative JSON
object; later text items are compatibility renderings and never another
authority source. For structured-content profiles, the object is validated
against the exact compact `outputSchema` advertised for that tool. A pre-Core
adapter rejection uses the same revision carrier and retains its structured
error code and retry/side-effect flags even where `isError` is not available.

Mutations retain the selected `summary`, `workflow`, or `full` public
projection with one fresh `AuthorityReceipt`, exact effect identity, replay
facts, and bounded recovery information. Response-size accounting and compact
recovery use the actual selected-profile carrier. They do not change retry
rules, Core effects, or the authoritative public result body.

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
