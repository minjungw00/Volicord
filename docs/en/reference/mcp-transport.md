# MCP transport reference

This document owns the local `volicord mcp --stdio` process contract and the
experimental `volicord serve --transport streamable-http` process-boundary
contract: process startup, process environment, MCP protocol-version
negotiation, initialization lifecycle, stdio transport framing, local HTTP MCP
request handling, JSON-RPC message validation, Agent-Connection-bound startup
validation, MCP-visible tool discovery, MCP response wrapping, and
shutdown/reconnection behavior.

It does not define public Volicord API method behavior, public request or
response schemas, Agent Connection meaning, storage record layout, security
guarantees, or Core authority semantics.

## Owns / Does Not Own

This document owns:

- `volicord mcp --stdio` process startup and exit behavior
- `volicord serve --transport streamable-http` startup, local listener, and
  transport-bound security checks
- process configuration used by generated host configuration and exported MCP
  config
- MCP Runtime Home environment resolution
- MCP protocol-version negotiation and initialization lifecycle
- stdio JSON-RPC framing, message validation, and supported MCP methods
- local HTTP JSON-RPC request handling for the experimental serve transport
- server-initiated MCP elicitation at the stdio transport boundary
- local loopback web consent fallback for pending user judgments
- MCP startup validation for one internal Agent Connection binding
- MCP `tools/list` and `tools/call` behavior at the transport boundary
- MCP-visible tool-schema projection that hides internal envelopes and
  invocation metadata
- MCP `tools/call` response wrapping
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
- storage layout, migrations, and storage effects; see the storage owners
  through [Storage](storage.md)

## Process Model

`volicord mcp --stdio` is a local MCP stdio process mode of the installed
`volicord` executable. An MCP host starts it as a child process and communicates
through stdin/stdout. It is not an MCP TCP listener, HTTP MCP listener,
Unix-domain socket listener, or other MCP network listener. It may start a
separate loopback-only local web consent listener for pending user judgments
when MCP elicitation and prompt capture are unavailable.

`volicord serve --transport streamable-http` is a separate explicit process mode
for Docker and localhost MCP use. It starts a local HTTP listener and reuses the
same Agent-Connection-bound MCP adapter logic as stdio where possible. It is not
the default MCP transport, not used by generated local non-Docker host
configuration, and not a general unauthenticated Volicord network service.

The current serve transport is an authenticated experimental Streamable
HTTP-style subset. It accepts JSON-RPC over HTTP `POST /mcp` with MCP session
headers and bearer-token checks, and returns JSON responses. It does not
implement server-sent event streams, HTTP elicitation, or full MCP Streamable
HTTP compatibility. Documentation and startup diagnostics must not claim full
protocol compatibility until those transport features are implemented and
tested.

Generated host configuration and generic exports launch the stdio loop with an
internal connection binding. When the generated entry is safely project-bound,
it also carries the selected internal project binding:

```text
volicord mcp --stdio --connection <connection_id> [--project <project_id>]
```

The `<connection_id>` process-binding value comes from the stored
`connection_internal_id` created by `volicord connect` or the export flow.
The optional `<project_id>` process-binding value is a stored
`project_internal_id` already allowed for that connection. Ordinary users
should not need to type either value in text-mode flows.

Baseline command-line behavior:

- `volicord mcp --stdio --connection <connection_id> [--project <project_id>]`
  launches the stdio loop. When `--project` is present, the supplied value must
  be in the connection's allowlist and the stdio process is narrowed to that
  project before serving tool requests.
- `volicord mcp --check --connection <connection_id>` runs startup validation
  without reading stdin.
- `volicord mcp --check --connection <connection_id> --project <project_id>`
  runs the same startup validation and limits project-detail diagnostics to
  one allowed `project_internal_id` value.
- `-h` and `--help` print usage and environment summary, then exit with code
  `0`.
- `-V` and `--version` print `volicord <version>`, then exit with code `0`.
- No mode, `--check` or `--stdio` without `--connection`, unknown options,
  combined command-line modes, missing required option values, and extra
  positional arguments write usage diagnostics to stderr and exit with code
  `2`.
- Help and version handling happen before Runtime Home or Agent Connection
  lookup.

Experimental HTTP serve command-line behavior:

- `volicord serve --transport streamable-http` is the only supported serve
  transport spelling. Other transport values are usage errors.
- `--listen 127.0.0.1:<port>` selects the listener. Omission uses
  `127.0.0.1:8765`.
- The default listener is loopback-only. Binding `0.0.0.0`, `::`, or another
  non-loopback address requires `--allow-nonlocal-listen` and writes a clear
  warning at startup.
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
- `--token TOKEN` supplies the bearer token for this process. If omitted,
  Volicord generates a process-local token and writes it to stderr during
  startup. Tokens are not stored in repository files.
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
  implemented by this experimental endpoint.
- `GET /healthz` is a minimal local health endpoint, but it still requires the
  same bearer token.
- `GET /consent` and `POST /consent` are local web consent endpoints only when
  local web consent is available. They are not MCP endpoints and do not use the
  MCP bearer token. They require a valid one-time consent token tied to the
  project, connection, and pending judgment.
- There are no unauthenticated arbitrary resource endpoints.
- CORS preflight is accepted only for the MCP endpoint, only after Origin
  allowlist validation, and only when at least one allowed Origin is configured.
- Structured HTTP errors use stable transport error codes for authentication,
  Origin, project allowlist, unsupported transport, unsupported method, and
  unsupported content negotiation failures.

Session-watch startup coverage:

- When stdio startup is project-bound by `--project <project_id>` or by a
  connection context with exactly one available allowed project, the process
  creates or attaches a session-watch baseline before serving tool requests
  whenever bounded snapshot creation is available. The coverage basis is
  `mcp_start`.
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

Supported optional environment input:

- `VOLICORD_HOME`
- `VOLICORD_LOCAL_WEB_CONSENT`

`VOLICORD_HOME` selects the Runtime Home for the process. It is normally written
by generated host configuration when needed, not typed by the user in ordinary
flows. It does not select a project, connection intent, actor provenance,
operation category, connection mode, or host trust state. The stdio process and
`--check` use `VOLICORD_HOME` before entering startup validation. Help and
version modes do not use it.

`VOLICORD_LOCAL_WEB_CONSENT=0`, `false`, `off`, or `disabled` disables the
stdio local web consent listener. Other values do not change the listener
address or token policy.

Connection process binding is supplied by `--connection <connection_id>` in
generated host configuration or generic export output. It names the stored
`connection_internal_id` for the selected Agent Connection and is not a normal
user-chosen value. The bound Agent Connection and Runtime Home registry state
supply the connection mode, connected projects, and adapter-derived `actor_source` and
`operation_category`. Project access is controlled by the selected Agent
Connection's connected projects and repository-root resolution. No other
process environment input is interpreted by the MCP process.

Current MCP Runtime Home resolution:

1. A present but empty `VOLICORD_HOME` is an error.
2. An absolute `VOLICORD_HOME` is used as supplied.
3. A relative `VOLICORD_HOME` is resolved against the process current working
   directory without requiring the path to exist.
4. When `VOLICORD_HOME` is absent, use the Runtime Home established by
   `volicord init` or `volicord setup`, or the platform default local runtime
   location.
5. Do not require canonicalization before startup validation.

## Startup Validation

Before entering the stdio loop, `volicord mcp --stdio` validates the Agent
Connection binding and the local registry records it depends on.

Startup validation requires:

- the Runtime Home registry exists and is valid
- the configured `connection_id` process argument names an existing stored
  `connection_internal_id`
- the connection is enabled
- the connection mode is supported
- at least one connected project row is readable
- the installation profile can resolve the MCP command information needed for
  diagnostics
- registry JSON and metadata needed for startup are valid

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

- one `connection_id` process binding for a stored Agent Connection

The Agent Connection supplies:

- one connection mode: `workflow` or `read_only`
- one connection intent: `personal`, `shared`, or `global`
- an explicit allowlist of connected projects
- host configuration inventory and last verification state through the registry

The process binding remains fixed for the process lifetime. Changing the Agent
Connection identity requires another process or host configuration update.
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
runtime_home: <absolute path>
connection_id: <connection_internal_id process-binding value>
mode: workflow|read_only
enabled: true|false
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
| Lifecycle violation on a request, including a request before `initialize`, `tools/list` or `tools/call` before the ready state, or duplicate `initialize` | JSON-RPC `-32600` Invalid Request |
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
| Waiting for `notifications/initialized` | `notifications/initialized` notification; `ping` request | `notifications/initialized` completes the transition to ready. `ping` may be used after `initialize` has succeeded, including while the server waits for the notification. |
| Ready | `ping`, `tools/list`, `tools/call` | Normal MCP tool discovery and tool execution are available. |

`tools/list` and `tools/call` are available only after
`notifications/initialized` has completed the ready transition. A duplicate
`initialize` request is invalid. An early or malformed
`notifications/initialized` notification does not make the connection ready.

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

After the connection is ready, `tools/list` exposes tools according to the
current stored Agent Connection mode:

| Mode | MCP-visible tools |
|---|---|
| `workflow` | `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.prepare_write`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_judgment`, `volicord.reconcile_changes`, `volicord.check_close`, `volicord.close_task`, `volicord.list_projects` |
| `read_only` | `volicord.status`, `volicord.check_close`, `volicord.list_projects` |

The MCP-visible tools are not the same thing as the public Volicord Core API
method list. `volicord.check_close` is the read-only MCP tool for close
readiness and internally calls the Core close-readiness check path.
`volicord.close_task` is the workflow-only MCP mutation tool and is not listed
for `read_only` connections. `volicord.record_user_judgment` is a public Core
API method for the User Channel path, but it is not exposed as an Agent
Connection MCP tool; see [API Methods](api/methods.md) for the public method
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

For a known public Volicord method tool, object `arguments` that fail the tool
input schema return a `CallToolResult` with `isError: true` and actionable text
content. They are tool execution errors, not JSON-RPC protocol errors.

For a public Volicord method-tool call, the adapter first performs deterministic
repository-root project selection and per-project validation owned by
[Agent Connection](agent-connection.md#current-connection-context). Ambiguous or
unavailable project selection is rejected before Core execution and the
actionable text must name the `volicord project use` or `volicord connect`
command needed to repair the state.

`volicord mcp --stdio` does not advertise or implement MCP task-augmented tool
execution. A `tools/call` request does not return `CreateTaskResult`, and a
`task` parameter is not a supported baseline feature.

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
  `resolved_verification_basis=mcp_elicitation_user_channel`. The returned
  `tools/call` content contains the resulting Volicord response JSON.
- If the elicitation response is `action=decline` and the pending judgment has
  a Core reject option, the adapter records that reject option through the same
  User Channel path. If no reject option exists, the judgment remains pending.
- If the elicitation response is `action=cancel`, invalid, malformed, or cannot
  be matched to the pending judgment, the adapter records no answer and the
  pending judgment remains pending.
- If elicitation is unavailable because the client did not declare the
  capability, the adapter records no answer and returns the pending
  `RequestUserJudgmentResult` plus additional text content. When prompt-capture
  availability is `configured`, `observed`, or `active`, that text may include
  exact chat prompt-capture commands compatible with the prompt-submit hook path
  and the current verification code.
- If prompt capture is unavailable and local web consent is available, the
  adapter creates a short-lived one-time token and returns a loopback consent
  URL plus structured fallback JSON. The URL contains only the project selector
  and token. It does not include the Runtime Home path, repository path, prompt
  body, answer, or arbitrary API parameters.
- If local web consent is disabled, cannot bind safely, or cannot create a
  token, the fallback text points to the `volicord user` local CLI recovery
  path.

For all branches, `result.content[0].text` remains the Volicord response JSON
string. Additional `content[]` text, when present, is adapter guidance such as
fallback instructions or an explanation that elicitation was cancelled or
invalid. The additional text is not Core authority, not a public API response
field, and not a user judgment record.

The local web consent listener binds to `127.0.0.1` by default and must fail
closed if it cannot bind safely. In stdio mode it uses an ephemeral loopback
port. In `volicord serve --transport streamable-http`, local web consent is
available only when the actual serve listener is loopback; an explicitly
non-local serve listener must not expose the consent form.

Local web consent endpoint behavior:

- `GET /consent?project=<project_id>&token=<token>` validates the one-time token
  against the current project and connection, rejects expired, consumed,
  invalid, wrong-project, and wrong-connection tokens with a safe HTML error
  page, and otherwise renders a minimal HTML page with the judgment text,
  options, verification facts, and a form.
- `POST /consent` accepts only
  `application/x-www-form-urlencoded` form submissions with the token, selected
  Core option ID, and optional note. If an `Origin` header is present, it must
  match the consent endpoint origin.
- A successful post consumes the token exactly once and records the answer
  through Core with `actor_source=local_user`, `operation_category=user_only`,
  and `resolved_verification_basis=local_user_local_web`.
- Replay, expiration, consumed token reuse, wrong project, and wrong connection
  are rejected before recording another answer.
- The endpoint serves no Runtime Home files, product repository files, static
  assets, MCP methods, or arbitrary APIs.

For known public Volicord method-tool calls that reach Volicord, `tools/call`
wraps the Volicord response JSON inside the MCP result:

- Volicord response JSON is serialized as the string in
  `result.content[0].text`.
- Clients must parse that string as JSON to inspect the Volicord response.
- Successful MCP transport returns `isError: false`, including Volicord
  domain-level rejected responses.
- Volicord domain success or rejection is determined from the parsed Volicord
  response, especially `base.response_kind` and `errors`.
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
