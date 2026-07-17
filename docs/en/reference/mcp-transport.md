# MCP transport reference

This document owns the first-release local MCP process boundary: managed stdio
startup, strict binding, JSON-RPC lifecycle, tool discovery, public argument
projection, response wrapping, and shutdown. Core methods, Codex configuration,
storage effects, and release evidence remain with their focused owners.

<a id="surface-stability"></a>
## Surface Stability

Labels use [Documentation Policy](../maintain/documentation-policy.md#surface-stability-labels).

| Surface | Stability |
|---|---|
| `volicord mcp --stdio`, initialization, `tools/list`, `tools/call`, and response wrapping | `stable` |
| Pre-1.0 additions not listed in the stable process and method set | `beta` |
| Process-binding values and generated configuration details | `internal` |
| Startup and protocol diagnostics | `diagnostic` |

## Process Model

`volicord mcp --stdio` is a child process started by managed Codex
configuration. It exchanges line-delimited JSON-RPC through stdin and stdout
and opens no TCP, HTTP, Unix-domain socket, or other network listener.

```text
volicord mcp --stdio --connection <connection_id> [--project <project_id>]
volicord mcp --stdio --discover-repository --host codex
volicord mcp --check --connection <connection_id> [--project <project_id>]
```

The bound form uses exact stored identifiers. Repository discovery is only for
the canonical shared Codex binding and resolves identity from the exact Runtime
Home and canonical Git work tree. It does not infer a connection from cwd
alone, scan nearby repositories, or accept another host selector. `--check`
performs preflight without entering the stdio loop.

## Environment And Startup

`VOLICORD_HOME` selects the Runtime Home according to
[Runtime Boundaries](runtime-boundaries.md). Shared configuration forwards the
value without embedding a machine-local path.

Before reading MCP requests, the adapter validates the current
`ExternalContractDescriptor`, `ManagedHostBinding`, selected connection,
allowed projects, Runtime Home/Product Repository separation, exact
`StorageManifest`, and required storage readability. Unknown descriptors,
unsupported artifacts, corrupt records, ambiguous selection, and unavailable
storage use the [Failure Model](failure-model.md). Startup never probes another
format, fills a missing field, or starts a different transport.

## MCP Wire Behavior

Each non-empty stdin line is one complete UTF-8 JSON-RPC 2.0 request. Malformed
JSON returns `-32700`; invalid requests return `-32600`; unknown methods return
`-32601`; invalid parameters return `-32602`; and internal protocol failures
return `-32603`. Responses preserve the request `id`.

`initialize` precedes `tools/list` and `tools/call`. The process negotiates only
the supported MCP protocol version and then accepts
`notifications/initialized`. Calls before initialization, repeated initialize,
batch input, and unsupported versions fail before Core.

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

EOF closes the loop after in-flight response handling. A new process repeats
startup validation and MCP initialization; it inherits no connection, project,
receipt, or current state from the previous process.

## Related Owners

- [Agent Connection](agent-connection.md)
- [Administrative CLI](admin-cli.md)
- [API Methods](api/methods.md)
- [API User-Action Schemas](api/schema-user-action.md)
- [Storage Effects](storage-effects.md)
- [Security](security.md)
