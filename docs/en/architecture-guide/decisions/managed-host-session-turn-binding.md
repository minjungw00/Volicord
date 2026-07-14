# Managed-host session/thread binding and per-call turn validation

## Context

A generated Codex MCP descriptor can show that Volicord owns the launch shape,
but it does not identify the native Codex session or thread that issued a tool
call. Ambient environment variables are not forwarded uniformly to MCP and
hook children, and process ancestry, PID, event order, or temporal proximity
cannot establish an exact cross-surface correlation. Binding at process startup
would therefore either invent an identity or associate durable observations
with the wrong host session.

Codex `0.144.4` supplies authoritative cooperative correlation metadata on each
MCP tool call: flat `_meta.threadId` and nested
`_meta["x-codex-turn-metadata"]` values `session_id`, `thread_id`, and
`turn_id`. Its MCP initialize identity is exactly
`clientInfo.name=codex-mcp-client` and `clientInfo.version=0.144.4`.

## Decision

Volicord treats descriptor or managed-marker validation as launch provenance
only. A managed Codex stdio process starts in a pending-binding state. Pending
state creates no diagnostic session, session-watch baseline, managed lifecycle
row, Core effect, token, or local-web eligibility.

After exact initialize identity and the ready transition, the first
structurally valid call to a known tool may bind the process. The adapter first
validates JSON-RPC shape, tool name, and `arguments`, then requires:

- string `_meta.threadId`;
- object `_meta["x-codex-turn-metadata"]` with string `session_id`,
  `thread_id`, and `turn_id`;
- exact equality between the flat `threadId` and nested `thread_id`; and
- every native value to be 1 through 256 UTF-8 bytes matching
  `[A-Za-z0-9._:-]+`.

The adapter maps `session_id` through the domain-separated
`volicord-managed-host-session-v1` function and derives a separate
domain-separated in-memory digest for `thread_id`. It discards the raw session,
thread, and turn values after validation and hashing. The mapped session and
thread digest are immutable for that stdio process. Later calls must match
both; a new turn may use a different valid `turn_id`.

The first successful binding starts bounded session-watch coverage and
materializes the process lifecycle facts observed up to that point. Coverage
is explicitly partial and starts at binding; materialized startup,
initialization, or tools-list facts do not claim observation of Product
Repository changes before the baseline. A missing, malformed, session-mismatched,
or thread-mismatched call returns JSON-RPC `-32602` with zero durable, Core,
tool-invocation, token, and local-web effects. There is no rebind path.

Local `diagnostics.sqlite` persistence is best effort and non-authoritative.
Corruption, write denial, or a pre-existing conflicting diagnostic coordinate
cannot reject an otherwise valid binding or alter MCP, guard, or Core results.
The adapter skips or reports that diagnostic failure nonfatally where possible;
authoritative ownership conflicts are decided from project Agent Session and
registered connection state. Invalid or mismatched request metadata still
follows the zero-effect rejection above.

This metadata is a cooperative local correlation input, not user identity,
authority, host attestation, same-principal anti-forgery, or proof of host
isolation. The exact wire behavior remains owned by
[MCP Transport](../../reference/mcp-transport.md), and the opaque mapping and
release evidence rules remain owned by
[Host Release Evidence](../../reference/host-release-evidence.md).

## Consequences

- A repository descriptor can be valid while the process is still unbound.
- Managed lifecycle and Strong Evidence use one exact opaque root-session
  coordinate across MCP and hook paths without retaining raw host IDs.
- Separate Codex threads can map to the same root session while each stdio
  process keeps its own exact thread binding.
- Retrying in a later turn is valid when session and thread still match.
- Startup coverage honestly reports the pre-binding gap instead of backdating
  watcher coverage.
- Diagnostic-store availability or coordinate collisions cannot become a
  second managed-session authority source.

## Compatibility and migration

This tightens the managed Codex transport path for the reviewed `0.144.4`
client. A managed call without the required metadata no longer receives a
synthetic managed session. There is no environment fallback, timing
rendezvous, legacy alias, or migration of earlier observations. Compatible
observations are recreated through a new bound host session.

The change adds no public Core API method, public MCP tool argument, public
tool schema field, SQLite DDL, or storage-profile version. Request-side
`_meta` remains hidden transport metadata.

## Rejected alternatives

- Treating the descriptor or managed markers as session identity was rejected
  because they prove only launch provenance.
- Using `CODEX_THREAD_ID` or another ambient environment variable was rejected
  because it is not an authoritative per-call MCP and hook channel.
- Pairing the newest hook and MCP events by PID, parent process, time window,
  arrival order, or nearest session was rejected because concurrent sessions
  make the result ambiguous.
- Persisting raw session, thread, turn, event, or invocation identifiers was
  rejected because correlation needs only domain-separated opaque values.
- Allowing a later call to rebind the process was rejected because it would
  mix observations from different native sessions or threads.

## Relevant implementation areas and owners

- [`crates/volicord-mcp`](../../../../crates/volicord-mcp): pending/bound stdio
  state, initialize retention, request metadata validation, and deferred
  lifecycle materialization.
- Shared managed-host session types: opaque session and thread digest mapping.
- [Agent Connection](../../reference/agent-connection.md)
- [MCP Transport](../../reference/mcp-transport.md)
- [Host Release Evidence](../../reference/host-release-evidence.md)
- [Security](../../reference/security.md)
