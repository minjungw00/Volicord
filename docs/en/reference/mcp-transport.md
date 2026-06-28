# MCP transport reference

This document owns the local `volicord-mcp` process contract: process startup, process environment, MCP protocol-version negotiation, initialization lifecycle, stdio transport framing, JSON-RPC message validation, Agent-Connection-bound startup validation, MCP response wrapping, and shutdown/reconnection behavior.

It does not define public Volicord API method behavior, public request or response schemas, Agent Connection meaning, storage record layout, security guarantees, or Core authority semantics.

## Owns / Does Not Own

This document owns:

- `volicord-mcp` process startup and exit behavior
- required and optional process configuration for Agent-Connection-bound startup
- MCP Runtime Home environment resolution
- MCP protocol-version negotiation and initialization lifecycle
- stdio JSON-RPC framing, message validation, and supported MCP methods
- MCP startup validation for one `connection_id`
- MCP `tools/list`, `tools/call`, and `volicord.list_projects` adapter utility behavior at the transport boundary
- MCP `tools/call` response wrapping
- process shutdown and reconnection behavior

This document does not own:

- the public Volicord method list or method owner table; see [API Methods](api/methods.md)
- public Volicord request and response schemas; see [API Schema Core](api/schema-core.md)
- Agent Connection, Connection Projects, project selection meaning, current connection context, and actor provenance; see [Agent Connection](agent-connection.md)
- administrative Runtime Home, Agent Connection setup, project membership, and verification commands; see [Administrative CLI](admin-cli.md)
- storage layout, migrations, and storage effects; see the storage owners through [Storage](storage.md)

## Process Model

`volicord-mcp` is a local MCP stdio process. An MCP host starts it as a child process and communicates through stdin/stdout. It is not a TCP listener, HTTP listener, Unix-domain socket listener, or other network listener.

Baseline command-line behavior:

- Launch the stdio loop with `volicord-mcp --connection <connection_id>`.
- Run startup validation without reading stdin with `volicord-mcp --check --connection <connection_id>`.
- Run project-specific startup validation with `volicord-mcp --check --connection <connection_id> --project <project_id>`.
- `-h` and `--help` print usage and environment summary, then exit with code `0`.
- `-V` and `--version` print `volicord-mcp <version>`, then exit with code `0`.
- No arguments, `--check` without `--connection`, unknown options, combined command-line modes, missing required option values, invalid `--project` use, and extra positional arguments write usage diagnostics to stderr and exit with code `2`.
- Help and version handling happen before Runtime Home or Agent Connection lookup.

Exit and stream behavior:

- Normal stdin EOF shutdown flushes stdout and exits with code `0`.
- Successful `--check` writes its report to stdout and exits with code `0`.
- Startup configuration, JSON, or storage failures write diagnostics to stderr and exit with code `1`.
- Once the stdio loop is running, malformed JSON and unsupported JSON-RPC requests return JSON-RPC errors when a response can be written.

## Process Environment

Supported optional environment input:

- `VOLICORD_HOME`

`VOLICORD_HOME` is the only supported MCP process environment input. It selects the Runtime Home for the process; it does not select a project, connection, actor provenance, operation category, or connection mode. The stdio process and `--check` use `VOLICORD_HOME` before entering startup validation. Help and version modes do not use it.

Connection identity is supplied by `--connection <connection_id>`. The bound Agent Connection and Runtime Home registry state supply the connection mode, connected Projects, and adapter-derived `actor_source` and `operation_category`. Project access is controlled by the selected Agent Connection's connected Projects in Runtime Home registry state. The selected project is determined per public MCP tool call. No other process environment input is interpreted by the MCP process.

Current MCP Runtime Home resolution:

1. A present but empty `VOLICORD_HOME` is an error.
2. An absolute `VOLICORD_HOME` is used as supplied.
3. A relative `VOLICORD_HOME` is resolved against the process current working directory without requiring the path to exist.
4. When `VOLICORD_HOME` is absent, use the first non-empty home source in this order: `HOME`, `USERPROFILE`, then `HOMEDRIVE` plus `HOMEPATH`.
5. Append `.volicord` to the selected user home.
6. Resolve a relative selected home against the process current working directory.
7. Do not require canonicalization before startup validation.

## Startup Validation

Before entering the stdio loop, `volicord-mcp` validates the Agent Connection binding and the local registry records it depends on.

Startup validation requires:

- the Runtime Home registry exists and is valid
- the configured `connection_id` exists
- the connection is enabled
- the connection mode is supported
- the connected project rows are readable
- registry JSON and metadata needed for startup are valid

Startup validation does not select one project for all calls. Project availability, project status, path separation, and mode compatibility are verified per call as defined by [Agent Connection](agent-connection.md#current-connection-context).

A stored Agent Connection can remain after it reaches zero connected projects. That persistence is not startup eligibility: a new stdio process and `volicord-mcp --check --connection <connection_id>` fail startup validation while there are no connected projects.

An already running process is different from a new process. A process that passed startup while at least one project was connected refreshes registry state for `volicord.list_projects` and project routing. After the last membership is removed, `volicord.list_projects` may return an empty project list, but public tools that require project routing reject because no connected project remains.

## Agent-Connection-Bound Process

One `volicord-mcp` process is bound to:

- one `connection_id`

The Agent Connection supplies:

- one connection mode: `read_only` or `workflow`
- an explicit allowlist of connected projects
- host configuration inventory and last verification state through the registry

The process binding remains fixed for the process lifetime. Changing the Agent Connection identity requires another process or host configuration update. Changing project membership, mode, or enabled state takes effect through registry state; each new process reruns startup validation against the current registry state.

MCP call arguments and other MCP request bodies cannot set connection identity, `actor_source`, `operation_category`, or connection mode.

## Configuration Preflight

`volicord-mcp --check --connection <connection_id>` runs the same Runtime Home, Agent Connection, membership, and registry-shape startup validation used before entering the stdio loop. `volicord-mcp --check --connection <connection_id> --project <project_id>` limits the project detail section to one project and rejects a project that is outside the selected Agent Connection's allowlist. Neither form reads stdin.

On success, `--check` writes fixed summary lines, then one repeated project-detail block for each selected project, in this order:

```text
configuration: valid
transport: stdio
runtime_home: <absolute path>
connection_id: <value>
mode: read_only|workflow
enabled: true
allowed_projects: <count>
available_projects: <count>
verification_scope: startup_check_only
project[0].project_id: <value>
project[0].available: true|false
project[0].unavailable_reason: <value or empty>
project[0].repo_root: <path>
```

Project-detail rules:

- The detail index begins at zero.
- Without `--project`, one detail block is emitted for each connected project in stable `project_id` order.
- `--project <project_id>` rejects a project that is not connected to the Agent Connection and limits the detail block selection to that one project.
- `allowed_projects` describes the Agent Connection as a whole. With `--project`, `available_projects` describes the emitted detail selection and is therefore `0` or `1`.
- Unavailable projects still emit every project-detail key. `unavailable_reason` is populated for unavailable projects and empty for available projects.
- `verification_scope: startup_check_only` is a startup and preflight statement only, not complete host verification.

Startup validation failure:

- writes a diagnostic to stderr through the process entry point
- exits with code `1`
- does not enter the stdio loop or wait on stdin

A successful `--check` is not a complete host connection result. Complete host verification requires durable Agent Connection state, host configuration installation, successful MCP initialization, and successful tool discovery, as defined by [Administrative CLI](admin-cli.md#agent-connection-result-states).

## MCP Wire Behavior

`volicord-mcp` supports MCP protocol version `2025-11-25` over stdio. It does not advertise simultaneous compatibility with older MCP protocol versions. Each new process or stdio connection starts a new MCP lifecycle and must complete its own initialization sequence.

The server initialization response includes MCP server instructions. Those instructions may describe Volicord tool selection, deterministic project routing, and limitations, but they are guidance only; they are not access control or a guarantee of model behavior.

### Framing And JSON-RPC Validation

Framing rules:

- Each non-empty stdin line contains exactly one UTF-8 JSON-RPC message object.
- The JSON root must be one JSON-RPC message object. For the Volicord client-to-server baseline, the supported message objects are requests and the `notifications/initialized` notification. Arrays, primitive JSON roots, and `null` are invalid MCP stdio messages.
- JSON-RPC batches are not supported. An array input receives one Invalid Request response, not one response per array element.
- Messages are delimited by newlines and must not contain embedded newlines.
- Each output line contains one JSON-RPC response object. `volicord-mcp` writes no readiness message before `initialize`.
- Stdin EOF ends the process after stdout is flushed.

JSON-RPC validation rules:

- `jsonrpc` must be exactly `"2.0"`.
- A request `method` must be a string.
- Request IDs may be strings or integers and must not be `null`.
- A classifiable notification has a string `method`, no `id`, and receives no response even when its MCP method parameters are malformed.
- An object without an `id` is not automatically a valid notification; it must still satisfy the notification shape.
- For supported MCP requests, method `params`, when present, must be an object. For lifecycle notifications, absent or object `params` are the only shapes that can affect lifecycle.

Notification classification is based on the JSON-RPC envelope before MCP method-parameter validation. Once a message is classifiable as a notification, malformed `params` do not produce any JSON-RPC response. Those `params` are still invalid for lifecycle purposes: a malformed `notifications/initialized` does not move the connection to ready, and request-only methods received as notifications are ignored and must not execute.

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

The first valid MCP request in a connection is `initialize`. A valid `initialize` request has object `params` with:

- `protocolVersion` as a string
- `capabilities` as an object
- `clientInfo` as an object containing string `name` and `version` fields

Additional MCP `Implementation` metadata allowed by the 2025-11-25 schema, such as `title`, `description`, `icons`, or `websiteUrl`, may be accepted but is not required in examples.

Protocol-version negotiation:

- If the client requests `2025-11-25`, `volicord-mcp` returns `2025-11-25`.
- If the client sends another syntactically valid protocol-version string, `volicord-mcp` returns the version it supports: `2025-11-25`.
- The server response does not claim simultaneous compatibility with older MCP protocol versions.

Lifecycle states:

| Connection point | Valid client messages | Result |
|---|---|---|
| Before successful `initialize` | `initialize` request | On success, the server returns `protocolVersion: "2025-11-25"` and waits for `notifications/initialized`. |
| Waiting for `notifications/initialized` | `notifications/initialized` notification; `ping` request | `notifications/initialized` completes the transition to ready. `ping` may be used after `initialize` has succeeded, including while the server waits for the notification. |
| Ready | `ping`, `tools/list`, `tools/call` | Normal MCP tool discovery and tool execution are available. |

`tools/list` and `tools/call` are available only after `notifications/initialized` has completed the ready transition. A duplicate `initialize` request is invalid. An early or malformed `notifications/initialized` notification does not make the connection ready.

Supported MCP request methods:

- `initialize`
- `ping`
- `tools/list`
- `tools/call`

The supported lifecycle notification is `notifications/initialized`.

## Tool Discovery And `tools/call` Response Wrapping

After the connection is ready, `tools/list` exposes tools according to the bound Agent Connection mode:

| Mode | MCP method tools | MCP adapter utility tools |
|---|---|---|
| `read_only` | 2: `volicord.status`, `volicord.close_task` | `volicord.list_projects` |
| `workflow` | 8: `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.prepare_write`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_judgment`, `volicord.close_task` | `volicord.list_projects` |

The MCP method-tool counts above are not the same as the public Volicord Core API method list. `volicord.list_projects` is an MCP adapter utility, not a public Volicord Core API method. `volicord.record_user_judgment` is a public Core API method for the User Channel path, but it is not exposed as an Agent Connection MCP tool; see [API Methods](api/methods.md) for the public method owner table.

A structurally valid `tools/call` request has object `params` with:

- `name` as a string
- optional `arguments` as an object

Missing `arguments` are treated as an empty object. `arguments: null` and non-object `arguments` are malformed method parameters and return JSON-RPC `-32602`. Unknown tool names are protocol errors and return JSON-RPC `-32602`.

For public Volicord method tools, `tools/list` exposes MCP-visible input schemas derived from the shared Volicord request schemas with the Agent Connection binding applied. `envelope.project_id` remains an optional caller selector. `envelope.actor_source`, `envelope.operation_category`, `envelope.connection_id`, and `envelope.verification_basis` are not exposed in the MCP-visible schema and are not accepted in raw `tools/call` arguments. If raw public method-tool arguments include caller-owned invocation fields at the top level or inside `envelope`, the adapter rejects the call before Core execution.

For a known public Volicord method tool, object `arguments` that fail the tool input schema return a `CallToolResult` with `isError: true` and actionable text content. They are tool execution errors, not JSON-RPC protocol errors.

For `volicord.list_projects`, the adapter returns a read-only project list for the bound Agent Connection only. It must not enter Core, create storage effects, mutate project membership, or expose projects outside the connection allowlist. If a connected project has an invalid current registration, the adapter fails the utility call instead of returning that project as a normal available or unavailable entry.

For a public Volicord method-tool call, the adapter first performs deterministic project selection and per-project validation owned by [Agent Connection](agent-connection.md#current-connection-context). Ambiguous project selection is rejected before Core execution and the actionable text must instruct the agent to call `volicord.list_projects`.

`volicord-mcp` does not advertise or implement MCP task-augmented tool execution. A `tools/call` request does not return `CreateTaskResult`, and a `task` parameter is not a supported baseline feature.

For known public Volicord method-tool calls that reach Volicord, `tools/call` wraps the Volicord response JSON inside the MCP result:

- Volicord response JSON is serialized as the string in `result.content[0].text`.
- Clients must parse that string as JSON to inspect the Volicord response.
- Successful MCP transport returns `isError: false`, including Volicord domain-level rejected responses.
- Volicord domain success or rejection is determined from the parsed Volicord response, especially `base.response_kind` and `errors`.
- JSON-RPC `error` is reserved for protocol, invalid-parameter, or adapter/internal failures; it is not used for Volicord domain-level rejection.

Volicord response branch shapes and error meanings stay with their owners:

- shared response branches: [API Schema Core](api/schema-core.md#common-response)
- response branch routing: [API Error Routing](api/error-routing.md)
- public error codes: [API Error Codes](api/error-codes.md)
- machine-readable error details: [API Error Details](api/error-details.md)

## Shutdown And Reconnection

Closing stdin or terminating the child process ends the MCP session.

Shutdown and reconnection rules:

- SQLite state remains in the Runtime Home.
- Restarting with the same `connection_id` reconnects to the same Agent Connection and current registry state.
- Changing connection requires a new process or host configuration update.

Runtime data location boundaries are owned by [Runtime Boundaries](runtime-boundaries.md), and storage record details are owned by the storage owners routed from [Storage](storage.md).
