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
- local loopback web consent fallback for pending user judgments
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
separate loopback-only local web consent listener for pending user judgments
when host prompt input and chat command capture are unavailable.

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

Generated shared project configuration uses neither binding ID nor local
environment. Its complete process descriptor is one of:

```text
volicord mcp --stdio --discover-repository --host codex
volicord mcp --stdio --discover-repository --host claude-code
```

The shared command must be the PATH-resolved name `volicord`; an absolute
command, extra connection/project arguments, Runtime Home, managed-launch
markers, secret-like environment keys, and every other environment entry are
invalid. Codex verification treats any deviation from this exact project
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
- `--home PATH` selects the Runtime Home for the process. Without `--home`, the
  shared `VOLICORD_HOME` and platform default Runtime Home resolution apply.
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
  judgment.
- There are no unauthenticated arbitrary resource endpoints.
- Browser-facing requests are identified by the presence of an `Origin` header.
  MCP endpoint requests with `Origin` must match an exact `--allow-origin`
  value, and local web consent form submissions with `Origin` must match the
  consent endpoint's own origin.
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

- When stdio startup is project-bound by `--project <project_id>` or by a
  connection context with exactly one available allowed project, the process
  creates or attaches a session-watch baseline before serving tool requests
  whenever bounded snapshot creation is available. The coverage basis is
  `mcp_start`.
- For validated generated Codex launches with managed provenance markers, the
  stdio process also appends managed lifecycle metadata to that baseline for
  `managed_host_startup`, `managed_host_initialize_response`,
  `managed_host_tools_list`, and `managed_host_tool_call` observations when
  writable storage is available. Each lifecycle event records the selected
  connection and project, `host_kind=codex`, `launch_origin=managed_host`, a
  timestamp, observed storage capability, and effective tool mode when those
  values are available.
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
- standard platform home variables when `VOLICORD_HOME` is absent: `HOME`,
  `USERPROFILE`, and the `HOMEDRIVE` plus `HOMEPATH` pair

`VOLICORD_HOME` selects the Runtime Home for the process. A personal, local, or
user-wide host overlay may write it when needed. A shared repository-visible
Codex or Claude Code MCP entry must have no environment map, so discovery mode
receives Runtime Home selection only from the host process's inherited local
environment or the platform default. `VOLICORD_HOME` does not select a project,
connection intent, actor provenance, operation category, connection mode, or
host trust state. The stdio process and `--check` use it before entering startup
validation. Help and version modes do not use it.

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
these markers. The markers and descriptor grant neither project access, host
trust, nor broader authority.

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
2. An absolute `VOLICORD_HOME` is used as supplied.
3. A relative `VOLICORD_HOME` is resolved against the process current working
   directory without requiring the path to exist.
4. When `VOLICORD_HOME` is absent, derive the default user home from the
   platform home variables and append `.volicord`. Non-Windows platforms try
   `HOME`, then `USERPROFILE`, then `HOMEDRIVE` plus `HOMEPATH`. Native Windows
   tries `USERPROFILE`, then `HOMEDRIVE` plus `HOMEPATH`, then `HOME` when it is
   not a WSL-style mount path.
5. Do not require canonicalization before startup validation.

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

1. Canonicalize the process current directory and walk its ancestors to the
   nearest valid Git worktree root, including supported gitdir-file and linked
   worktree layouts.
2. Require that exact canonical root to be a project registered in the selected
   local Runtime Home.
3. Select enabled connections whose host matches `--host`, whose intent is
   `shared`, whose host scope is project, and whose Connection Projects contain
   that project.
4. Require exactly one match and narrow the process allowlist to that project.

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
- `clientInfo` as an object containing string `name` and `version` fields

If `params.capabilities.elicitation` is an object, the adapter treats the MCP
client as eligible for server-initiated elicitation. Other capability entries
do not create Volicord behavior by themselves.

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
contents.

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
`volicord.request_user_judgment`. That request is server-initiated MCP
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
| `workflow` with writable project state | `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.get_operation_result`, `volicord.prepare_write`, `volicord.prepare_evidence_capture`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_judgment`, `volicord.reconcile_changes`, `volicord.check_close`, `volicord.close_task`, `volicord.list_projects` |
| `workflow` with readable but non-writable project state | `volicord.status`, `volicord.get_operation_result`, `volicord.check_close`, `volicord.list_projects` |
| `read_only` with readable project state | `volicord.status`, `volicord.get_operation_result`, `volicord.check_close`, `volicord.list_projects` |
| No readable allowed project state | `volicord.list_projects` |

The MCP adapter may inspect project state read-only during startup and
discovery. If project state is readable but not writable in the current MCP
host environment, read-compatible method tools remain visible and workflow
mutation tools are withheld even when the stored Agent Connection mode is
`workflow`. If no allowed project state can be read, the adapter keeps only
`volicord.list_projects` visible so the caller can inspect project
availability.

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
`volicord.record_user_judgment` and `volicord.record_user_observation` are
public Core API methods for User Channel paths, but neither is exposed as an
Agent Connection MCP tool; see
[API Methods](api/methods.md) for the public method
owner table.

A structurally valid `tools/call` request has object `params` with:

- `name` as a string
- optional `arguments` as an object

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
- `volicord.request_user_judgment`: `change_unit_id=null`,
  `sensitive_action_scope=null`, `options=null`, `affected_refs=[]`, and
  `expires_at=null`

Every MCP-visible mutation tool also accepts `detail=summary|workflow|full`.
Omitted `detail` defaults to `summary`. This is an adapter response-projection
choice, not a Core request field and not permission to omit any method-owned
request member.

These defaults belong only to the MCP-visible argument DTO. After decoding, the
adapter constructs the complete Core request shape. They do not change the
public Core API present-member contract owned by the focused method references.
For `volicord.request_user_judgment`, `task_id`, `judgment_kind`,
`presentation`, `question`, `context`, and `required_for` remain required MCP
arguments. For `volicord.record_run`, `target` and `coverage_state` remain
required inside each `evidence_updates` item, while `target`, `source_kind`,
`assurance_level`, and `observed_at` remain required inside each
`evidence_observations` item. Each `target` is the strict tagged
acceptance-criterion or supplemental-claim union owned by the API state schema.
This rule supplies no implicit value
for any other field; the exact advertised `required` array remains
authoritative.

Tool descriptions contain only a short purpose and key boundary. Because
`volicord.record_run.kind` compatibility depends on the current persisted Task
rather than another visible MCP argument, that tool's description includes the
exhaustive mode-to-kind matrix: `advisor` uses `shaping_update`, `direct` uses
`direct`, and `work` uses `shaping_update` or `implementation`. Frequently used
argument-shape examples are advertised as values in `inputSchema.examples`,
including intake create/resume/supersede/reject, update-scope
keep/create/replace, all three status detail levels, first-page operation-result
retrieval, prepare-write, all three prepare-evidence-capture variants,
stage-artifact, an advisor `shaping_update` with no Product Repository write, an
evidence-bearing work `implementation`, request-judgment, reconcile,
check-close, and close complete/cancel/supersede branches. Each advertised
example conforms to the same `inputSchema` and MCP argument DTO used for calls.
Examples illustrate supported argument branches only; they do not assert
matching project state, authority, preconditions, or a successful Core result.

Every listed Volicord tool also exposes an MCP 2025-11-25 `outputSchema` whose
root type is `object`. Read-only public method tools derive that schema from
their public method response branches. Mutation tools additionally advertise
summary and workflow wrappers that pair a fresh `AuthorityReceipt` with the
method result needed for the next step, a full wrapper that pairs the same
fresh receipt with the exact public method response, and bounded post-effect
recovery branches. The
`volicord.request_user_judgment` full branch also covers the User Channel
response returned when host elicitation records the pending judgment before the
original tool call completes. `volicord.list_projects` uses its exact
adapter-utility result schema. A server result that includes
`structuredContent` must conform to the advertised schema.

`tools/list` supplies the following conservative MCP `annotations`:

| Tool class | `readOnlyHint` | `destructiveHint` | `idempotentHint` | `openWorldHint` |
|---|---:|---:|---:|---:|
| `volicord.status`, `volicord.get_operation_result`, `volicord.check_close`, `volicord.list_projects` | `true` | `false` | `true` | `false` |
| `volicord.prepare_write`, `volicord.prepare_evidence_capture`, `volicord.stage_artifact` | `false` | `false` | `false` | `false` |
| `volicord.intake`, `volicord.update_scope`, `volicord.record_run`, `volicord.request_user_judgment`, `volicord.reconcile_changes`, `volicord.close_task` | `false` | `true` | `false` | `false` |

For the non-destructive mutation row, `destructiveHint=false` means the tool's
committed storage updates are additive rather than replacing, invalidating, or
consuming existing authority state. It does not mean that the call is read-only
or that a later distinct MCP call is replay-safe.

`volicord.record_run` uses `destructiveHint=true` because a commit may consume a
compatible write ticket or staged input, update evidence and blockers,
increment `close_basis_revision`, invalidate current judgments, and replace the
current close basis or leave a previous basis stale. A committed
`volicord.request_user_judgment` may also change the Task lifecycle while it
creates the pending judgment. The method and storage-effect owners define the
exact effects; the annotation conservatively tells MCP clients that these tools
can alter existing authority state.

All mutation tools have `idempotentHint=false` because each distinct
MCP-visible call receives fresh adapter-managed request identity. Core replay
handling for one generated identity does not promise that a later visible MCP
call has the same result or no additional effects.

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

The MCP adapter generates the Core envelope before dispatch. It supplies
`request_id`, `idempotency_key` for workflow effects, `expected_state_version`
from the selected project's current state where Core freshness requires it,
`dry_run=false`, the default locale, the selected internal project, and the
derived invocation context. Public MCP arguments cannot override those facts.

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

The compact `method_result` always preserves effect kind, resulting state
version, and committed event refs. It additionally preserves the issued write
ticket and decision for `volicord.prepare_write`, the exact capture-intent ref,
intent, and expiry for `volicord.prepare_evidence_capture`, the staged handle and expiry
for `volicord.stage_artifact`, the exact Run ref, registered `ArtifactRef`
values, newly recorded evidence-observation refs, and nullable
`close_basis_anchor` for `volicord.record_run`, per-finding results for
`volicord.reconcile_changes`, and the pending or resolved outcome for
`volicord.request_user_judgment`. `close_basis_anchor` contains
`close_basis_revision`, `scope_revision`, `source_run_ref`, and nullable
`evidence_summary_ref`. It is a typed coordinate for the close basis stored on
the Task, not a `StateRecordRef` and not a separate close-basis record. A
resolved compact Judgment outcome contains the Judgment ref, status, selected
option ID and label, and resolution outcome; it omits the free-form user note.
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
host User Channel adapter after the pending judgment was created;
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

<a id="user-judgment-elicitation"></a>
### User Judgment Elicitation

`volicord.request_user_judgment` remains the only Agent Connection tool for
asking Core to create a focused pending `UserJudgment`. The MCP adapter does
not expose `volicord.record_user_judgment` as an Agent Connection tool and does
not accept agent-supplied answer fields as substitutes for user input.

When a `workflow` connection calls `volicord.request_user_judgment` and Core
commits a pending judgment:

- If the initialized client declared `capabilities.elicitation`, the adapter
  may send `elicitation/create` before returning the original `tools/call`
  response. The requested schema is a flat object with required
  `selected_option_id` drawn from the Core-created option IDs and optional
  `note`. It does not request secrets, credentials, tokens, private keys, or
  other private secret material.
- If the elicitation response is `action=accept`, the adapter validates
  `content.selected_option_id` against the pending judgment options. A valid
  response is recorded through Core's User Channel method with
  `actor_source=local_user`, `operation_category=user_only`, and
  `resolved_verification_basis=mcp_elicitation_user_channel`. The adapter then
  applies the selected mutation `detail` projection to the fresh Task status.
- If the elicitation response is `action=decline` and the pending judgment has
  a Core reject option, the adapter records that reject option through the same
  User Channel path. If no reject option exists, the judgment remains pending.
- If the elicitation response is `action=cancel`, invalid, malformed, or cannot
  be matched to the pending judgment, the adapter records no answer and the
  pending judgment remains pending.
- If host prompt input is unavailable because the client did not declare the
  capability, the adapter records no answer and returns the pending
  `RequestUserJudgmentResult` plus additional text content. When chat command
  capture availability is `configured`, `observed`, or `active`, that text may
  include exact verification-code chat commands compatible with the
  prompt-submit hook path and the current verification code.
- If chat command capture is unavailable and a local consent URL is available,
  the adapter creates a short-lived one-time token and returns a loopback
  consent URL plus structured fallback JSON. The URL contains only the project
  selector and token. It does not include the Runtime Home path, repository
  path, prompt body, answer, or arbitrary API parameters.
- If the local consent URL path is disabled, cannot bind safely, or cannot
  create a token, the fallback text points to the `volicord inbox` CLI inbox
  path.

For all successful branches, `result.structuredContent` follows the selected
mutation `detail` projection. When host elicitation resolves the judgment, every
projection preserves the original agent-owned
`volicord.request_user_judgment` `operation_result_ref`; it never substitutes a
reference to the user-only recording operation. `detail=full` pairs an
agent-safe recorded public response projection with the fresh authority
receipt. The projection retains the selected outcome but sets or keeps the
free-form user note null. The exact user-only response remains stored for its
owner and is not retrievable through the Agent Connection. The default
`summary` pairs that receipt with a compact Judgment result containing the Judgment ref,
status, and, after resolution, the selected option ID and label plus resolution
outcome. The compact result does not include the free-form note.
`result.content[0].text` remains a short compatibility summary, not a JSON
duplicate. Additional `content[]` text, when present, is adapter guidance
such as fallback instructions or an explanation that elicitation was cancelled
or invalid. The additional text is not part of `structuredContent`, not Core
authority, not a public API response field, and not a user judgment record.

<a id="local-web-consent-fallback"></a>
The local web consent listener binds to `127.0.0.1` by default and must fail
closed if it cannot bind safely. In stdio mode it uses an ephemeral loopback
port. In `volicord serve --transport local-http`, the consent route is served
only by the same loopback-only local HTTP listener.

Local web consent endpoint behavior:

- `GET /consent?project=<project_id>&token=<token>` validates the one-time token
  against the current project and connection, rejects expired, consumed,
  invalid, wrong-project, and wrong-connection tokens with a safe HTML error
  page, and otherwise renders a minimal HTML page with the judgment text,
  available options and their meanings, project name or identifier, registered
  repository path when available, connection identifier, judgment id, token
  expiry, fallback CLI command, and a form. The page states that the user is
  recording a user-owned judgment, that the agent cannot record it on the
  user's behalf, and that the judgment does not prove correctness, test
  sufficiency, deployment success, review completion, security enforcement, or
  close readiness.
- `POST /consent` accepts only
  `application/x-www-form-urlencoded` form submissions with the token, selected
  Core option ID, and optional note. If an `Origin` header is present, it must
  match the consent endpoint origin. A validation failure, including an unknown
  option ID, does not consume the token.
- A successful post records the answer through Core with
  `actor_source=local_user`, `operation_category=user_only`, and
  `resolved_verification_basis=local_user_local_web`, and marks the token
  `consumed` in the same project-state transaction or an equivalent atomic
  operation.
- Expiration, wrong project, wrong connection, wrong judgment binding, and
  consumed token reuse are rejected before recording another answer. A duplicate
  submit after a successful post returns the consumed-token result
  deterministically and does not change the recorded judgment. A write failure
  while recording the judgment leaves the token pending until it expires, as
  long as the pending judgment remains current.
- Local web consent captures a human user's answer. It must not be used to let
  an Agent Connection answer user-owned judgments.
- The endpoint serves no Runtime Home files, product repository files, static
  assets, MCP methods, or arbitrary APIs.

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
