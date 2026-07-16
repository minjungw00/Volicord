# MCP transport reference

This document defines the local `volicord mcp --stdio` process contract and the
local/Docker `volicord serve --transport local-http` process-boundary
contract: process startup, process environment, MCP protocol-version
negotiation, initialization lifecycle, stdio transport framing, local HTTP MCP
request handling, JSON-RPC message validation, Agent-Connection-bound startup
validation, MCP-visible tool discovery, MCP response wrapping, and
shutdown/reconnection behavior.

## Owns / Does Not Own

This document owns:

- `volicord mcp --stdio` process startup and exit behavior
- `volicord serve --transport local-http` startup, local listener, and
  transport-bound authentication and Origin checks
- process configuration used by generated host configuration and user-managed
  generic host configuration
- MCP Runtime Home environment resolution
- MCP protocol-version negotiation and initialization lifecycle
- stdio JSON-RPC framing, message validation, and supported MCP methods
- local HTTP JSON-RPC request handling for the loopback-only serve transport
- server-initiated MCP elicitation at the stdio transport boundary
- local loopback web consent fallback for pending user actions
- MCP startup validation for one internal Agent Connection binding
- MCP `tools/list` and `tools/call` behavior at the transport boundary
- MCP-visible input/output tool-schema projection that hides internal envelopes
  and invocation metadata, including MCP-only omission defaults and input
  examples
- MCP `tools/call` response wrapping and adapter error shape
- process shutdown and reconnection behavior

This document does not own:

- the public Volicord method list or method owner table; see
  [API Methods](api/methods.md)
- public Volicord request and response schemas; see
  [API Schema Core](api/schema-core.md)
- Agent Connection, Connection Projects, project selection meaning, current
  connection context, and actor provenance; see
  [Agent Connection](agent-connection.md)
- administrative Runtime Home setup, connection, project, export, and
  verification commands; see [Administrative CLI](admin-cli.md)
- generated host hook command syntax, hook path safety diagnostics, and
  host-hook wrapper repair; see [Administrative CLI](admin-cli.md#guard-hook-commands)
- storage layout, schema initialization and validation, and storage effects; see the storage owners
  through [Storage](storage.md)

<a id="surface-stability"></a>
## Surface Stability

Labels follow the canonical vocabulary in
[Documentation Policy](../maintain/documentation-policy.md#surface-stability-labels).

| Surface | Stability | Notes |
|---|---|---|
| `volicord mcp --stdio`, stdio JSON-RPC framing, MCP initialization, supported MCP methods, `tools/list`, `tools/call`, and response wrapping | `stable` | Local process and MCP transport contracts for the supported method set. |
| Local HTTP serve transport, Docker host-loopback publishing shape, and local web consent fallback endpoint | `beta` | Supported local surfaces with owner-defined limits; they are not public network API surfaces or full MCP Streamable HTTP compatibility. |
| Process-binding values, generated host configuration details, internal connection/project identities, and hidden invocation metadata | `internal` | These details bind local processes and generated adapters; public MCP tool schemas must hide them unless a focused owner exposes a selector. |
| Startup diagnostics, `/healthz`, structured HTTP error reports, and human-readable transport warnings | `diagnostic` | Diagnostic output must preserve owner-defined codes and disclosures where documented, but prose presentation is not a public API schema. |

## Process Model

`volicord mcp --stdio` is a local MCP stdio process mode of the installed
`volicord` executable. An MCP host starts it as a child process and communicates
through stdin/stdout. It is not an MCP TCP listener, HTTP MCP listener,
Unix-domain socket listener, or other MCP network listener. It may start a
separate loopback-only local web consent listener for pending user actions.
Listener startup alone never selects that channel or authorizes token issuance;
each tool call also requires the exact negotiated model-invisible host
capability owned below. Otherwise the pending action remains available through
the CLI inbox.

`volicord serve --transport local-http` is a separate explicit process mode
for Docker and localhost MCP use. Native local runs start a loopback-only HTTP
listener. Docker runs may use the explicit `--container-listen` mode only with
host-loopback port publishing. The process reuses the same
Agent-Connection-bound MCP adapter logic as stdio where possible. It is not
the default MCP transport, not used by generated local non-Docker host
configuration, and not a general Volicord network service. It is local/Docker
transport only: not a public network API, SaaS endpoint, multi-user server, or
security boundary. No serve option changes this process into a public
host-interface or remote HTTP service.

The current serve transport is an authenticated local MCP-over-HTTP subset. It
accepts JSON-RPC over HTTP `POST /mcp` with MCP session headers and bearer-token
checks, and returns JSON responses. It does not implement server-sent event
streams, HTTP elicitation, or full MCP Streamable HTTP compatibility.
Documentation and startup diagnostics describe only this subset and do not
claim full protocol compatibility.
Startup diagnostics, `/healthz`, and structured HTTP error JSON include a
detective-observation disclosure. They are transport diagnostics, not OS
sandboxing, network isolation, malware defense, full write prevention, actor
attribution proof, correctness proof, test sufficiency proof, or human review
replacement.

Personal/local generated host configuration and user-managed generic host
configuration launch the stdio loop with an internal connection binding. When
the local entry is safely project-bound, it can also carry the selected internal
project binding:

```text
volicord mcp --stdio --connection <connection_id> [--project <project_id>]
```

Generated shared project configuration uses neither binding ID nor a literal
local Runtime Home path. Its command and arguments are one of:

```text
volicord mcp --stdio --discover-repository --host codex
volicord mcp --stdio --discover-repository --host claude-code
```

The shared command must be the PATH-resolved name `volicord`. The same entry
must carry exactly one clone-portable Runtime Home forwarding directive:

- Codex `.codex/config.toml` uses `env_vars = ["VOLICORD_HOME"]` to allow the
  host to forward the same-named value from its launch environment.
- Claude Code `.mcp.json` uses
  `"env": {"VOLICORD_HOME": "${VOLICORD_HOME}"}` to forward that value through
  the host's project-configuration placeholder.

Neither form embeds a Runtime Home path. An absolute command, extra
connection/project arguments, a literal Runtime Home path, managed-launch
markers, secret-like environment keys, and every other environment entry are
invalid. Codex and Claude Code verification treat a missing forwarding
directive or any other deviation from the exact host-specific project
descriptor as configuration drift. Personal/user-scoped Codex bindings retain
the local managed-launch marker contract described below.

The `<connection_id>` process-binding value comes from the stored
`connection_internal_id` created by `volicord init` or
`volicord connection add`.
The optional `<project_id>` process-binding value is a stored
`project_internal_id` already allowed for that connection. Ordinary users
should not need to type either value in text-mode flows.
Repository-discovery mode obtains neither value from the repository; it
resolves both only from the selected local Runtime Home after identifying the
canonical current Git clone.

Baseline command-line behavior:

- `volicord mcp --stdio --connection <connection_id> [--project <project_id>]`
  launches the stdio loop. When `--project` is present, the supplied value must
  be in the connection's allowlist and the stdio process is narrowed to that
  project before serving tool requests.
- `volicord mcp --stdio --discover-repository --host codex|claude-code`
  launches the same stdio loop from a repository-visible portable descriptor.
  This mode requires the exact host selector and rejects `--connection`,
  `--project`, `--check`, extra arguments, and unknown hosts.
- `volicord mcp --check --connection <connection_id>` runs startup validation
  without reading stdin.
- `volicord mcp --check --connection <connection_id> --project <project_id>`
  runs the same startup validation and limits project-detail diagnostics to
  one allowed `project_internal_id` value.
- `-h` and `--help` print usage and environment summary, then exit with code
  `0`.
- `-V` and `--version` print
  `volicord <package-version> (build_id=<build-id>)`, then exit with code `0`.
- No mode, bound `--check` or `--stdio` without `--connection`, discovery
  without `--stdio` and one supported `--host`, unknown options, combined
  command-line modes, missing required option values, and extra positional
  arguments write usage diagnostics to stderr and exit with code `2`.
- Help and version handling happen before Runtime Home or Agent Connection
  lookup.

Successful `--check` output is a diagnostic report. It reports configuration
validity, stdio transport, Runtime Home, `connection_id`, `connection.mode`,
connection enabled state, registry read status, selected project-state read
status, selected project-state write status, startup-observation status,
effective tool mode, `tools/list` schema validation, tool naming style, project
availability, and `verification_scope`.
`registry_read` and `project_state_read` report read capability. The
`project_state_write` diagnostic uses a non-persistent SQLite write-capability
probe and reports `passed`, `readonly`, `failed`, or `skipped`; the probe must
not create Core records, durable diagnostic rows, replay rows, session-watch
records, tool-invocation records, or advance `project_state.state_version`.
`startup_observation` reports whether ordinary stdio startup for one available
project is `recordable`, would be
`best_effort_skipped_if_readonly`, or is `skipped_verification_probe`.
`effective_tool_mode` is `workflow`, `read_only_degraded`, `read_only`, or
`unavailable` and must match the effective `tools/list` behavior for the same
connection and project storage capability. A passed write-capability diagnostic
does not prove OS sandboxing, host identity, security isolation, future write
success, or Product Repository write authority.

Local HTTP serve command-line behavior:

- `volicord serve --transport local-http` is the only supported serve
  transport spelling. Other transport values are usage errors.
- `--listen 127.0.0.1:<port>` or `--listen [::1]:<port>` selects the listener.
  Omission uses `127.0.0.1:8765`.
- Native `--listen` is loopback-only. Binding `0.0.0.0`, `::`, public
  interfaces, container-wide interfaces, or another non-loopback address through
  `--listen` is rejected.
- `--container-listen 0.0.0.0:<port>` or `--container-listen [::]:<port>` is
  the explicit Docker host-loopback publishing mode. It requires a fixed
  nonzero container port and must be paired with a host loopback publish rule
  such as `-p 127.0.0.1:8765:8765`. It is not valid for native local runs and
  is not a public host-interface or remote serving option.
- `--listen` and `--container-listen` are mutually exclusive.
- `--home PATH` selects the Runtime Home for the process. Outside
  repository-discovery mode, omitting `--home` uses the shared `VOLICORD_HOME`
  and then the platform default Runtime Home resolution. Repository-discovery
  mode instead requires a forwarded, nonempty, absolute `VOLICORD_HOME` and
  never substitutes the platform default.
- `--connection <connection_id>` binds the server to one stored Agent
  Connection. Without it, startup succeeds only when exactly one enabled Agent
  Connection with connected projects matches the optional serve project
  allowlist.
- `--project PATH` may be repeated. Each path resolves to a registered
  repository root and narrows the serve process to those project identities.
  The narrowed set must still be inside the selected Agent Connection's
  connected-project allowlist.
- `--token-file PATH` supplies the bearer token from a UTF-8 local file. A
  trailing line ending is not part of the token. Prefer `--token-file` over
  `--token` so the local secret is not carried directly in shell history or the
  serve process arguments.
- `--token TOKEN` supplies the bearer token directly on the command line. It is
  supported for controlled local use but is not the preferred documented form.
- If neither `--token-file` nor `--token` is supplied, Volicord generates a
  process-local token and writes it to stderr during startup. The generated
  token output warns that the token is a local secret and that the endpoint
  must stay on host loopback or the intended Docker host-loopback boundary.
- `--generate-token` explicitly selects the generated-token path and is
  mutually exclusive with `--token-file` and `--token`.
- `--allow-origin ORIGIN` may be repeated to permit browser-capable requests
  from exact Origin values. Without it, requests carrying an `Origin` header are
  rejected and CORS response headers are not emitted.

Exit and stream behavior:

- Normal stdin EOF shutdown flushes stdout and exits with code `0`.
- Successful `--check` writes its report to stdout and exits with code `0`.
- Startup configuration, JSON, or storage failures write diagnostics to stderr
  and exit with code `1`.
- HTTP serve startup configuration, listener, authentication-token, Origin, and
  project-allowlist failures write diagnostics to stderr and exit with code
  `1`.
- HTTP serve startup diagnostics warn that Local HTTP is for host loopback or
  intended Docker host-loopback publishing only. `--container-listen` emits an
  additional warning that it is not for public interfaces or remote hosts.
- Once the stdio loop is running, malformed JSON and unsupported JSON-RPC
  requests return JSON-RPC errors when a response can be written.

HTTP serve request behavior:

- The MCP endpoint path is `/mcp`.
- `POST /mcp` requires `Authorization: Bearer <token>`, `Content-Type:
  application/json`, and an `Accept` header that includes both
  `application/json` and `text/event-stream`.
- Successful `initialize` creates an `Mcp-Session-Id`. Later JSON-RPC requests
  must supply that session ID.
- `DELETE /mcp` deletes a session when the bearer token and session ID are
  valid.
- `GET /mcp` returns `SSE_UNSUPPORTED`; server-sent event streams are not
  implemented by this local HTTP endpoint.
- `GET /healthz` is a minimal local health endpoint, but it still requires the
  same bearer token.
- `GET /consent` and `POST /consent` are local web consent endpoints only when
  local web consent is available. They are not MCP endpoints and do not use the
  MCP bearer token. They are a loopback User Channel capture path that requires
  a valid one-time consent token tied to the project, connection, and pending
  user action.
- There are no unauthenticated arbitrary resource endpoints.
- MCP endpoint browser requests are identified by the presence of an `Origin`
  header. An MCP endpoint request with `Origin` must match an exact
  `--allow-origin` value.
- A top-level `GET /consent` navigation does not require `Origin`. If the
  request supplies `Origin`, it must contain exactly one header field whose
  value is a valid serialized origin that exactly matches the consent
  endpoint's own origin.
- Every `POST /consent` requires exactly one `Origin` header field whose value
  is a valid serialized origin that exactly matches the consent endpoint's own
  origin. A missing, empty, `null`, malformed, comma-combined, repeated, or
  different value fails with HTTP 403 `ORIGIN_NOT_ALLOWED` before form-body
  decoding or validation, token lookup or consumption, or resolution effects.
- CORS preflight is accepted only for the MCP endpoint, only after Origin
  allowlist validation, and only when at least one allowed Origin is configured.
- Local HTTP responses include `Cache-Control: no-store` and
  `X-Content-Type-Options: nosniff`. Local web consent HTML responses also
  include `Referrer-Policy: no-referrer` and a restrictive
  `Content-Security-Policy`. CORS response headers are emitted only for
  explicitly allowed Origins.
- Request headers are limited to 16 KiB and request bodies are limited to
  1 MiB. Larger headers fail with `HTTP_HEADERS_TOO_LARGE`; larger bodies fail
  with `HTTP_BODY_TOO_LARGE`.
- Structured HTTP errors use stable transport error codes for authentication,
  Origin, project allowlist, unsupported transport, unsupported method, and
  unsupported content negotiation failures.

Docker publishing behavior:

- The supported Docker publishing shape maps host loopback to the container
  port, for example `-p 127.0.0.1:8765:8765`.
- In that shape the container process uses `--container-listen 0.0.0.0:8765`
  so Docker can forward the published host-loopback port to the container.
- Publishing the container port on `0.0.0.0`, a public host interface, or a
  remote host is outside the Local HTTP transport contract.
- Docker publishing does not add authentication, authorization, multi-user
  isolation, host trust, or any broader security boundary beyond the
  transport-bound bearer-token and Origin checks above.

Session-watch startup coverage:

- Outside the managed Codex pending-binding path, a project-bound stdio startup
  can create or attach a session-watch baseline before serving tool requests
  whenever bounded snapshot creation is available. The coverage basis is
  `mcp_start`.
- A validated generated Codex descriptor or local managed-marker set establishes
  managed launch provenance only. It creates no diagnostic session,
  session-watch baseline, managed lifecycle row, Core effect, or local-web
  eligibility. The process remains pending until the per-call binding below
  succeeds.
- On the first binding-eligible Codex `tools/call`, the process starts the
  session-watch baseline and materializes the bounded lifecycle facts observed
  for that process: `managed_host_startup`,
  `managed_host_initialize_response`, `managed_host_tools_list` when listing
  occurred, and `managed_host_tool_call`. Coverage starts at this binding
  boundary and is explicitly partial; these startup facts do not claim
  observation of Product Repository changes before the baseline.
- When HTTP serve initialization creates an `Mcp-Session-Id` and the selected
  serve connection/project context has exactly one available allowed project,
  the server creates or attaches the same `mcp_start` baseline before accepting
  later tool requests for that session.
- When a session still has multiple available projects, watcher coverage is
  `pending_project_selection`; no full detective coverage is claimed until a
  tool request names an explicit `project_selector`.
- If a project-selected method request creates the first baseline, the basis is
  `first_project_selection` for an explicit selector and `method_boundary` for
  the one-project method-boundary fallback. Both bases report partial coverage
  because earlier Product Repository changes are outside watcher coverage.
- These baseline attempts are bounded observations. They do not prevent writes,
  identify the actor that changed a file, store raw file contents, or create
  OS-level enforcement.

<a id="process-environment"></a>
## Process Environment

The MCP process interprets environment input in the bounded roles below.

Supported operator and Runtime Home inputs:

- `VOLICORD_HOME`
- `VOLICORD_LOCAL_WEB_CONSENT`
- standard platform home variables in non-discovery modes when
  `VOLICORD_HOME` is absent: `HOME`, `USERPROFILE`, and the `HOMEDRIVE` plus
  `HOMEPATH` pair

`VOLICORD_HOME` selects the Runtime Home for the process. A personal, local, or
user-wide managed host overlay writes the absolute Runtime Home selected by the
administrative setup that created it. A shared repository-visible entry cannot
embed that local path: its host-specific forwarding directive passes the
launching host process's `VOLICORD_HOME` into the child. Repository-discovery
mode requires that forwarded value to be present, nonempty, and absolute and
never substitutes the platform default. The launching host environment
therefore must provide the same absolute local Runtime Home selected when that
clone was initialized.
`VOLICORD_HOME` does not select a project, connection intent, actor provenance,
operation category, connection mode, or host trust state. The stdio process and
`--check` use it before entering startup validation. Help and version modes do
not use it.

`VOLICORD_LOCAL_WEB_CONSENT=0`, `false`, `off`, or `disabled` disables the
stdio local web consent listener. Other values do not change the listener
address or token policy.

`VOLICORD_MCP_VERIFICATION=1` is a diagnostic-only marker. The administrative
`volicord connection verify` flow sets it automatically for the child MCP
handshake. An operator may set it manually only for the bounded
[manual stdio lifecycle probe](#manual-stdio-lifecycle-probe). It preserves
normal connection and project startup checks but classifies the process as a
verification probe, so the process does not create a startup session-watch
baseline or managed Codex runtime observations. It is not a normal host
configuration setting.

Volicord-managed personal or user-scoped Codex configuration can carry these
local managed-launch provenance markers:

- `VOLICORD_MCP_LAUNCH=managed_host`
- `VOLICORD_MCP_HOST=codex`
- `VOLICORD_MCP_CONNECTION_ID=<connection_id>`
- `VOLICORD_MCP_PROJECT_ID=<project_id>` when the command has a project binding

These markers are part of a local Volicord-managed configuration identity, not
general operator selectors. Do not hand-add or alter them to make a
user-managed launch appear managed; regenerate managed configuration with
`volicord init` or `volicord connection add`. Their connection and optional
project values must match the corresponding process arguments. A partial or
mismatched marker set is invalid managed provenance and does not create managed
lifecycle observations. A repository-discovery launch uses its exact typed
descriptor and host selector as managed launch provenance and must not carry
these markers. Neither form supplies a Codex native session identity. The
markers and descriptor grant neither project access, host trust, session
binding, nor broader authority.

Local connection process binding is supplied by `--connection <connection_id>`
in personal/local generated configuration or user-managed generic host
configuration. It names the stored `connection_internal_id` and is not a normal
user-chosen value. Shared repository configuration instead supplies only
`--discover-repository --host <host>`; startup resolves the canonical current
Git root through local Runtime Home registration to one connection and one
project. In either form, the resolved Agent Connection and Runtime Home registry
state supply connection mode, connected projects, and adapter-derived
`actor_source` and `operation_category`. No other Volicord-specific environment
variable is a supported operator setting.

Current MCP Runtime Home resolution:

1. A present but empty `VOLICORD_HOME` is an error.
2. In repository-discovery mode, an absent or relative `VOLICORD_HOME` is an
   error. Together with the empty-value rule above, this requires a present,
   nonempty, absolute value before platform-default Runtime Home substitution,
   registry access, or repository discovery.
3. An absolute `VOLICORD_HOME` is used as supplied.
4. Outside repository-discovery mode, a relative `VOLICORD_HOME` is resolved
   against the process current working directory without requiring the path to
   exist.
5. Outside repository-discovery mode, when `VOLICORD_HOME` is absent, derive
   the default user home from the platform home variables and append
   `.volicord`. Non-Windows platforms try `HOME`, then `USERPROFILE`, then
   `HOMEDRIVE` plus `HOMEPATH`. Native Windows tries `USERPROFILE`, then
   `HOMEDRIVE` plus `HOMEPATH`, then `HOME` when it is not a WSL-style mount
   path.
6. Do not require canonicalization before startup validation.

## Startup Validation

Before entering the stdio loop, `volicord mcp --stdio` validates either an
explicit local Agent Connection binding or a repository-discovery binding and
the local registry records it depends on.

Startup validation requires:

- the Runtime Home registry exists and is valid
- in explicit-binding mode, the configured `connection_id` process argument
  names an existing stored `connection_internal_id`
- the connection is enabled
- the connection mode is supported
- at least one connected project row is readable
- the installation profile can resolve the MCP command information needed for
  diagnostics
- registry JSON and metadata needed for startup are valid

Repository-discovery mode performs these additional fail-closed steps before
the shared validation above:

1. Require an explicitly forwarded, nonempty, absolute `VOLICORD_HOME`. An
   absent, empty, or relative value fails startup before platform-default
   substitution or registry access.
2. Canonicalize the process current directory and walk its ancestors to the
   nearest valid Git worktree root, including supported gitdir-file and linked
   worktree layouts.
3. Require that exact canonical root to be a project registered in the selected
   local Runtime Home.
4. Select enabled connections whose host matches `--host`, whose intent is
   `shared`, whose host scope is project, and whose Connection Projects contain
   that project.
5. Require exactly one match and narrow the process allowlist to that project.

No match fails with `REPOSITORY_DISCOVERY_CONNECTION_NOT_FOUND`; multiple
matches fail with `REPOSITORY_DISCOVERY_CONNECTION_AMBIGUOUS`; an unregistered
clone fails with `REPOSITORY_DISCOVERY_PROJECT_NOT_REGISTERED`. Diagnostics
name the repository and Runtime Home and direct the operator to the applicable
`volicord init --shared`, `connection verify`, or `connection list` and
duplicate-removal action. The adapter never chooses one ambiguous row and never
reads a Connection ID or project ID from repository files.

Startup validation does not grant host trust and does not record user-owned
judgments. Project availability, project status, path separation, repository
root matching, and mode compatibility are verified per call as defined by
[Agent Connection](agent-connection.md#current-connection-context).

A stored Agent Connection can remain after it reaches zero connected projects.
That persistence is not startup eligibility: a new stdio process and startup
check fail while there are no connected projects.

An already running process is different from a new process. A process that
passed startup while at least one project was connected refreshes registry state
for project routing. After the last membership is removed, project discovery
may report no available project and public tools that require project routing
reject because no connected project remains.

## Managed-Host Session Input

Managed Codex launch provenance and session binding are separate states. The
reviewed path retains exact `clientInfo.name=codex-mcp-client` and
`clientInfo.version=0.144.4` from a successful `initialize`; initialization
alone does not bind a managed session. The canonical installed-host coordinate
is `0.144.4`, parsed from the exact probe envelope `codex-cli 0.144.4`.
These exact versions are preserved validation and release-evidence coordinates,
not runtime feature gates. A different valid version is retained as observed;
feature availability comes from the current capability probes owned by
[Agent Connection](agent-connection.md#host-feature-support), not equality to
this reviewed coordinate.

An actual successful managed local-web handoff, after current capability,
listener, binding, response-budget, and token-creation checks, records
`model_separated_user_action_ui=passed` and
`mcp_capability_advertised_and_exercised=passed` for both managed profiles. An
`initialize` response, capability advertisement, configured listener, or token
candidate alone does not pass either probe. Current listener, binding, or
configuration failure records a bounded failed or unavailable observation.
Explicit advertised-capability absence records only
`mcp_capability_advertised_and_exercised=unsupported`; it does not relabel the
separate native user-action surface. These observations remain bound to the
current Agent Connection fingerprint and actual client coordinates.

When a managed binding materializes its `session_watch_baselines` row, the
baseline's `metadata_json` retains only the exact bounded initialize identity as
top-level `client_name` and `client_version` alongside the existing bounded
watch metadata. These strings are the actual successful initialize values; the
adapter does not infer them from host kind, executable or probe text,
configuration, protocol version, constants, request metadata, or another
session. The live-host release recorder may read those two fields. The raw
initialize request, its other parameters, and raw protocol, session, thread,
turn, or tool-call payload are not retained for that purpose.
One managed baseline retains exactly one client pair. Re-observing the same
initialize pair is idempotent; an existing managed baseline with a missing,
partial, or different client pair is a binding conflict and must not be used as
successful managed-client provenance or repaired by replacement.

The first structurally valid call to a known tool after the ready transition
must carry `_meta.threadId` and object
`_meta["x-codex-turn-metadata"]` with string `session_id`, `thread_id`, and
`turn_id`. The flat `threadId` must exactly equal the nested `thread_id`. Each
native value must be 1 through 256 UTF-8 bytes matching
`[A-Za-z0-9._:-]+`. Only after JSON-RPC shape, known tool name, and
`arguments` validation succeed does the adapter derive
`managed_host_session_id` from `session_id` and a domain-separated in-memory
thread-binding digest from `thread_id`. Those bindings are immutable for the
stdio process. Later calls must match both; `turn_id` may change for a new
turn. Raw session, thread, and turn values are discarded after validation and
hashing.

Missing, malformed, or mismatched metadata returns JSON-RPC `-32602` before a
diagnostic session, session-watch row, managed lifecycle event, tool-invocation
row, Core call, token, or local-web handoff is created. This hidden metadata is
transport input, not a public tool argument, authority, or host attestation.
Environment variables, process IDs, process ancestry, arrival time, proximity
to a hook event, and newest-session lookup must not substitute for it. Claude
Code uses its owner-defined adapter input; both hosts map a validated native
session to the opaque `managed_host_session_id` defined by
[Host Release Evidence](host-release-evidence.md). Raw session, thread, turn,
event, call, capture, and invocation identifiers are never persisted, logged,
diagnosed, rendered, or attached to evidence. Missing, invalid, or mismatched
binding remains ineligible for Strong Evidence and must not be repaired by
synthesizing a replacement.

`diagnostics.sqlite` is best-effort operability storage, not binding authority.
Corruption, write denial, or an existing conflicting diagnostic coordinate
cannot reject otherwise valid managed metadata or change an MCP result, guard
result, Core result, or authoritative binding. Such diagnostic persistence is
skipped or recorded as a nonfatal diagnostic failure where possible. Exact
ownership conflicts still come from project Agent Session and registered
connection state. This non-authoritative diagnostic boundary does not weaken
the zero-effect rejection above for invalid or mismatched request metadata.

## Agent-Connection-Bound Process

One `volicord mcp --stdio` process is bound to:

- one stored Agent Connection, selected either by one local `connection_id`
  process binding or by the unique repository-discovery result
- in repository-discovery mode, exactly one registered project selected from
  the canonical current Git worktree

The Agent Connection supplies:

- one connection mode: `workflow` or `read_only`
- one connection intent: `personal`, `shared`, or `global`
- an explicit allowlist of connected projects
- host configuration inventory and last verification state through the registry

The resolved process binding remains fixed for the process lifetime. Changing
the Agent Connection identity requires another process or host configuration
update.
Changing project membership, mode, enabled state, or verification state takes
effect through registry state; each new process reruns startup validation
against the current registry state.

MCP call arguments and other MCP request bodies cannot set
`connection_internal_id`, `project_internal_id`, `actor_source`,
`operation_category`, connection intent, or connection mode. Administrative
connection-status output belongs to the `volicord` CLI; MCP startup diagnostics
belong to `volicord mcp --check`; public MCP tool arguments use the
`project_selector` behavior described below.

<a id="configuration-preflight"></a>
## Configuration Preflight

`volicord mcp --check --connection <connection_id>` runs the same Runtime Home,
Agent Connection, membership, and registry-shape startup validation used before
entering the stdio loop. It does not read stdin and does not perform complete
host verification.

On success, `--check` writes fixed summary lines, then one repeated
project-detail block for each connected project, in this order:

```text
configuration: valid
transport: stdio
Does not prove: public API availability, authentication service status, security boundary, full MCP Streamable HTTP compatibility, OS sandboxing, network isolation, write prevention, actor identity proof, correctness proof, test sufficiency proof, or human review completion
runtime_home: <absolute path>
connection_id: <connection_internal_id process-binding value>
mode: workflow|read_only
enabled: true|false
registry_read: passed
project_state_read: passed|failed
project_state_write: passed|readonly|failed|skipped
startup_observation: recordable|best_effort_skipped_if_readonly|skipped_verification_probe
effective_tool_mode: workflow|read_only_degraded|read_only|unavailable
tools_list_schema_validation: passed|failed
tool_naming_style: dotted_namespace
allowed_projects: <count>
available_projects: <count>
verification_scope: startup_check_only
watcher_status: pending_mcp_start|pending_project_selection|unavailable
watcher_baseline_created_at: <timestamp or empty>
watcher_coverage_start_at: <timestamp or empty>
watcher_coverage_basis: mcp_start|empty
watcher_partial_coverage_warning: <warning or empty>
project[0].project_id: <project_internal_id diagnostic value>
project[0].available: true|false
project[0].state_read: passed|failed
project[0].state_write: passed|readonly|failed|skipped
project[0].unavailable_reason: <value or empty>
project[0].repo_root: <path>
```

Project-detail rules:

- The detail index begins at zero.
- Without `--project`, one detail block is emitted for each allowed project in
  stable repository-root order.
- With `--project <project_id>`, the supplied value must be in the connection's
  allowlist and only that project's detail block is emitted.
- `connection_id` is the process binding for the stored Agent Connection.
- `Does not prove` summarizes the startup diagnostic non-guarantees and does not
  change the machine-readable Core response disclosure used by method calls.
- `registry_read` reports whether the Runtime Home registry was readable for
  startup validation.
- `project_state_read` summarizes read access for the selected project-state
  set. Per-project `state_read` lines report the same fact for each detail
  block.
- `project_state_write` summarizes effective write capability for the selected
  project-state set. Per-project `state_write` lines report the same capability
  for each detail block.
- `startup_observation` reports whether ordinary startup can record a bounded
  session-watch observation, would skip that observation under read-only
  storage, or is only a verification probe.
- `effective_tool_mode` reports the startup check's expected `tools/list` mode
  for the same connection and project storage capability.
- `tools_list_schema_validation` reports whether the MCP-visible tool list for
  that effective mode passes Volicord's client-compatibility checks for MCP
  tool names, object input schemas, required fields, and property shapes. It is
  a Volicord-side diagnostic and does not prove that a host will register or
  expose the tools.
- `tool_naming_style: dotted_namespace` reports that the effective Volicord
  tool names use the `volicord.*` dotted namespace. It does not create
  dot-free aliases.
- `allowed_projects` describes the Agent Connection allowlist as a whole.
- Unavailable projects still emit every project-detail key.
  `unavailable_reason` is populated for unavailable projects and empty for
  available projects.
- `verification_scope: startup_check_only` is a startup and preflight statement
  only, not complete host verification.
- `--check` does not create a session-watch baseline. `watcher_status:
  pending_mcp_start` means a future project-bound stdio or HTTP session can
  start coverage with basis `mcp_start`; `pending_project_selection` means a
  future session must select a project before coverage starts.
- Empty `watcher_baseline_created_at`, `watcher_coverage_start_at`, and
  `watcher_coverage_basis` values mean no baseline was created by this preflight
  command.
- `--check` output does not include administrative status fields for connection
  presence, connected-project count, or project display name.

Startup validation failure:

- writes a diagnostic to stderr through the process entry point
- exits with code `1`
- does not enter the stdio loop or wait on stdin

A successful `--check` is not a complete host connection result. Complete host
verification requires durable Agent Connection state, host configuration
installation, satisfied host-owned gates when observable, successful MCP
initialization, and successful tool discovery, as defined by
[Administrative CLI](admin-cli.md#agent-connection-result-states).

## MCP Wire Behavior

`volicord mcp --stdio` supports MCP protocol version `2025-11-25` over stdio.
It does not advertise simultaneous compatibility with older MCP protocol
versions. Each new process or stdio connection starts a new MCP lifecycle and
must complete its own initialization sequence.

The server initialization response includes MCP server instructions. Those
instructions may describe Volicord tool selection, repository-root project
routing, and limitations, but they are guidance only; they are not access
control or a guarantee of model behavior.

### Framing And JSON-RPC Validation

Framing rules:

- Each non-empty stdin line contains exactly one UTF-8 JSON-RPC message object.
- The JSON root must be one JSON-RPC message object. For the Volicord
  client-to-server baseline, the supported message objects are requests and the
  `notifications/initialized` notification. Arrays, primitive JSON roots, and
  `null` are invalid MCP stdio messages.
- JSON-RPC batches are not supported. An array input receives one Invalid
  Request response, not one response per array element.
- Messages are delimited by newlines and must not contain embedded newlines.
- Each output line contains one JSON-RPC response object, except that a
  server-initiated `elicitation/create` request may be written during an
  elicitation-capable `tools/call`. `volicord mcp --stdio` writes no readiness
  message before `initialize`.
- Stdin EOF ends the process after stdout is flushed.

JSON-RPC validation rules:

- `jsonrpc` must be exactly `"2.0"`.
- A request `method` must be a string.
- Request IDs may be strings or integers and must not be `null`.
- A classifiable notification has a string `method`, no `id`, and receives no
  response even when its MCP method parameters are malformed.
- An object without an `id` is not automatically a valid notification; it must
  still satisfy the notification shape.
- For supported MCP requests, method `params`, when present, must be an object.
  For lifecycle notifications, absent or object `params` are the only shapes
  that can affect lifecycle.

Notification classification is based on the JSON-RPC envelope before MCP
method-parameter validation. Once a message is classifiable as a notification,
malformed `params` do not produce any JSON-RPC response. Those `params` are
still invalid for lifecycle purposes: a malformed `notifications/initialized`
does not move the connection to ready, and request-only methods received as
notifications are ignored and must not execute.

Error classification:

| Condition | MCP response |
|---|---|
| JSON parse failure | JSON-RPC `-32700` Parse error |
| Invalid JSON-RPC message structure, including arrays, primitive roots, missing or invalid `jsonrpc`, invalid request `id`, missing or non-string request `method`, or malformed non-notification objects | JSON-RPC `-32600` Invalid Request |
| Lifecycle violation on a request, including a request before `initialize`, `tools/call` before `notifications/initialized`, or duplicate `initialize` | JSON-RPC `-32600` Invalid Request |
| Unknown request method | JSON-RPC `-32601` Method not found |
| Malformed method parameters on a request | JSON-RPC `-32602` Invalid params |
| Unknown tool name in a structurally valid `tools/call` request | JSON-RPC `-32602` Invalid params |
| Adapter or server internal failure | an appropriate JSON-RPC internal-error response |
| Classifiable notification, including one with malformed method parameters | no response; invalid parameters do not trigger lifecycle transitions or request-only behavior |

### Protocol Version And Lifecycle

The first valid MCP request in a connection is `initialize`. A valid
`initialize` request has object `params` with:

- `protocolVersion` as a string
- `capabilities` as an object
- `clientInfo` as an object containing string `name` and `version` fields; each
  field is 1 through 256 UTF-8 bytes, contains at least one non-whitespace
  character, contains no control character, and is otherwise preserved exactly

If `params.capabilities.elicitation` is an object, the adapter treats the MCP
client as eligible for server-initiated elicitation. The separate Volicord
extension capability is:

```json
{
  "capabilities": {
    "experimental": {
      "io.volicord/user-channel": {
        "model_invisible_user_surface": true
      }
    }
  }
}
```

Only the exact boolean `true` supplies the client's cooperative declaration for
a model-invisible local-web handoff. It is necessary but never sufficient for
eligibility. Missing members, `false`, wrong types, wrong namespaces, and
malformed nested objects are capability-unavailable rather than initialize
errors. The flag is not user authority or proof of host trust; it is the
client's promise that the namespaced tool-result `_meta` value is delivered to
a user-owned surface and never supplied to model context. The adapter retains
the exact `clientInfo.name` and `clientInfo.version` as verification inputs;
client text is not identity proof. Other capability entries do not create
Volicord behavior by themselves.

Examples use the fields listed above. `volicord mcp --stdio` may accept additional MCP
`Implementation` metadata allowed by the 2025-11-25 schema, such as `title`,
`description`, `icons`, or `websiteUrl`.

The successful initialize result returns `serverInfo.name=volicord-mcp` and
keeps the inherited Cargo package SemVer in `serverInfo.version`.
`serverInfo` contains only standard MCP `Implementation` fields. The standard
initialize Result `_meta` object exposes the Volicord extension
`_meta["io.volicord/build"]`; no non-standard `serverInfo.buildId` field is
used. The extension value is the same structured build object documented for
`volicord doctor --json`, including its `build_id`, Git metadata source,
target, exact profile or approximate profile class, optimization level, and
debug state. It has no build timestamp. Unknown Git metadata is explicit, and
a dirty tree is labeled without claiming to identify its exact modified
contents. This build object contains neither the final executable digest nor a
release evidence digest and is not a release evidence manifest. Host-capability
evaluation must obtain its expected `evidence_artifact_sha256` from a
separately verified exact-final-artifact release evidence manifest or receipt
outside the executable.
The initialize result also advertises the
`capabilities.experimental["io.volicord/user-channel"]` extension so clients
can negotiate this optional handoff; advertisement alone does not make the
client capability available.

Protocol-version negotiation:

- If the client requests `2025-11-25`, `volicord mcp --stdio` returns `2025-11-25`.
- If the client sends another syntactically valid protocol-version string,
  `volicord mcp --stdio` returns the version it supports: `2025-11-25`.
- The server response does not claim simultaneous compatibility with older MCP
  protocol versions.

Lifecycle states:

| Connection point | Valid client messages | Result |
|---|---|---|
| Before successful `initialize` | `initialize` request | On success, the server returns `protocolVersion: "2025-11-25"` and waits for `notifications/initialized`. |
| Waiting for `notifications/initialized` | `notifications/initialized` notification; `ping` request; `tools/list` request | `notifications/initialized` completes the transition to ready. `ping` may be used after `initialize` has succeeded, including while the server waits for the notification. `tools/list` is read-only discovery available after the successful `initialize` response. |
| Ready | `ping`, `tools/list`, `tools/call` | Normal MCP tool discovery and tool execution are available. |

`tools/list` is available after the successful `initialize` response, including
while the server waits for `notifications/initialized`, and remains available
after the ready transition. `tools/call` is available only after
`notifications/initialized` has completed the ready transition. A duplicate
`initialize` request is invalid. An early or malformed
`notifications/initialized` notification does not make the connection ready.

<a id="manual-stdio-lifecycle-probe"></a>
### Manual Stdio Lifecycle Probe

Use this probe only for troubleshooting a configured Agent Connection outside
the active Codex host process. Replace `<repo>`, `<connection_id>`, and
`<project_id>` with values from the connection you are checking. Run from
`<repo>` unless the process environment already selects the intended Runtime
Home. A manual or elevated probe can prove that the MCP server can run in that
launch environment; it does not prove that the active Codex session registered
or exposed the tools. `VOLICORD_MCP_VERIFICATION=1` marks the launch as a
verification probe: it keeps normal Agent Connection and project startup
checks, but does not record the process as a Codex host runtime observation or
create a startup session-watch baseline. A `volicord mcp --stdio` launch
without the managed Codex provenance markers is classified as manual CLI
startup for host-runtime observation purposes.

The process command shape is:

```sh
VOLICORD_MCP_VERIFICATION=1 volicord mcp --stdio --connection "<connection_id>" --project "<project_id>"
```

`initialize` followed by `tools/list` should return successful JSON-RPC
responses and list the mode-appropriate `volicord.*` tools:

```sh
cd "<repo>"
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"volicord-lifecycle-probe","version":"0.0.0"}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | VOLICORD_MCP_VERIFICATION=1 volicord mcp --stdio --connection "<connection_id>" --project "<project_id>"
```

`initialize`, then `notifications/initialized`, then `tools/list` should also
succeed:

```sh
cd "<repo>"
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"volicord-lifecycle-probe","version":"0.0.0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | VOLICORD_MCP_VERIFICATION=1 volicord mcp --stdio --connection "<connection_id>" --project "<project_id>"
```

`tools/call` before `notifications/initialized` should fail with JSON-RPC
Invalid Request, because tool execution is not ready before the initialized
notification:

```sh
cd "<repo>"
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"volicord-lifecycle-probe","version":"0.0.0"}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"volicord.status","arguments":{}}}' \
  | VOLICORD_MCP_VERIFICATION=1 volicord mcp --stdio --connection "<connection_id>" --project "<project_id>"
```

After `notifications/initialized`, a read-only `volicord.status` call can
succeed when project state is readable. When effective storage is read-only, a
workflow mutation call may be absent from `tools/list`; if a stale host cache
still calls it, the MCP result wraps a Volicord `MCP_UNAVAILABLE` rejection
rather than proving write-capable storage.

Supported MCP request methods:

- `initialize`
- `ping`
- `tools/list`
- `tools/call`

When the initialized client declared `capabilities.elicitation`, the server may
send one nested `elicitation/create` request while processing
`volicord.request_user_action`. That request is server-initiated MCP
protocol traffic, not an Agent Connection tool. The client response to that
server request is validated before any User Channel recording attempt.

The supported lifecycle notification is `notifications/initialized`.

<a id="tool-discovery-and-toolscall-response-wrapping"></a>
## Tool Discovery And `tools/call` Response Wrapping

After a successful `initialize` response, `tools/list` exposes tools according
to the current stored Agent Connection mode and the effective storage
capability of the selected allowed projects:

| Mode and storage capability | MCP-visible tools |
|---|---|
| `workflow` with writable project state | `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.get_operation_result`, `volicord.prepare_write`, `volicord.prepare_evidence_capture`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_action`, `volicord.reconcile_changes`, `volicord.check_close`, `volicord.close_task`, `volicord.list_projects` |
| `workflow` with readable but non-writable project state | `volicord.status`, `volicord.get_operation_result`, `volicord.request_user_action` (resume only), `volicord.check_close`, `volicord.list_projects` |
| `read_only` with readable project state | `volicord.status`, `volicord.get_operation_result`, `volicord.check_close`, `volicord.list_projects` |
| No readable allowed project state | `volicord.list_projects` |

The MCP adapter may inspect project state read-only during startup and
discovery. If project state is readable but not writable in the current MCP
host environment, read-compatible method tools remain visible and workflow
mutation branches are withheld even when the stored Agent Connection mode is
`workflow`. The mixed `volicord.request_user_action` tool remains visible so
its explicit resume branch can read an existing request; its create branch
returns `MCP_UNAVAILABLE`. If no allowed project state can be read, the adapter keeps only
`volicord.list_projects` visible so the caller can inspect project
availability.

For one resolved connection mode and effective storage capability, this is a
static tool set. Task state, current blockers, write-ticket state, and a model's
previous call do not add or remove tools; those conditions are reported by tool
results. The complete compact `tools/list` result object, exactly
`{"tools":[...]}` before the JSON-RPC envelope and request ID are added, must be
at most 35,000 serialized UTF-8 bytes for every supported mode and storage-capability
combination.

When effective storage is read-only, read-compatible public method tools run
without creating session-watch baselines, `tool_invocations`, `task_events`, or a
new `project_state.state_version`. If a stale host-side tool cache still calls
a public workflow mutation tool while the selected project state is not
writable, the adapter returns a normal Volicord rejection with
`code=MCP_UNAVAILABLE`, `operation_category=agent_workflow`, and message
`Volicord project state is not writable in the current MCP host environment.`

In `workflow` mode, the Evidence path is:

- Use `volicord.prepare_evidence_capture` to create the exact current-basis
  intent when a registered command, tool, guard, or watcher source will provide
  the observation. Receipt fulfillment is not an MCP tool.
- Use `volicord.stage_artifact` only to prepare an Evidence attachment input
  when bytes or a safe notice are needed.
- Use `volicord.record_run` to record the Run or observation, target-scoped
  evidence update, observation provenance, and any attachment link or promotion.

A staged handle alone is not accepted Evidence and does not satisfy Close
Status.

The MCP-visible tools are not the same thing as the public Volicord Core API
method list. `volicord.check_close` maps to the first-class read-only Core
method for close readiness. `volicord.close_task` maps to the workflow-only
Core mutation method and is not listed for `read_only` connections.
`volicord.get_operation_result` maps to the read-only Core method for bounded
retrieval of one exact historical mutation response and is listed for both
connection modes when project state is readable.
`volicord.resolve_user_action` is the public Core API method for every User
Channel resolution, but it is not exposed as an Agent Connection MCP tool; see
[API Methods](api/methods.md) for the public method
owner table.

A structurally valid `tools/call` request has object `params` with:

- `name` as a string
- optional `arguments` as an object

The managed Codex path additionally consumes the hidden request-side `_meta`
binding described in [Managed-Host Session Input](#managed-host-session-input).
It is not exposed by `tools/list`, is not part of a public tool input schema,
and is never copied into Core request arguments.

Missing `arguments` are treated as an empty object. `arguments: null` and
non-object `arguments` are malformed method parameters and return JSON-RPC
`-32602`. Unknown tool names are protocol errors and return JSON-RPC `-32602`.

For public Volicord method tools, `tools/list` exposes MCP-visible input schemas
that carry workflow-domain arguments rather than the Core request envelope. The
visible schema exposes optional `project_selector` and must hide internal
request envelopes, protocol metadata, `project_id`, `connection_id`,
`request_id`, `idempotency_key`, `expected_state_version`, `dry_run`, `locale`,
`actor_source`, `operation_category`, and verification-basis fields. Those
hidden fields are not required or accepted public MCP tool arguments. If raw
public method-tool arguments include them, the adapter rejects the call before
Core execution.

The MCP argument projection applies omission defaults only where omission has
exactly the same meaning as the previously accepted explicit `null` or empty
array:

- `volicord.intake`: `initial_context_refs=[]` and `initial_source_refs=[]`
- `volicord.update_scope`: `goal_summary=null`, `scope_update=null`,
  `scope_boundary=null`, `non_goals=null`, `acceptance_criteria=null`,
  `autonomy_boundary=null`, `baseline_ref=null`, and
  `related_scope_decision_refs=[]`
- `volicord.get_operation_result`: `cursor=null`
- `volicord.prepare_write`: `task_id=null`, `change_unit_id=null`, and
  `sensitive_categories=[]`
- `volicord.prepare_evidence_capture`: `expected_exit_code=0` for the command
  branch, `expected_success=true` for the tool branch, and
  `expected_complete=true` for the registered-connection branch; explicit null
  has the same meaning
- `volicord.stage_artifact`: `expected_sha256=null`,
  `expected_size_bytes=null`, and `relation_hint=null`
- `volicord.record_run`: `run_id=null`, `write_ticket_id=null`,
  `artifact_inputs=[]`, `evidence_updates=[]`, `evidence_observations=[]`, and
  `close_assessment=null`; inside each `evidence_updates` item,
  `supporting_run_refs=[]`, `observation_refs=[]`,
  `supporting_artifact_refs=[]`, and `gap_refs=[]`; inside each
  `evidence_observations` item, `observed_by_actor_source=null`,
  `tool_name=null`, `tool_invocation_id=null`, `tool_metadata={}`,
  `input_refs=[]`, `source_refs=[]`, `output_artifact_refs=[]`, and
  `limitations=[]`
- `volicord.request_user_action`: under `request.operation=create`,
  `request.change_unit_id=null`,
  `request.action.sensitive_action_scope=null`,
  `request.action.options=null`, `request.action.affected_refs=[]`, and
  `request.expires_at=null` for judgment variants; the observation variant has
  no caller expiry default; `request.operation=resume` has no create defaults

Every MCP-visible mutation tool also accepts `detail=summary|workflow|full`.
Omitted `detail` defaults to `summary`. This is an adapter response-projection
choice, not a Core request field and not permission to omit any method-owned
request member.

These defaults belong only to the MCP-visible argument DTO. After decoding, the
adapter constructs the complete Core request shape. They do not change the
public Core API present-member contract owned by the focused method references.
For `volicord.request_user_action`, the nested `request.operation` discriminator
is required. Its create variant requires `request.task_id` and the complete
closed `request.action`; its resume variant requires only
`request.user_action_request_id` and rejects create fields. For
`volicord.record_run`, `target` and `coverage_state` remain
required inside each `evidence_updates` item, while `target`, `source_kind`,
`assurance_level`, and `observed_at` remain required inside each
`evidence_observations` item. Each `target` is the strict tagged
acceptance-criterion or supplemental-claim union owned by the API state schema.
This rule supplies no implicit value
for any other field; the exact advertised `required` array remains
authoritative.

Schema generation has two explicit detail modes. `ToolSchemaDetail::RuntimeCompact`
is used for MCP `tools/list`; it omits every `inputSchema.examples` member and
keeps each description to the tool's outcome, authority or write boundary, and
when to call it. It does not embed mode matrices, long procedures, recovery
catalogs, or examples. `ToolSchemaDetail::Documentation` retains the canonical
examples and exhaustive branch documentation used by generated documentation
and schema checks. Runtime compact input schemas retain each tool's exact
top-level properties, required list, closed-object rule, branch discriminators,
and the essential nested required/closed skeleton, while omitting repeated
nested validation detail that cannot fit the wire-size bound. Documentation
detail remains the exhaustive validation schema, and the server always applies
that exact validation regardless of the advertised detail. Runtime compact
mode advertises a bounded root-object `outputSchema`; Documentation mode
retains the exhaustive public response branches. These presentation differences
do not change server-side validation or any public method request or response
contract.
Mode-to-kind compatibility such as
`advisor`/`shaping_update`, `direct`/`direct`, and `work` with
`shaping_update` or `implementation` remains in Documentation detail and the
method owner, not the runtime description.

Every listed Volicord tool also exposes an MCP 2025-11-25 `outputSchema` whose
root type is `object`. Runtime compact detail uses the bounded root-object
advertisement defined above. Documentation detail derives read-only public
method tool schemas from their public method response branches. Mutation tools
additionally advertise
summary and workflow wrappers that pair a fresh `AuthorityReceipt` with the
method result needed for the next step, a full wrapper that pairs the same
fresh receipt with the exact public method response, and bounded post-effect
recovery branches. `volicord.request_user_action` instead uses the compound
`agent_workflow_result`, replay marker, snapshot-anchored current status, and
nullable `user_channel_resolution` shape; a user-only resolution never
replaces the exact request result.
`volicord.list_projects` uses its exact
adapter-utility result schema. A server result that includes
`structuredContent` must conform to the advertised schema.

`tools/list` supplies the following conservative MCP `annotations`:

| Tool class | `readOnlyHint` | `destructiveHint` | `idempotentHint` | `openWorldHint` |
|---|---:|---:|---:|---:|
| `volicord.status`, `volicord.get_operation_result`, `volicord.check_close`, `volicord.list_projects` | `true` | `false` | `true` | `false` |
| `volicord.prepare_write`, `volicord.prepare_evidence_capture`, `volicord.stage_artifact` | `false` | `false` | `false` | `false` |
| `volicord.intake`, `volicord.update_scope`, `volicord.record_run`, `volicord.request_user_action`, `volicord.reconcile_changes`, `volicord.close_task` | `false` | `true` | `false` | `false` |

For the non-destructive mutation row, `destructiveHint=false` means the tool's
committed storage updates are additive rather than replacing, invalidating, or
consuming existing authority state. It does not mean that the call is read-only
or that a later distinct MCP call is replay-safe.

`volicord.record_run` uses `destructiveHint=true` because a commit may consume a
compatible write ticket or staged input, update evidence and blockers,
increment `close_basis_revision`, invalidate current judgments, and replace the
current close basis or leave a previous basis stale. A committed
`volicord.request_user_action` may also change the Task lifecycle while it
creates the pending user action. The method and storage-effect owners define the
exact effects; the annotation conservatively tells MCP clients that these tools
can alter existing authority state.

The one annotation covers both `volicord.request_user_action` operations and is
therefore conservative. `request.operation=create` can have the documented
destructive mutation effects. `request.operation=resume` is a read-only exact
historical replay plus a current projection and creates no effect, even though
the tool-level annotation remains `readOnlyHint=false`,
`destructiveHint=true`, and `idempotentHint=false`.

Mutation-capable tools have `idempotentHint=false` because a distinct create or
mutation call ordinarily receives fresh adapter-managed request identity. Core
replay handling for one generated identity does not promise that a later
visible mutation call has the same result or no additional effects. The
explicit `request_user_action` resume branch is the exception in behavior: it
names the already committed request and never generates a replacement
idempotency key or mutation.

These values are client hints, not trusted authorization facts. They do not
grant Agent Connection authority, bypass host trust or approval, suppress a
host safety review, prove idempotent storage behavior outside the advertised
surface, or broaden access beyond the selected connection and project. A host
must continue to apply its own trust, approval, and sandbox policy.

Project selection is resolved from the Agent Connection context. When exactly
one allowed project is connected, public method tools may omit project
selection. Multi-project connections require the `project_selector` value
returned by `volicord.list_projects`; otherwise the adapter rejects the call
with actionable ambiguity text. Agents must not infer project identity from
folder names, current working directory, MCP roots, host labels, repository
labels, or memory.

`volicord.list_projects` returns the selected connection binding, mode, project
selectors, project availability, repository-root display paths, and
session-watch coverage fields for the current MCP session:
`watcher_status`, `watcher_baseline_created_at`,
`watcher_coverage_start_at`, `watcher_coverage_basis`, and
`watcher_partial_coverage_warning`. In a multi-project session with no explicit
project selection yet, `watcher_status=pending_project_selection`, the coverage
timestamps and basis are `null`, and the warning states that coverage has not
started. After explicit project selection creates a baseline, later
`volicord.list_projects` output reports the stored coverage start and basis.

For mutation and create branches, the MCP adapter generates the Core envelope before dispatch. It supplies
`request_id`, `idempotency_key` for workflow effects, `expected_state_version`
from the selected project's current state where Core freshness requires it,
`dry_run=false`, the default locale, the selected internal project, and the
derived invocation context. Public MCP arguments cannot override those facts.
The request-user-action resume branch instead derives read-only access context
and looks up the stored origin; it does not generate a mutation envelope,
idempotency key, or expected state version.

`volicord.status` uses a compact public `detail` argument instead of exposing
the Core include matrix. Supported values are `summary`, `workflow`, and
`full`; omitted `detail` defaults to `workflow`.

Mutation `detail` has the same three values but a different default and effect.
`summary`, the default, returns `authority_receipt` and a compact
`method_result`. `workflow` returns those fields plus current `next_actions`.
`full` returns `authority_receipt` and the exact public response under
`method_result`. A Core/domain rejected branch keeps its existing response
object for every detail value. The adapter validates the argument before Core
entry.

Every workflow `next_actions` item emitted by the adapter has a non-null public
`owner_method`. When a non-terminal workflow state requires Agent action, the
primary action names the public `volicord.*` method that can perform it and
includes the bounded arguments or durable refs needed to call it; the Agent is
not required to issue an extra `volicord.status` merely to discover that method.
Local policy repair, host reload, connection setup, storage upgrade, and other
administrative work remain separately typed diagnostic actions with exact CLI
commands where applicable. They are not ownerless workflow next actions.

The compact `method_result` always preserves effect kind, resulting state
version, and committed event refs. It additionally preserves the issued write
ticket and decision for `volicord.prepare_write`, the exact capture-intent ref,
intent, and expiry for `volicord.prepare_evidence_capture`, the staged handle and expiry
for `volicord.stage_artifact`, the exact Run ref, registered `ArtifactRef`
values, newly recorded evidence-observation refs, and nullable
`close_basis_anchor` for `volicord.record_run`, per-finding results for
`volicord.reconcile_changes` without newly created request refs or forms, and
the exact request result, replay marker,
current-projection state version/time, separate safe resolution facts, and
resolution-derived refs for `volicord.request_user_action`. `close_basis_anchor` contains
`close_basis_revision`, `scope_revision`, `source_run_ref`, and nullable
`evidence_summary_ref`. It is a typed coordinate for the close basis stored on
the Task, not a `StateRecordRef` and not a separate close-basis record. A
resolved compact user-action outcome contains the exact closed three-field
request summary and no request ref, the exact historical
resolution ref, snapshot-anchored status, selected option ID and label or
evidence-observation summary, resolution outcome where applicable, and any
public resolution-derived refs; it omits the free-form user note and evidence
observation summary text.
`detail=full` is for callers that need fields beyond those next-step results,
not for recovering a required handle, ticket, Run or evidence ref, finding
result, or host-native selection.

The capture-intent compact result is part of the same bounded summary/workflow
and post-effect recovery order as other mutation results. If the authority
receipt cannot fit but the compact result can, recovery preserves the exact
intent ref, full intent needed by the registered source, and expiry. It never
projects a receipt or producer because this MCP call creates neither.

For a known tool, the adapter validates object `arguments` against the exact
advertised `inputSchema` before project selection, session-watch setup, generated
Core-envelope creation, or Core method entry. The bounded prevalidator collects
all independently discoverable failures for supported structural keywords:
local `$ref`, `type`, `enum`, `required`, `properties`, `additionalProperties`,
array `items`, and `allOf`/`anyOf`/`oneOf` branches. It reports multiple missing
root or nested fields, unknown fields, type mismatches, enum mismatches, and
array-item failures in one result when those failures can be evaluated
independently. Unsupported JSON Schema keywords are left to the typed decoder or
later owner and must not cause the prevalidator to reject an otherwise valid
input. A residual typed-decoder failure becomes one structured decode issue.

The fixed `MAX_VALIDATION_ISSUES` value is `32`. Validation stops traversing an
invalid aggregate when continuing would exceed that issue budget. Each union
branch receives an independent bounded evaluation: reaching the budget in an
earlier invalid `anyOf` or `oneOf` branch must not prevent evaluation of later
branches, and a later valid branch still makes the union acceptable. Returned
issue paths and messages are limited to `256` and `512` UTF-8 bytes,
respectively. Path shortening preserves RFC 6901 JSON Pointer syntax, and enum
and received-value previews are bounded before they enter a message. After JSON
escaping and compatibility-text wrapping, the compact JSON serialization of the
entire known-tool error `CallToolResult` is at most `65536` bytes. The adapter
may omit otherwise reportable issues to satisfy that final byte limit.

Input validation precedes adapter preconditions. After valid input decoding, a
public method-tool call performs deterministic repository-root project selection
and per-project validation owned by
[Agent Connection](agent-connection.md#current-connection-context). Ambiguous or
unavailable project selection is rejected before Core execution and its message
must name the `volicord project use` or `volicord connection add` command needed
to repair the state when applicable.

Known-tool input and adapter-precondition failures return a `CallToolResult` with
`isError: true`. `result.structuredContent` is an object with:

- `code`: `MCP_INVALID_ARGUMENTS` or `MCP_ADAPTER_PRECONDITION_FAILED`
- `tool_name`: the requested MCP tool name
- `retryable`: `true` for corrected arguments and `false` for an adapter
  precondition that requires connection, project, mode, or environment repair
- `reached_core: false`
- `committed: false`
- `reported_issue_count`: exactly the length of the returned `issues` array
- `truncated`: `true` exactly when further validation traversal or otherwise
  returnable path, message, or issue detail was suppressed by an issue, field,
  or whole-result bound; otherwise `false`
- non-empty `issues`, where every item has RFC 6901 JSON Pointer `path`, stable
  `code`, and human-readable `message`

The stable issue codes are `MCP_ARGUMENT_REQUIRED`, `MCP_ARGUMENT_UNKNOWN`,
`MCP_ARGUMENT_TYPE_MISMATCH`, `MCP_ARGUMENT_ENUM_VALUE`,
`MCP_ARGUMENT_DECODE_FAILED`, and `MCP_ADAPTER_PRECONDITION_FAILED`. The root
pointer is the empty string. `result.content[0].text` is a JSON serialization of
the same object and must parse equal to `result.structuredContent`.
Adapter-precondition and residual typed-decoder errors use the same
`reported_issue_count`, `truncated`, field-size, and whole-result rules.

These failures do not enter a Core method, commit Core method state, advance the
project aggregate state version, or create Core method events. Transport-owned
diagnostic lifecycle observations remain a separate boundary. Malformed
JSON-RPC request envelopes, non-object `tools/call.arguments`, and unknown tool
names keep their JSON-RPC error behavior. Core/domain rejected responses keep
their normal Volicord response shape and `isError: false` transport meaning.

`volicord mcp --stdio` does not advertise or implement MCP task-augmented tool
execution. A `tools/call` request does not return `CreateTaskResult`, and a
`task` parameter is not a supported baseline feature.

<a id="mutation-authority-receipt-projection"></a>
### Mutation Authority Receipt Projection

After a mutation returns `base.response_kind=result`, the adapter performs a
read-only `volicord.status` refresh for the same selected project and resolved
Task before it returns the tool result. It accepts the refresh only when the
status branch is a non-dry-run read-only result and its `AuthorityReceipt`
matches the refreshed `base.state_version`, project, Task, Task reference
version, and current status projection. The mutation's own Core effect remains
the method owner's effect; this refresh creates no second mutation.

For one applied or replayed mutation result, the adapter derives the exact
method result, compact method result, effect facts, nullable
`operation_result_ref`, fresh receipt, and current next actions once as one
canonical mutation outcome. Normal `detail`
projections, response-budget recovery, post-effect recovery, and authoritative
refresh recovery select from that same outcome. A recovery branch must not
recompute a different compact result or use a branch-local preservation order.
For an eligible committed or replayed agent-workflow Core mutation, the ref is
present and identical in every normal or recovery projection. It is independent
of the receipt/result preservation ladder and cannot be dropped to make another
candidate fit. Non-Core staging and results without an eligible durable row use
`operation_result_ref=null`.

For an accepted refresh:

- `detail=summary` returns `operation_result_ref`, `authority_receipt`, and the compact
  `method_result` in `result.structuredContent`.
- `detail=workflow` returns those fields plus `next_actions` in
  `result.structuredContent`.
- `detail=full` returns `operation_result_ref`, `authority_receipt`, and the exact public method response
  under `method_result` in `result.structuredContent`. If state changed after
  the method response was built and before refresh, `authority_receipt` is the
  fresh authority view; method-owned state inside `method_result` remains the
  result of that method invocation.
- `result.content[0].text` is a short compatibility summary of at most 512
  UTF-8 bytes. It is not a second JSON serialization of `structuredContent`.
- A compact `summary` or `workflow` `CallToolResult` is at most 65,536 bytes.
  If the refreshed projection cannot fit that bound without changing the
  Core-owned receipt, the adapter omits that projection rather than truncating
  authority data.
- A `full` `CallToolResult` is larger but still bounded at 262,144 bytes. It
  uses the same omission branch rather than returning an unbounded or truncated
  method response.

An oversized fresh projection returns a separate bounded post-effect recovery
branch with `isError=false`, so an MCP host does not classify an already-applied
operation as a failed mutation and automatically retry it. Its
`structuredContent` contains
`code=MCP_RESPONSE_BUDGET_EXCEEDED`, the method `tool_name`, the
`requested_detail`, `retryable=false`, `reached_core`, `committed`, nullable
`effect_kind`, `effect_applied`, nullable stable `effect_anchor`, nullable
`operation_result_ref`, nullable `authority_receipt`, nullable compact
`method_result` used by the requested tool,
`authoritative_refresh_succeeded=true`, `response_projection_omitted=true`,
`status_read_required=true`, and `completion_claim_withheld=true`. `committed`
reports a new Core commit, while `effect_kind` and `effect_applied` also report
non-Core effects such as a created staging handle and replayed applied effects.
The bounded recovery attempts, in order, the fresh receipt with the compact
method result, the fresh receipt alone, the compact method result alone, and
effect facts alone. Neither the receipt nor method result is truncated. In
particular, when a successful `volicord.stage_artifact` compact result fits after
the receipt does not, the recovery retains its staging handle and expiry. A
field that cannot fit at its preservation step is `null`.

An eligible durable `operation_result_ref` remains present at every preservation
step, including receipt-only, compact-only, and effect-facts-only recovery. A
caller whose exact result was omitted reads it with
`volicord.get_operation_result`, concatenates the returned chunks, and then
reads `volicord.status` for current authority. A caller must not resubmit the
mutation.

`effect_anchor` identifies the first committed authority event, staged handle,
or resulting state effect. It is an effect-correlation anchor, not an operation
result lookup credential. Only `operation_result_ref` is accepted by
`volicord.get_operation_result`; `volicord.status` cannot reconstruct the exact
method result.
This branch does not claim `MCP_UNAVAILABLE`, is not counted as an
authoritative-refresh failure, and does not return a partial receipt or
oversized status body. The caller must not submit a new mutation as a retry; it
must use every preserved field and read current status before acting.

If Core has already returned an applied result and later adapter work cannot
produce the normal wrapper, the adapter first performs the same validated
authority refresh and returns another `isError=false`, `retryable=false`
post-effect branch. `code=MCP_POST_EFFECT_ADAPTER_FAILED` identifies a failed
host User Channel adapter after the pending user action was created;
`code=MCP_RESPONSE_PROJECTION_FAILED` identifies a failure while building the
normal response projection. Both branches include the method `tool_name`,
`requested_detail`, effect facts, nullable `effect_anchor`, nullable
`operation_result_ref`, nullable `authority_receipt`, nullable `method_result`,
`authoritative_refresh_succeeded=true`, `response_projection_omitted=true`,
`status_read_required=true`, and
`completion_claim_withheld=true`. A projection failure preserves the exact
method result when it can be represented; a host-adapter failure may leave it
`null` when the canonical outcome has no method result or neither available
result representation fits the recovery budget. The bounded recovery attempts,
in order when each value is available, the fresh receipt with the exact result,
the fresh receipt with the compact method result, the fresh receipt alone, the
compact method result alone, and effect facts alone. Neither branch authorizes
replay of the mutation.

If the refresh call fails, returns a rejected or malformed branch, lacks a
receipt, or fails any freshness comparison, the adapter returns the same
success-class `isError=false` recovery boundary. Its bounded
`structuredContent` contains `code=MCP_UNAVAILABLE`, the method `tool_name`,
`retryable=false`, `reached_core`, `committed`, nullable `effect_kind`,
`effect_applied`, nullable stable `effect_anchor`, nullable
`operation_result_ref`, nullable compact `method_result`,
`status_read_required=true`, and
`completion_claim_withheld=true`. The compact result preserves tool-specific
next-step data such as a write ticket, staging handle, per-finding reconcile
outcomes, or selected Judgment outcome. It is never truncated; if that compact
result itself cannot fit the fixed recovery budget, the field is `null`. The
branch does not return the exact original success or completion body, a stale
receipt, or a private refresh error body. The caller must not resubmit the
mutation. When `operation_result_ref` is non-null, it retrieves the omitted
exact historical result; the caller must also read current status before
acting. The `effect_anchor` has the same correlation-only meaning described
above; status does not recover the omitted exact method result. Local session
diagnostics count this as an
authoritative-refresh failure without storing the error body.

Core/domain rejected mutation responses do not enter this success-projection
path. They retain the existing public response object and `isError=false`, with
a short compatibility text directing the client to `structuredContent`.

<a id="user-action-elicitation"></a>
### User Action Elicitation

`volicord.request_user_action` is the only Agent Connection tool that creates a
pending `UserActionRequest`, through the strict nested
`request.operation=create` variant. Its sibling
`request.operation=resume` variant names a directly created request and performs
read-only continuation. `volicord.resolve_user_action` is never exposed as an
MCP tool, and agent arguments cannot become a User Channel resolution.

After a `request.operation=create` commit, the adapter consumes the Core-owned
`UserActionInboxForm` only inside the selected User Channel renderer. The Agent
Connection result receives `AgentSafeUserActionRequestSummary` with only the
request ID, `status=pending`, and `next_actor=user`. A judgment form requests only a stored
`selected_option_id` and optional note. An evidence-observation form requests
one stored target selector, a non-empty subset of stored artifact IDs,
`supported` or `contradicted`, and a bounded summary. Labels, descriptions,
consequences, and default markers are display-only. The complete stored
`EvidenceTarget` metadata and complete exact `ArtifactRef` metadata are also
display-only, including target statements and the `ArtifactRef` fields
`display_name`, `content_type`, `sha256`, `size_bytes`, `integrity_status`,
`redaction_state`, `availability`, `created_by_run_ref`,
`created_by_actor_source`, and `storage_ref`. Only the selected target selector and artifact IDs
are submitted; display metadata is not submitted as candidate authority. MCP
elicitation may be used only when the complete, untruncated
`elicitation/create` JSON-RPC request object encoded as UTF-8 JSON plus its one
trailing LF byte is at most 32 KiB. Otherwise that path is reported unavailable
and the adapter uses a negotiated model-invisible local-web surface or CLI in
that order. An agent-visible prompt-capture fallback is not a complete-form
delivery surface. Forms and submissions are never truncated.

Before opening a separately verified User Channel surface, adapters apply one
conservative presentation-safety classification to the question, context
summary, and complete rendered closed form, including every displayed
`EvidenceTarget` and `ArtifactRef` metadata value. When that complete
presentation indicates secret or credential material and requires a user-only
channel, the adapter sends no `elicitation/create` request and emits no rich
prompt-capture question, context, form, verification code, or resolve-command
template. It falls through to negotiated model-invisible local web when
available and otherwise to the CLI inbox. Those user-only local web and CLI surfaces continue to render
the complete canonical form. This classification is conservative adapter
routing, not a general secret scanner, redaction service, isolation boundary,
or guarantee that arbitrary secret material is detected.

Accepting a valid elicitation causes the adapter to invoke the user-only
resolution path with derived `local_user` provenance, a recognized verification
basis, a unique `channel_submission_id`, and
`expected_state_version=null`. Core pins current state during preflight.
Every adapter-generated submission identity is 1 through 256 bytes of visible
ASCII `0x21..=0x7e`; the adapter never truncates or normalizes an invalid value
into that shape.
Decline maps to a stored reject option only for a judgment form that has one.
Cancel, malformed content, an unknown or mixed candidate, a stale form, or a
state conflict records no resolution and leaves the request effectively pending
when it remains current and unexpired.

The MCP result is a compound projection. `agent_workflow_result` is always the
byte-exact agent-safe request response committed by the original Agent
Connection call, and its `operation_result_ref` addresses only that result. The
historical result was created without the complete request or form, so
presentation-safety routing does not need to redact or rewrite it.
`agent_workflow_result_replayed` distinguishes create from explicit resume.
Resume requires the same enabled workflow Agent Connection actor scope and an
allowed project, does not compare later Git workspace coordinates, and is
unavailable for another connection or a reconciliation-created request. It
creates no request, replay row, event, token, prompt, resolution, or state
version and does not update the persisted canonical-UTC floor.

A separate nullable `user_channel_resolution` carries an agent-safe structured
projection of the immutable resolution and current compact facts. Core reads
that resolution, `current_status`, and exact historical `derived_refs` in one
SQLite snapshot identified by `current_projection_state_version` and
`current_projection_observed_at`. The later generic authority refresh may have
a greater state version and does not relabel the projection. Immediate host
resolution must not replace `agent_workflow_result` with the user-only method
result. Free-form note and observation summary remain excluded. The resolution
ref and derived refs retain their original `produced_at_state_version` after
unrelated later commits.

The adapter sends `elicitation/create` only while handling
`request.operation=create` and only after its post-commit reread says the
request is still `pending`. A resume returns the exact historical
`agent_workflow_result` plus the current safe projection without sending
`elicitation/create` or running local-web or CLI fallback,
even when the current status is `pending`. For create, a resolved, stale,
superseded, or expired request returns the current safe projection without
another prompt. Cancelled, declined, or invalid host input during create that
leaves the request pending includes exact nested resume guidance and creates no
second request.

Fallback guidance stays outside Core authority. Unavailable host prompt input
does not hide another available path. If the centralized delivery evaluator
confirms a managed stdio host path, a ready loopback listener, the exact client
declaration, and a current exact-match host-capability verification, a
short-lived local web token is bound to the exact request, form digest,
project, connection, and delivery-surface marker. The raw credential-bearing
URL is placed only in
the following closed top-level handoff; unknown or additional fields are not
allowed:

```json
{
  "_meta": {
    "io.volicord/user-channel": {
      "kind": "local_web_consent",
      "url": "http://127.0.0.1:PORT/consent?...&token=...",
      "expires_at": "RFC3339 UTC timestamp"
    }
  }
}
```

`CallToolResult._meta["io.volicord/user-channel"]` is outside the public tool
`outputSchema`. Agent-visible content reports
only the request ID, pending state, next actor, and safe continuation guidance.
If any eligibility input is unavailable, the adapter issues no token and
points to `volicord inbox`. A pending fallback does not synthesize a second
request or add a structured continuation object outside the closed public
response schema. After User Channel completion, a caller that continues the
workflow uses the public `request.operation=resume` branch with the request ID
from the exact pending summary rather than issuing another create.

<a id="local-web-consent-fallback"></a>
The local web consent listener remains loopback-only and fails closed. `GET
/consent` does not require `Origin`; it validates the one-time token and renders
the exact canonical form. If GET supplies `Origin`, it must be one exact
same-origin header field. `POST /consent` accepts only form fields for that form
and first requires exactly one valid serialized same-origin `Origin` header
field. Missing, empty, `null`, malformed, comma-combined, repeated, and
different `Origin` values fail with HTTP 403 `ORIGIN_NOT_ALLOWED` before
form-body decoding or validation, token lookup or consumption, or resolution
effects. Project, connection, request, form digest, expiry, candidate
membership, and token state are then revalidated. Successful insertion of the
closed resolution body and token consumption are atomic.

Listener startup alone never selects this path. The listener context carries a
shared live-readiness state, and the accept loop makes that state unavailable
before it exits after a listener failure. One adapter evaluator combines that
current state with the exact model-invisible client declaration, managed stdio
launch origin, retained `clientInfo`, and current persisted host-capability
state for invocation derivation, User Channel projection, fallback selection,
and final handoff materialization. The pointed-to immutable verification must
have `outcome=passed`, satisfy `observed_at <= now < expires_at`, and exactly
match the enabled non-generic connection host kind, managed fingerprint,
adapter profile/version, Volicord build, source revision, target and executable
digest, client name/version, and bounded live-host evidence digest.
The expected evidence digest would require trusted production acquisition of
the exact external `volicord-host-release-manifest-v3` owned by
[Host Release Evidence](host-release-evidence.md), binding the same capability,
host/client, adapter, build, source, target, and executable digest.
The row's `evidence_artifact_sha256` must exactly match that expected value;
the row, build descriptor, or copied value cannot self-supply it. Missing,
unknown, malformed, unverified, or mismatched manifest input fails closed. The
current adapter has no trusted acquisition path for that manifest, and the
external release artifact is not itself a runtime trust input, so
production local-web selection remains unavailable and returns CLI fallback.
Manual stdio, CLI verification probes, Local HTTP transport, generic connections, and
invalid or unknown managed markers are ineligible. A passing source revision
is exact lowercase 40- or 64-hex; `unknown` cannot pass. For the built-in stdio
adapter, `host_version == client_version == clientInfo.version`, and the same
version must match the live artifact's installed-host version. If that equality
cannot be proved, the row is not passing. The verification interval must also
satisfy `observed_at <= created_at`,
`observed_at < expires_at <= observed_at + 86,400 seconds`, and
`created_at < expires_at`. Twenty-four hours is a maximum freshness window
rather than a default lifetime or attestation period; a publisher may choose a
shorter expiry. Before issuing a token, the
adapter verifies that the complete
safe tool result plus the closed `_meta` handoff fits the selected 65,536- or
262,144-byte response budget. It then passes the negotiated capability to the
same evaluator and acquires a shared ready-listener issuance lease across token
insertion and handoff construction. Listener invalidation takes the exclusive
side of that lease. This defines one ordering point: if invalidation wins, the
adapter issues no token, omits `_meta`, and returns generic CLI fallback; if an
issuance lease wins, that token is already issued and retains its bounded TTL
even if the listener fails later. A result that cannot fit also falls back
without a token. Budget rejection and readiness invalidation that linearizes
before issuance create no token, and the adapter never truncates the URL. The
same no-token, no-`_meta`, and no project-time-floor effect applies when host-
capability state is absent, non-passing, expired, corrupt, or mismatched. The
handoff is absent on
resume, non-pending results, CLI fallback, token issuance failure, unsupported
or malformed declaration, absent, stale, revoked, corrupt, or mismatched
verification, listener startup failure, degradation that linearizes before
issuance, and response-budget degradation. The URL and token must not
appear in MCP `content`,
`structuredContent`, compatibility or diagnostic text, status or close
projections, exact Core replay, operation-result bytes, logs, or templates. A
host declaration and a matching bounded verification record remain
cooperative integration evidence, not host attestation or proof of host
isolation, user identity, or user authority. A host that cannot preserve this
separation must omit the capability and receives CLI fallback.

The legacy public programmatic builder that accepts only a base URL is an
untracked, source-compatible, fail-closed shim: it does not make local web
available or issue a token. Supported stdio and combined local HTTP process
entry points own a non-cloneable listener guard and configure the adapter
through the shared managed-readiness path. A future external embedder that
needs local web requires a separately owned public listener-lifetime contract;
a caller-supplied base URL or lifetime assertion is not readiness evidence.

For POST resolution, the adapter uses the Core-owned derivation for the only
accepted digest-only `local_web:<sha256>` submission identity. The derivation
binds the exact project, user-action request, raw bearer-token credential,
expected Agent Connection, and typed canonical completion metadata. Core
recomputes that identity at its token-bearing entry point and also binds a
domain-separated token digest, the connection, and the same closed metadata
into mutation replay identity. Only a duplicate with that entire binding and
the canonical resolution returns the original safe completion. A hand-crafted
identity or a changed token, connection, metadata, or resolution does not open
the replay and creates no second effect. The raw token remains transient: the
token table stores its domain-separated hash, while resolution and replay
storage and responses contain only derived digests or hashes. The endpoint
serves no Runtime Home or Product Repository files, static assets, MCP methods,
or arbitrary APIs.

Token issuance uses the project's canonical Core UTC clock in a separate
storage transaction. It stores token `created_at`, derives `expires_at` as
exactly the earlier of the request expiry when present and `created_at + 600
seconds` (or exactly `created_at + 600 seconds` when the request has no
expiry), and
atomically advances the persisted project-time floor to at least `created_at`;
it creates no authority event or replay row and does not increment
`state_version`. GET and POST validation use canonical current project time. A
token is valid only in the half-open interval
`created_at <= now < expires_at`. `now < created_at` is invalid and must not
consume the token; `now >= expires_at` is expired and must not create or consume
a resolution. Persisting an already-derived `expired` token status does not
advance the project-time floor. The full floor contract belongs to
[Storage Versioning](storage-versioning.md#canonical-core-utc-clock).
The token TTL uses checked timestamp addition and must produce canonical RFC
3339 UTC stored strings. Stored `created_at` must not precede the request's
stored `requested_at`. A noncanonical string or any stored expiry other than
the exact derived value is corruption and fails validation, GET, POST, expiry
cleanup, and consumption without effects. Overflow or an unrepresentable
expiry fails before token or floor insertion.

The token's stored creation metadata is the closed object
`{fallback_kind="local_web_consent", delivery_surface="model_invisible_user_surface", endpoint="/consent", form_digest}`.
Missing, extra, malformed, wrong-typed, or mismatched members fail before the
form is rendered or a resolution is attempted. That failure neither consumes
the token nor creates a User Channel effect. This required marker also makes a
pre-correction token fail closed rather than reuse the former Agent-visible
delivery contract.

For known public Volicord method-tool calls that reach Volicord, `tools/call`
wraps the Volicord response JSON inside the MCP result:

- Read-only method results return the Volicord response object in
  `result.structuredContent`. Their compatibility JSON text continues to parse
  equal to that object, except that `volicord.get_operation_result` uses a
  non-JSON summary of at most 512 UTF-8 bytes naming page offsets and completion.
  It never duplicates `chunk_utf8` into compatibility text. Each page contains
  at most 16,384 source UTF-8 bytes and its complete serialized
  `CallToolResult` is at most 65,536 bytes.
- Successful mutation results use the selected receipt projection described
  above. Their `result.content[0].text` is a bounded short summary and is not
  required to parse as JSON.
- Core/domain rejected mutation results retain the public response object in
  `result.structuredContent` and use bounded short compatibility text.
- Clients may validate `structuredContent` against the tool's advertised
  `outputSchema`.
- Successful MCP transport returns `isError: false`, including Volicord
  domain-level rejected responses.
- Volicord domain success or rejection is determined from the parsed Volicord
  response, especially `base.response_kind` and `errors`.
- Parsed public method responses include `base.disclosure` with stable
  `guarantee_class` and `non_guarantees` values from the API schema owners.
- JSON-RPC `error` is reserved for protocol, invalid-parameter, or
  adapter/internal failures; it is not used for Volicord domain-level rejection.

Volicord response branch shapes and error meanings stay with their owners:

- shared response branches: [API Schema Core](api/schema-core.md#common-response)
- response branch routing: [API Error Routing](api/error-routing.md)
- public error codes: [API Error Codes](api/error-codes.md)
- machine-readable error details: [API Error Details](api/error-details.md)

## Shutdown And Reconnection

Closing stdin or terminating the child process ends the MCP session.

Shutdown and reconnection rules:

- SQLite state remains in the Runtime Home.
- Restarting with the same `connection_id` process binding reconnects to the
  same Agent Connection and current registry state.
- Changing connection requires a new process or host configuration update.

Runtime data location boundaries are owned by
[Runtime Boundaries](runtime-boundaries.md), and storage record details are
owned by the storage owners routed from [Storage](storage.md).
