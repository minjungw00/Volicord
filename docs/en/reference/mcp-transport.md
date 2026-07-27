# MCP transport reference

This document owns the local MCP process boundary: managed stdio startup,
strict binding, JSON-RPC lifecycle, tool discovery, public argument projection,
response wrapping, and shutdown. Core methods, Codex configuration, connection
verification, and storage effects remain with their focused owners.

<a id="surface-stability"></a>
## Surface Stability

Labels use [Documentation Policy](../maintain/documentation-policy.md#surface-stability-labels).

| Surface | Stability |
|---|---|
| `volicord mcp serve`, initialization, `tools/list`, `tools/call`, and response wrapping | `stable` |
| `volicord mcp preflight` read-only inspection and output contract | `stable` |
| Authoritative runtime-session lifecycle milestones | `stable` |
| The five production MCP revisions and closed 16-tool catalog, including mode/storage visibility subsets | `stable` |
| Current beta MCP transport surface | `beta` — none; every current transport surface is classified by another row |
| Hidden managed launcher, launch leases, and generated configuration details | `internal` |
| Host executable version, MCP client name/version, and best-effort protocol metrics | `diagnostic` |

## Wire schema ownership

`volicord-mcp-protocol` owns the closed protocol-profile registry and its
semantic capabilities. `volicord-mcp-wire` owns the exact MCP request, result,
error, structured-content, annotation, JSON-RPC envelope, and generated wire
schema vocabulary. This includes `McpOperationalErrorCode`,
`McpOperationalOperation`, `McpOperationalResource`,
`McpOperationalFailure`, `McpReadOnlyToolStructuredContent`,
`McpToolDefinitionEnvelope`, and `McpToolResultEnvelope`.

`volicord-types` owns only adapter-neutral method facts, domain values, and
public method schemas. `volicord-mcp` consumes those neutral outcomes and maps
them into the wire-owned values. Core, Store, the UserAction service, the
administrative CLI, and UserAction presentation have no dependency on
`volicord-mcp-wire`. Profile-dependent carriers and optional fields are chosen
only from the selected profile's semantic capabilities.

## Process Model

The hidden `volicord _host-launch codex` entry is started by managed Codex
configuration and transitions in the same process to the stdio adapter after
lease consumption. The public `volicord mcp serve` entry is the manual stdio
surface. Both exchange line-delimited JSON-RPC through stdin and stdout and
open no TCP, HTTP, Unix-domain socket, or other network listener. Canonical
command syntax is generated in the
[Administrative CLI Reference](admin-cli.md).

The `--connection` process form accepts an explicit `--project` for current
manual or preflight selection. The canonical personal Codex entry uses this
form without `--project`; its current project associations remain Store-owned
Connection Project memberships. Repository discovery is only for the canonical
shared Codex entry and resolves the Connection and project from the exact
Runtime Home and canonical Git work tree. It does not infer a connection from
cwd alone, scan nearby repositories, or accept another host selector.
`mcp preflight` validates the canonical managed entry and reads the selected
Registry, Connection, projects, protocol profiles, tool schemas, and host
contract without entering the stdio loop. It performs no writeability probe,
creates no runtime session or finding, and succeeds against readable read-only
SQLite databases and filesystems. Its JSON projection declares
`side_effects: []`, evidence class `read_only_preflight`, and writeability
`not_checked` with `requires_active_verification`. Connection verification
decodes that result into immutable `McpPreflightEvidence`; later active probes
cannot replace any preflight field. The connection check projects active
results only through the sibling `last_active_verification` member.

## Environment And Startup

`VOLICORD_HOME` selects the Runtime Home according to
[Runtime Boundaries](runtime-boundaries.md). The canonical managed launch
contract stores the selected absolute value in personal configuration and
forwards the parent value only in shared configuration, which embeds no
machine-local path. Exact generated shapes and strict parsing belong to
[Agent Connection](agent-connection.md#managed-mcp-launch-contract).

Connection preflight and the CLI stdio handshake derive their public process
launches from that same binding contract. Materialization preserves ordinary
inherited process variables, applies only explicit process configuration, and
resolves every forwarded name from
explicit verification input. Personal verification therefore
uses the static Runtime Home already in its contract. Shared verification uses
the operation-selected Runtime Home as the forwarded `VOLICORD_HOME` while the
repository-visible configuration remains portable. Shared repository discovery
runs from the canonical Product Repository root; personal verification uses
its bound identifiers without a repository-discovery working-directory
dependency.

Before reading MCP requests, the hidden launcher resolves the exact registered
Connection from the canonical managed configuration and
validates that it is enabled, its selected projects are current members, the
Runtime Home and Product Repository are separated, the `StorageManifest` is
current, and required storage is readable. It creates a bounded one-time launch
lease only after the strict current entry, integration revision, and managed
fingerprint match.
Corrupt records, ambiguous selection, and unavailable storage use the
[Failure Model](failure-model.md).

MCP bootstrap consumes that lease exactly once and creates the `managed_host`
Registry runtime session in the same Store transaction. A replayed, expired,
cancelled, Connection-mismatched, revision-mismatched, or fingerprint-mismatched
lease creates no runtime. Public `mcp serve` always creates `manual_cli`; no
public flag or environment variable can select `managed_host`. A dedicated
integration probe uses `integration_probe`. Read-only preflight creates no
runtime.
Executable path, host version, and client version remain diagnostics;
managed-call authorization is established from the current session and project
bindings described below.

## MCP Wire Behavior

Each non-empty stdin line is one complete UTF-8 JSON-RPC 2.0 request. Malformed
JSON returns `-32700`; invalid requests return `-32600`; unknown methods return
`-32601`; invalid parameters return `-32602`; and internal protocol failures
return `-32603`. Responses preserve the request `id`.

The `initialize` request and the `notifications/initialized` notification are
each standalone top-level messages. `initialize` precedes `tools/list` and
`tools/call`. Calls before initialize, repeated initialize, and invalid
lifecycle operations fail before Core. A selected initialization profile does
not become negotiated until the standalone `notifications/initialized` step
completes. A top-level batch is rejected until that step makes the session
operation-ready.

Observable failures use shared structured findings rather than classifying
human-readable error text. The current MCP code families are:

| Family | Stable codes |
|---|---|
| JSON-RPC and framing | `mcp.json_rpc.parse_error`, `mcp.json_rpc.invalid_request`, `mcp.json_rpc.invalid_id`, `mcp.json_rpc.unknown_method`, `mcp.json_rpc.malformed_response`, `mcp.json_rpc.framing_failure`, `mcp.json_rpc.message_size_exceeded`, `mcp.json_rpc.error_response` |
| Lifecycle | `mcp.lifecycle.initialize_required`, `mcp.lifecycle.duplicate_initialize`, `mcp.lifecycle.initialization_batch_forbidden`, `mcp.lifecycle.initialized_notification_missing`, `mcp.lifecycle.initialized_notification_invalid`, `mcp.lifecycle.operation_before_ready`, `mcp.lifecycle.invalid_shutdown_sequence` |
| Revision and capabilities | `mcp.protocol.malformed_version`, `mcp.protocol.unsupported_version`, `mcp.protocol.capability_shape_invalid`, `mcp.protocol.schema_projection_failed` |
| Tool discovery | `mcp.tools.protocol_error`, `mcp.tools.schema_failure`, `mcp.tools.required_missing`, `mcp.tools.definition_projection_invalid` |
| Tool call | `mcp.tool_call.unknown_tool`, `mcp.tool_call.invalid_arguments`, `mcp.tool_call.protocol_error`, `mcp.tool_call.output_schema_failed`, `mcp.tool_call.response_budget_failed`, `mcp.tool_call.core_execution_failed`, `mcp.tool_call.adapter_execution_failed`, `mcp.tool_call.safe_read_only_failed`, `mcp.tool_call.session_correlation_invalid` |

Negotiation findings keep bounded `requested_revision`, `selected_revision`,
`negotiated_revision`, `production_supported_revisions`, attempted
`clientInfo` name/version, JSON-RPC error code, safe error data, and runtime
session ID as separate facts. Requested, selected, and negotiated revisions are
never substituted for one another. Facts exclude full requests, tool
arguments, environments, and unrestricted process output.

## Protocol Revision Negotiation

The production-supported initialization revisions are exactly:

- `2024-10-07`
- `2024-11-05`
- `2025-03-26`
- `2025-06-18`
- `2025-11-25`

A revision is production-supported only when the specification manifest marks
it released and not pre-release-only, pins its schema artifacts, and provides a
matching production protocol profile. The offline specification gate requires
exact revision-set parity between released manifest entries marked
`production_supported=true` and production profiles in `ProtocolRegistry`.
Executable runtime conformance is established independently by running the
registry-driven protocol conformance test for every production profile; it is
not represented as manifest metadata or an upstream or third-party MCP
certification. A tracked pre-release revision does not become
production-supported merely by being pinned.

There is no separate conformance-coverage boolean or conformance revision
array. The executable coverage set is the direct
`ProtocolRegistry::production().oldest_to_newest()` iteration. `xtask` reads
that registry through `volicord-mcp-protocol` for manifest parity and does not
depend on the `volicord-mcp` runtime adapter, Core, Store, or platform crates.

The request's string `protocolVersion` is the requested revision. An exact
member of this closed set selects the same profile and the initialize result
returns the same revision. Every other identifier is rejected with `-32602`
and `mcp.protocol.unsupported_version`; the server does not substitute its
preferred, oldest, newest, or default profile. Selection uses exact registry
membership, not lexical comparison, date or numeric parsing, range or prefix
tests, or package-version conditions. The supported set is not
user-configurable.

Missing or non-string `protocolVersion`, non-object `capabilities`, and
malformed `clientInfo` remain `-32602` invalid parameters with bounded error
data. A `protocolVersion` outside the production revision set is rejected as
unsupported.

After valid parameter decoding, the active MCP connection owns one typed,
session-scoped selection. It retains the exact requested string, selected
profile, client capabilities, bounded attempted client name/version, and
initialized-notification completion fact.
The selected profile generates the initialize response `protocolVersion` and
capabilities and governs later lifecycle validation. The profile is selected
after a successful initialize request, but its revision is negotiated only
after the valid initialized notification completes the handshake.

### Registry-owned semantic capabilities

`ProtocolRegistry` is the single owner of the mapping from each exact
production revision to adapter behavior. Each profile carries one complete
semantic capability bundle. Tool and initialize projection receive that
bundle, not a revision string, and cannot select behavior by revision order.
Pinned schema field sets remain parity evidence; adapters do not use raw field
sets as a second behavior map.

| Profile | Result carrier | `structuredContent` | `isError` | Result `_meta` |
|---|---|---|---|---|
| `2024-10-07` | direct `toolResult` | no | no | supported |
| `2024-11-05` | authoritative JSON text in the first `content` item | no | supported | supported |
| `2025-03-26` | authoritative JSON text in the first `content` item | no | supported | supported |
| `2025-06-18` | `structuredContent` plus compatibility `content` | supported | supported | supported |
| `2025-11-25` | `structuredContent` plus compatibility `content` | supported | supported | supported |

| Profile | Tool-definition capabilities | Initialize result | Accepted client capabilities |
|---|---|---|---|
| `2024-10-07` | base `name`, `description`, `inputSchema` | `_meta`, `protocolVersion`, `capabilities`, `serverInfo`; no `instructions` | open object; known `experimental`, `roots`, `sampling` |
| `2024-11-05` | base fields | base initialize fields plus `instructions` | open object; known `experimental`, `roots`, `sampling` |
| `2025-03-26` | base fields plus `annotations` | base initialize fields plus `instructions` | open object; known `experimental`, `roots`, `sampling` |
| `2025-06-18` | base fields plus `annotations`, `outputSchema`; optional populated `title` and definition `_meta` are supported | base initialize fields plus `instructions` | open object; known `elicitation`, `experimental`, `roots`, `sampling` |
| `2025-11-25` | base fields plus `annotations`, `outputSchema`; optional populated `title` and definition `_meta` are supported | base initialize fields plus `instructions` | open object; known `elicitation`, `experimental`, `roots`, `sampling`, `tasks` |

Unknown additive client capability fields remain accepted because every
current profile declares an open object. Every profile also declares the same
committed-result recovery capability: preserve fresh authority first, then a
compact method result, then stable effect facts; never retry the committed
mutation. The adapter's fixed compact, full, and compatibility-text byte
budgets are applied after projection through the selected result carrier.

Volicord advertises only the `tools` server capability, and only when that
field is permitted by the selected profile; all five supported profiles permit
it. After initialization is complete, the profile stored in session state
controls operation-phase JSON-RPC batching exactly as follows:

| Selected profile | Operation-phase batch requests and responses |
|---|---|
| `2024-10-07` | disallowed |
| `2024-11-05` | disallowed |
| `2025-03-26` | allowed |
| `2025-06-18` | disallowed |
| `2025-11-25` | disallowed |

These are reviewed profile facts, not chronology inferred from revision names
or batch contents. Initialization batching is prohibited for every supported
revision: a batch containing `initialize` or `notifications/initialized` is
rejected before any entry is processed. Any other batch received before the
session is operation-ready is also rejected without changing the selected or
negotiated revision or recording a tool observation. Once ready, `2025-03-26`
admits operation batches from its already-selected session profile; every other
production profile rejects them. An admitted batch is processed sequentially
in input order. Notifications have no response entry, and a batch containing
only notifications produces no response.

## CLI Conformance And Host Compatibility Probes

The executable protocol conformance test and the CLI server-conformance probe
both iterate production profiles directly from `ProtocolRegistry` in its
deterministic order. Adding a production profile therefore adds it to each
generic matrix without another revision declaration. The focused executable
case covers standalone `initialize`, `notifications/initialized`, `tools/list`,
pinned-schema and required-tool validation, the exact designated round-trip
tool, revision-specific definition and result projection,
initialization-batch rejection, profile-selected operation-phase batching or
rejection, invalid lifecycle behavior, and EOF/shutdown. Initialization
batching is rejected for every production profile.

Connection verification gives each production profile a separate `mcp serve`
process and exact request in a fresh disposable Runtime Home and Product
Repository. The probe completes `initialize`,
`notifications/initialized`, `tools/list`, validation against that revision's
pinned schema, current-mode required-tool validation, exactly one
call to the tool selected by `ToolVerificationRole::ManagedHostRoundTrip`, and
graceful EOF/shutdown. The current role owner is exactly
`volicord.list_projects`. The probe records the
requested and negotiated revisions, returned tools, completed stages, and a
typed failure per revision. The aggregate server check passes only if every
production profile passes; one failed profile does not prevent the remaining
profiles from being probed.

Host compatibility uses a separately pinned semantic fixture rather than a
projection of the protocol registry or a substitute for the complete revision
matrix. The current `codex` fixture uses the
reviewed Codex initialize request shape with `clientInfo.name` set to
`codex-mcp-client`, the `Codex` title, an empty current capability object, and
the independently pinned revision `2025-06-18`. Its one tool call carries valid
`codex-mcp-turn-metadata` session/thread/turn metadata. That fixture ID names
the reviewed wire contract and is not a Codex package-version identity. It executes
`tools/list` and the tool selected by
`ToolVerificationRole::ManagedHostRoundTrip`, currently
`volicord.list_projects`, and it never derives its requested revision from the
server's preferred or newest profile.

Both matrices are CLI probe evidence. Their disposable `manual_cli` or
`integration_probe` runtime sources remain excluded from managed checks and
are removed with the verification fixture. A passing host-compatibility fixture
shows that the reviewed request shape works against this server; it does not
show that a managed Codex process ran. Only lifecycle observations from a
runtime created by successful launch-lease consumption with source
`managed_host` can satisfy managed-host operational checks.

The active connection-verification evidence records each matrix result under
`protocol_conformance` or `host_compatibility`, next to the separate Registry
and project write-probe results. It carries its own `observed_at`,
`source=connection_verify`, and closed side-effect list. No conformance process
uses the selected live Runtime Home: every protocol and host fixture runs in
the fresh disposable state above. Only the explicitly bounded SQLite
writeability probes touch selected live databases, and those probes use a
minimal immediate transaction whose probe table is created and dropped before
an unconditional rollback.

## Semantic Codex Host Contracts

Managed Codex wire input is decoded through an explicitly selected host
contract profile. The `CodexMcpTurnMetadata` marker selects
`codex-mcp-turn-metadata`, which owns `tools/call` `_meta` and requires the
native session, thread, and turn plus equality between the top-level
`threadId` and nested `x-codex-turn-metadata.thread_id`. The distinct
`CodexCommandHooks` marker selects `codex-command-hooks`, which separately owns
command-hook envelopes. Its `UserPromptSubmit` correlation is session plus
turn, while `PreToolUse` and `PostToolUse` correlation is session, turn,
tool-use ID, and canonical tool name. Command-hook correlation has no thread
coordinate.

The two profiles are not inferred from payload shape and their typed
correlations are not interchangeable. Both accept unknown additive fields,
retain only bounded contract-owned presentation and tool values, and return
bounded typed failures without retaining the complete input. Managed MCP also
requires the parsed session and thread to match the registered managed runtime
binding. `volicord-host-contract` owns both markers, their deterministic
profile identities and digests, and the source-specific correlation types.
The reviewed fixtures and coverage manifest under
`tests/conformance/codex-host/`, with parser and checksum assertions in
`crates/volicord-host-contract/tests/host_contracts.rs`, are pinned contract
inputs rather than protocol-revision or package-version claims.

MCP registration supplies an explicit `McpServerKey`; `AgentToolId` supplies
the complete `McpRawToolName`. `McpToolIdentity` preserves both coordinates,
and `CodexMcpCallableNames` projects them under
`codex-mcp-callable-names` to a validated `HostCallableIdentity`. The
projection normalizes the server key and complete raw name independently,
joins them with the Codex separator, applies the 64-byte callable bound, and
uses the current deterministic 12-hex SHA-1 source-identity suffix when the
source exceeds that bound. This suffix is a name-fitting rule, not an integrity
claim. The catalog rejects distinct sources that still project to one callable. It never
extracts server identity from a dotted raw tool name. `McpToolCatalog` is also
the only reverse-resolution source, so underscores and punctuation are never
used to guess namespace boundaries. The adapter selects this semantic
contract directly; an observed Codex package version does not control
callable projection.

The same semantic owner defines `HostHookMatcherStrategy`. For current Codex
tool hooks, the reviewed strategy is a union of the native Guard host tools
and server-qualified MCP routing. It uses the registered `McpServerKey`
namespace where the bounded callable representation preserves it, and
otherwise derives exact callable tokens from the same canonical catalog.
Matcher JSON is generated only from this typed value, and strict configuration
validation parses it back to the same value. Routing is not semantic tool acceptance:
`McpToolCatalog` still performs exact callable resolution in the wrapper, and
the canonical catalog assigns the resolved `AgentToolId` exactly one
integration-verification role: probe target, workflow control, or unrelated
known tool. Catalog construction rejects contradictory role metadata. Only the
probe-target role proceeds to probe-specific coordinate checks; begin, status,
and all other known roles are nonterminal routed trace. No numeric host-version
branch changes the strategy.

## Authoritative Lifecycle Recording

Each bounded MCP lifecycle observation or tool operation that can persist
Runtime Home state acquires `SharedWriter` for that operation only. The lease
is not held for the server process lifetime and remains live through Core,
Store, milestone, receipt, and terminal-finding effects. This covers
managed-launch consumption, runtime/session creation and updates, initialize
and initialized observations, `tools/list`, designated verification tools,
integration verification, public method mutations, operational evidence, and
terminal failure linkage.

If setup owns `ExclusiveSetup`, the message or call returns the typed
`runtime_home.mutation.setup_in_progress` lifecycle/tool error before any
partial effect. It does not emit protocol success, advise raw stdio fallback,
or silently claim that an observation was stored. A later message or call may
retry after setup releases the lease.

After `mcp serve` or the hidden launcher resolves the Agent Connection, the process creates a Registry runtime
session before validating thread metadata or reading a protocol message. The
row identifies this Volicord-generated process launch, its Connection, one of
the exact `managed_host`, `manual_cli`, `cli_preflight`, or `integration_probe`
sources, current connection integration revision, process ID, and process-start
time. Only atomic launch-lease consumption creates `managed_host`; every other
source is excluded from managed-host operational checks and authorization.

As soon as it parses bounded `clientInfo.name`, `clientInfo.version`, and
`protocolVersion`, the adapter durably records them as the attempted client and
requested revision even if later initialize validation fails. On successful
initialize it records completion and the server-selected profile revision
before returning that revision. The selected value becomes the negotiated
revision only when a valid `notifications/initialized` fully completes the
handshake. The adapter records each actual `tools/list` response before
returning it, including the canonical sorted exact returned tool identities.
The discovery fact says whether that generated response contained every tool
required by the current Connection mode; successful validation records its own
`required_tools_validated_at` milestone. Duplicate initialized notifications
are idempotent after the first valid observation and cannot change the
negotiated revision.

Only a successful `tools/call` for the exact tool bound to
`ToolVerificationRole::ManagedHostRoundTrip` can record managed-host
round-trip evidence. The role's compile-time binding is
`AgentToolId::LIST_PROJECTS`, whose wire-name projection is
`volicord.list_projects`. The call must follow same-session required-tool
validation, carry valid current managed Codex session/thread/turn correlation, belong
to the current enabled `managed_host` runtime and Connection revision, and
complete without a JSON-RPC or tool error. Store then atomically records the
exact `verification_tool_name` and `verification_tool_observed_at` pair before
the tool result is emitted. Successful `volicord.status`,
`volicord.get_operation_result`, and `volicord.check_close` calls do not update
that pair. Failed or rejected designated calls also leave it absent. Observable fatal
transport or protocol failure creates one bounded shared `DiagnosticFinding`
and atomically links its finding ID as the runtime session's terminal finding.
EOF-driven graceful close remains a distinct mutually exclusive terminal fact.
An authoritative Store failure withholds the corresponding protocol success.
Bounded writes to `diagnostics.sqlite` remain best effort and are never
consulted for these facts.

Failures before the Registry can be opened emit one bounded
`VOLICORD_DIAGNOSTIC_V1` envelope on stderr. After a Registry runtime session
exists, findings are persisted with its exact Connection, integration
revision, and runtime-session coordinates; terminal failures are linked by
finding ID rather than stored as a second free-form failure object.

A typed platform Store failure is represented in MCP bootstrap and persisted
terminal findings by its canonical `platform.*` code, `platform` domain, and
`platform_observation` stage. Its recommended action is selected from the
typed platform class. Bounded human-readable detail remains separate from the
machine-readable finding identity.

Connection verification performs read-only preflight against the selected
Runtime Home, then starts manual stdio probe processes only in a disposable
per-command fixture. It calls the same canonical role owner, currently
`volicord.list_projects`, for its safe read-only self-test round trip. The
fixture processes validate the server surface, but their lifecycle facts
cannot satisfy a `managed_host` operational check or authorize a Connection
call. They create no session or finding in the selected user Runtime Home.
Successful CLI verification does not fabricate managed `host_session`,
`tools/list`, or tool-round-trip observations.

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
Guard behavior observed in one current-revision `managed_host` runtime.
Verification selects the newest managed runtime as `latest_managed_attempt` for current
health and independently selects the newest runtime with that complete same-
session chain as `latest_managed_capability_proof`. It never combines milestones across
runtimes. The actual protocol peer is the selected runtime's `clientInfo`; the
separately probed executable path and `codex --version` are installation and
manual-invocation aids, not protocol-peer authority. A peer/PATH version
mismatch may produce warning evidence but does not itself invalidate a complete
managed session. These records are cooperative and do not establish client,
host, actor, or human identity.

## Per-Call Session Authorization

A valid Guard observation establishes normalized project `host_sessions` and
`host_turns` rows. Tool phases also establish `host_tool_invocations`. Guard
does not create `managed_mcp_sessions`, does not select a runtime from open
process rows, and does not synthesize a thread. Before an actual project is
selected, MCP runtime state retains only the exact `CodexMcpCorrelation`; it
does not derive or search for a Connection-only internal session coordinate.
After project selection, Store resolves the current project integration
revision and derives the project Agent Session ID from the Connection, that
exact revision, and the native session.

For the first actual managed `tools/call`, Store validates the current managed
runtime without mutation, establishes the normalized host session and turn,
and creates or validates the MCP-only `managed_mcp_sessions` anchor with the
exact Connection, native session, thread, project revision, current Guard
ownership, and latest turn. Only after that project transaction commits does
Store revalidate current owner facts and reserve the exact Registry
runtime/project/revision/host-session binding. A final project transaction
attaches that runtime to the same anchor. A deterministic project ownership
conflict therefore creates no Registry reservation. An unbound MCP anchor is
not authority, and a Registry reservation without the matching project
attachment is not authority. If the final project write is interrupted, an
identical call under unchanged owner state reuses the reservation and finishes
the attachment. CLI preflight never performs this binding.

Project Agent Session validation therefore precedes Registry runtime
reservation. Authorization requires both the completed project attachment and
the exact completed Registry binding.

Before constructing Core invocation context for a project tool, the adapter
validates the authoritative current Registry runtime session, the exact
`mcp_runtime_project_session_bindings` row, and the project
`managed_mcp_sessions` row joined to its `host_sessions` owner. The Connection
must exist and be enabled; the project must exist and
remain a Connection Project; the runtime session must be a current
`managed_host` session owned by that Connection; and the project session must
be non-null-bound to the same runtime, Connection, project, and host session.
The Registry binding revision and project row revision must be identical, and
both integration revisions must match their current Connection and project
inputs. The current Connection mode must allow the requested operation
category. Hook-only host rows retain Guard history but cannot authorize a tool
call. Every real mode transition advances the Store-owned
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
| `workflow`, writable | `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.get_operation_result`, `volicord.prepare_write`, `volicord.prepare_evidence_capture`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_action`, `volicord.reconcile_changes`, `volicord.check_close`, `volicord.close_task`, `volicord.list_projects`, `volicord.begin_integration_verification`, `volicord.guard_probe`, `volicord.get_integration_verification` |
| `workflow`, readable only | `volicord.status`, `volicord.get_operation_result`, `volicord.request_user_action` (resume only), `volicord.check_close`, `volicord.list_projects`, `volicord.begin_integration_verification`, `volicord.guard_probe`, `volicord.get_integration_verification` |
| `read_only`, readable | `volicord.status`, `volicord.get_operation_result`, `volicord.check_close`, `volicord.list_projects`, `volicord.begin_integration_verification`, `volicord.guard_probe`, `volicord.get_integration_verification` |
| no readable allowed project | `volicord.list_projects` |

Task state and previous calls do not dynamically add tools. A withheld mutation
fails without Core effects. `volicord.resolve_user_action` is a public Core API
method but is never an MCP tool.

`AgentToolId` is the canonical typed identity and catalog for every Agent
Connection MCP tool. Core-owned identities reuse `MethodName`; adapter
utilities and Connection-integration tools belong to the same closed 16-tool
catalog. Each identity owns its stable MCP wire-name projection, category,
Connection-mode availability, Core-method, adapter-utility, or
Connection-integration ownership, idempotence, and optional operational
verification role.

The canonical tool registry keys each definition by `AgentToolId` and supplies
its description, compact input schema, compact output schema, annotations, and
any populated optional presentation or metadata values. `tools/list` emits the
identity's wire-name projection through the selected session profile; Volicord
does not maintain a separate tool registry or server implementation for each
revision. Connection mode and storage capability may withhold tools as listed
above, but protocol revision does not rename or substitute the tools that
remain visible.

`ToolVerificationRole::ManagedHostRoundTrip` is bound at compile time to
`AgentToolId::LIST_PROJECTS`. The MCP runtime, administrative CLI, Store
observation, and diagnostic comparison use that identity and project
`volicord.list_projects` only at wire or persisted-name boundaries.

| Selected profile | Emitted fields for each current Volicord tool |
|---|---|
| `2024-10-07`, `2024-11-05` | `name`, `description`, `inputSchema` |
| `2025-03-26` | `name`, `description`, `inputSchema`, `annotations` |
| `2025-06-18`, `2025-11-25` | `name`, `description`, `inputSchema`, `outputSchema`, `annotations` |

The two structured-content profiles support optional `title` and definition
`_meta`. Those fields are emitted only when the canonical registry owns
populated values; the current Volicord registry owns none, so they are absent
rather than fabricated.

## Public Argument Projection

`tools/call` uses string `params.name` and optional object `params.arguments`.
Public schemas hide Core envelopes, internal connection/project IDs, protocol
metadata, idempotency fields, actor source, operation category, and verification
basis. Hidden fields are rejected before Core. Compact discovery schemas never
relax the complete owner-defined request validation.

<a id="user-action-wire-projection"></a>
### UserAction wire projection

```yaml
McpRequestUserActionResponse:
  agent_workflow_result: RequestUserActionResponse
  agent_workflow_result_replayed: boolean
  current_projection_state_version: integer
  current_projection_observed_at: string
  current_status: pending | resolved | stale | superseded | expired
  user_channel_resolution_ref: StateRecordRef | null
  user_channel_resolution: McpUserActionResolution | null
  derived_refs: StateRecordRef[]

McpRequestUserActionCompactResult:
  effect: McpMutationEffectSummary
  agent_workflow_result_replayed: boolean
  user_action_request_summary: AgentSafeUserActionRequestSummary
  current_projection_state_version: integer
  current_projection_observed_at: string
  user_action_resolution_ref: StateRecordRef | null
  status: pending | resolved | stale | superseded | expired
  resolution_summary: McpUserActionResolutionSummary | null
  derived_refs: StateRecordRef[]

McpUserActionResolution:
  user_action_resolution_id: string
  user_action_request_id: string
  action_kind: string
  channel_kind: cli
  resolved_at: string
  resolution_summary: McpUserActionResolutionSummary

McpUserActionResolutionSummary:
  # choice variant
  resolution_type: choice
  selected_option_id: string
  selected_option_label: string
  machine_action: accept | reject | defer
  resolution_outcome: accepted | rejected | deferred

  # evidence-observation variant
  resolution_type: evidence_observation
  target: EvidenceTarget
  artifact_refs: ArtifactRef[]
  relevance_status: supported | contradicted
```

`agent_workflow_result` is the exact committed
`volicord.request_user_action` response branch addressed by its
`operation_result_ref`. `agent_workflow_result_replayed=false` means the call
created the request; `true` means the same Agent Connection used the explicit
read-only resume operation without a second Agent Workflow mutation. The
historical result contains `AgentSafeUserActionRequestSummary`, not the stored
request, resolution form, or CLI inbox presentation.

An immediate or later channel resolution is a separate nullable projection.
Its ref identifies the immutable resolution without retrieving the private
body. `McpUserActionResolution` omits the user's free-form note, observation
summary, channel submission identity, verification basis, and assurance text.
It never makes the user-only `volicord.resolve_user_action` response
retrievable through the request operation result. `derived_refs` preserves
the exact public refs created by an optional resolution and is empty while the
request remains pending.

`current_status`, the nullable safe resolution and ref, and `derived_refs`
come from one Core/Store read observed at
`current_projection_state_version` and
`current_projection_observed_at`. Resolution and derived refs preserve their
historical `produced_at_state_version`; a later unrelated state advance does
not rewrite them.

<a id="in-chat-integration-verification-schemas"></a>
### In-chat integration-verification schemas

The canonical user-level activation step is
`request_integration_verification`, initiated by the user through
`codex_chat` and executed by the agent. Its nested sequence starts from the
request `Run the Volicord integration verification.` The agent resolves an
exact project through `volicord.list_projects`, then calls
`volicord.begin_integration_verification`. It follows the returned tagged
`workflow`: `awaiting_probe` and `awaiting_observation` carry the exact
canonical `tool` to call, while `complete` and `repair_required` are terminal
and carry no tool. Begin, probe, and status expose this same state-directed
contract. The current Codex semantic host contract uses synchronous observation
with one allowed status read, so the agent uses no shell sleep or poll loop and
does not automatically restart in the same turn. The Guard probe is an
internal nested tool step, not a top-level user action. Only this first-party
sequence can supply current managed MCP and Guard correlation.

If Volicord tools are not exposed, the agent reports the managed MCP connection
unavailable. It does not start raw stdio, hand-author Codex `_meta`, or treat
`resources/list`, resource templates, CLI preflight, or connection status as
proof of managed tool availability. Those surfaces remain read-only
diagnostics. Hook review and project/configuration trust remain user/host
owned. `volicord connection verify` is optional active diagnostics and does not
replace the managed-host sequence.

The sequence combines read-only project discovery with three
Connection-integration tools. All four are MCP adapter operations, not Core
methods or Task workflows. The three integration tools are idempotent within
their exact current managed-host coordinate and have these public shapes:

| Tool | Canonical annotations | Direct effect |
|---|---|---|
| `volicord.list_projects` | `readOnlyHint=true`, `destructiveHint=false`, `idempotentHint=true`, `openWorldHint=false` | Reads the Connection project allowlist; no write. |
| `volicord.begin_integration_verification` | `readOnlyHint=false`, `destructiveHint=false`, `idempotentHint=true`, `openWorldHint=false` | Creates or resumes the one immutable Registry verification attempt for the current semantic coordinate; no Core, Task, or Product Repository effect. |
| `volicord.guard_probe` | `readOnlyHint=false`, `destructiveHint=false`, `idempotentHint=true`, `openWorldHint=false` | First-write-wins acknowledges the exact active run and returns its current shared workflow state; exact replay returns the current terminal or nonterminal state without repeating effects. No Core, Task, project-state, or Product Repository effect. |
| `volicord.get_integration_verification` | `readOnlyHint=false`, `destructiveHint=false`, `idempotentHint=true`, `openWorldHint=false` | Consumes at most the host policy's bounded status read, projects correlated phase status, and may persist terminal typed repair; no Core, Task, project-state, or Product Repository effect. |

These annotations describe the tools themselves. Ordinary compatible Guard
event persistence and its subsequent Registry correlation refresh retain the
separate effects defined by
[Storage Effects](storage-effects.md#connection-integration-verification-effects).
None of the four tools modifies Codex project trust or hook-review state.

```yaml
volicord.begin_integration_verification:
  arguments:
    project_selector?: string
  result:
    verification_id: GuardIntegrationVerificationId
    workflow: IntegrationVerificationWorkflowState
    matched_prompt_event_id: GuardEventId

volicord.guard_probe:
  arguments:
    verification_id: GuardIntegrationVerificationId
  result:
    verification_id: GuardIntegrationVerificationId
    workflow: IntegrationVerificationWorkflowState

volicord.get_integration_verification:
  arguments:
    verification_id: GuardIntegrationVerificationId
  result:
    verification_id: GuardIntegrationVerificationId
    workflow: IntegrationVerificationWorkflowState
    guard_phases:
      prompt_capture: pending | matched
      pre_tool: pending | matched
      post_tool: pending | matched
    matched_prompt_event_id?: GuardEventId
    matched_pre_tool_event_id?: GuardEventId
    matched_post_tool_event_id?: GuardEventId

IntegrationVerificationWorkflowState:
  awaiting_probe:
    kind: awaiting_probe
    tool: volicord.guard_probe
  awaiting_observation:
    kind: awaiting_observation
    tool: volicord.get_integration_verification
    acknowledged_at: UtcTimestamp
    remaining_status_reads: u8
  complete:
    kind: complete
    completed_at: UtcTimestamp
  repair_required:
    kind: repair_required
    reason: hook_event_not_observed | hook_payload_incompatible |
      callable_identity_mismatch | verification_id_mismatch |
      session_mismatch | turn_mismatch | tool_use_mismatch |
      integration_revision_changed | hook_definition_changed |
      policy_changed | observation_deadline_exceeded
    retry_policy: no_automatic_retry | new_turn_required |
      host_reload_required | hook_review_required | repair_required
    finding: { code: string, summary: string }
```

`project_selector` follows ordinary Connection project selection and may be
omitted only when selection is unambiguous. Begin binds the actual current
managed runtime and native session/turn, requires a current compatible
prompt-capture event, and creates or resumes exactly one attempt for
Connection, project, managed runtime session, native host session and turn,
integration revision, Guard Installation, semantic host-contract profile,
hook-definition digest, and policy digest. The coordinate is immutable and
unique even after the attempt becomes terminal; time passing never creates a
new attempt. Begin never accepts `manual_cli`, `cli_preflight`, or
`integration_probe` runtime evidence. One Store-owned projection maps an
unacknowledged attempt to `awaiting_probe`, an acknowledged attempt to
`awaiting_observation`, a successful attempt to `complete`, and a failed
acquisition or owner check to `repair_required` with separate typed reason and
retry policy. The tool references use the canonical `AgentToolId` wire
projection; no result accepts an arbitrary tool string.

The probe acknowledgement is first-write-wins over verification ID,
Connection, managed runtime session, native host session, and native host turn.
The first eligible call records `probe_acknowledged_at`. An exact replay
returns the same `awaiting_observation` state and authoritative acknowledgement
time. Exact replay after completion or repair returns the unchanged terminal
state. A different caller coordinate is rejected without exposing the
acknowledgement, and a terminal attempt without an acknowledgement cannot
acquire one late. Probe does not reactivate a terminal attempt or mutate Core
state, Task state, or Product Repository files. Get follows the semantic
`HookObservationPolicy`: the reviewed current Codex contract is
`Synchronous { allowed_status_reads: 1 }`. Its single read either observes
completion or persists the most precise `repair_required` acquisition or
correlation reason. This behavior is selected by the semantic host contract,
not a Codex version threshold.

A run passes only when the same run session and turn contain a compatible
prompt event followed by `PreToolUse` and `PostToolUse` for the same tool-use
ID, exact generated host tool name `mcp__volicord__volicord_guard_probe`, and exact
`verification_id` input. The Guard Installation, policy hash, integration
revision, hook-contract digest, and managed runtime must remain current, and
the event times must satisfy prompt at or before pre-tool and pre-tool before
post-tool. Historical, unrelated, stale, mismatched, or expired observations
cannot satisfy the attempt. Cleanup expiry only bounds retained records; it
does not drive synchronous observation or permit a same-coordinate retry.
Hook events for begin and status remain `unrelated_routed_tool` trace even when
they carry the current verification ID, so the status tool cannot classify its
own Pre/Post hooks as Guard-probe callable mismatches. An unknown callable in
the routed server namespace is also nonterminal unless it explicitly claims
the exact current verification ID; that claim is a terminal unknown-identity
observation.

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

When Core returns typed operational unavailability without a method result,
MCP emits the MCP-owned `MCP_UNAVAILABLE` structured failure through that same
selected carrier. The object carries the tool name, typed `operation` and
`resource` projections, `retryable`, `reached_core`, and `committed=false`.
Store failures use the existing `store_access` operation and Store resource
identities. A platform-owned Product Repository observation failure uses
`product_path_observation` with `product_repository`; the adapter does not
collapse it into a Store failure or expose a local absolute path.
Profiles with `isError` set it to `true`; profiles without that field retain
the failure identity in the authoritative carrier. The bounded compatibility
message contains no Store source detail. This is not a public
`ToolRejectedResponse`, and `MCP_UNAVAILABLE` is not a public Volicord
`ErrorCode`. Carrier and field selection use only the current semantic
capability registry; an unknown or unsupported profile is rejected.

Mutations retain the selected `summary`, `workflow`, or `full` public
projection with one fresh `AuthorityReceipt`, exact effect identity, replay
facts, and bounded recovery information. Response-size accounting and compact
recovery use the selected profile's semantic carrier capability. When an
ordinary committed result is oversized or cannot be projected after the
effect, recovery preserves the fresh authority receipt first, then the compact
method result, then stable effect facts. The recovery remains success-class,
sets `retryable=false`, withholds a completion claim, requires a current
`volicord.status` read, and retains an immutable operation-result reference
when one exists. It does not change Core effects or reinterpret the
authoritative public method result.

A delivery failure after a committed Core effect preserves operation-result
coordinates. The adapter does not retry a mutation merely because response
serialization or transport failed.

## UserAction Requests

An MCP agent may create a pending request through
`volicord.request_user_action` or use its explicit read-only resume branch. It
may later observe a safe snapshot of current status and the immutable CLI
resolution identity. Its projection never receives
`UserActionResolutionForm`, any `CliUserActionInboxResponse` presentation,
free-form note, submission identity, or credentials.

The adapter never answers or resolves the request and sends no server-initiated
resolution request. The user resolves only with `volicord inbox resolve`.
When MCP supplies a CLI recovery instruction, it obtains the canonical
invocation from the same command-model-backed presentation source used by the
CLI; MCP owns only the safe protocol projection and does not copy CLI syntax or
CLI schema. Guard prompt observations, when present, remain non-authority
observations.

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
