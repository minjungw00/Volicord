# Durable operation-result retrieval

## Context

An Agent Connection mutation can commit successfully while the MCP response
budget prevents the exact public method result from fitting in the returned
projection. A compact result preserves the next actionable fields, but it is
not a durable replacement for the exact historical response. An effect anchor
correlates an effect and cannot identify or authorize retrieval of response
bytes.

Normal committed Core mutations already store their exact serialized response
in the immutable `tool_invocations.response_json` replay row. Reusing that row
avoids a second result store, a storage-profile change, and disagreement between
replay and retrieval.

## Decision

Volicord exposes the additive stable read-only method
`volicord.get_operation_result`. Eligible committed and replayed operation
projections carry an `OperationResultRef` derived from the existing immutable
replay row. Core validates the current invocation and the reference, Store
loads the matching replay response, and Core returns fixed bounded UTF-8 JSON
text pages. Concatenating the decoded page chunks in cursor order reproduces
the stored response bytes exactly.

The lookup keeps these implementation boundaries:

- `OperationResultRef` is a content-bound locator. Its method, idempotency key,
  committed state version, byte size, and SHA-256 facts must all match the
  selected replay row.
- A cursor is opaque and bound to the complete result reference and next byte
  offset. Page boundaries preserve UTF-8 code points, and every page is
  integrity-checked before any response bytes are returned.
- Every page repeats current project-access and originating-actor checks. A
  reference, cursor, effect anchor, or copied connection identifier is not a
  bearer credential.
- The retrieved body is historical and non-authoritative. Callers use
  `volicord.status` for current authority rather than treating an old method
  response as current state.
- MCP retains the original agent-owned
  `volicord.request_user_action` result reference when a host-mediated User
  Channel answer completes the call. It pairs that reference with the safe
  compact selected outcome and never exposes the user-only
  `volicord.resolve_user_action` result or free-form note to the Agent
  Connection.
- `volicord.stage_artifact` remains outside the replay-row path. Before staging
  creates bytes or a handle, it must serialize and bound the complete result;
  its compact MCP result retains every actionable staging field.

The exact public method behavior, schema fields, page bound, errors, storage
effects, and security rules remain in the focused Reference owners. This
decision records only the durable implementation direction.

## Consequences

- Exact recovery and idempotent replay read the same immutable response body.
- Retrieval reads only the current immutable replay row. It does not decode,
  convert, or rewrite another storage contract.
- Core owns access and integrity decisions; Store owns the scoped replay-row
  read; MCP owns projection and retention of the result reference.
- Retrieval is read-only and does not replay the mutation, append an authority
  event, create another replay row, or advance `project_state.state_version`.
- Corrupt, missing, malformed, cross-result, or access-incompatible reads fail
  closed without returning a partial body.
- Adding a stable public method and MCP tool is a minor public-surface change;
  the recommended release version is `0.6.0`.

## Non-goals

- This decision does not make historical responses current Core authority.
- It does not make an `OperationResultRef` or cursor an authorization token.
- It does not expose user-only judgment notes to an Agent Connection.
- It does not turn transient artifact staging into a normal Core commit or
  replay row.
- It does not define a general artifact-body, event-body, or Runtime Home file
  download API.

## Rejected alternatives

- Requiring every compact mutation result to contain all exact response detail
  was rejected because compact projections have a different, bounded purpose
  and cannot preserve arbitrary exact historical data.
- Reusing `effect_anchor` as a lookup credential was rejected because an effect
  correlation value neither identifies exact response bytes nor grants access.
- Replaying the mutation to rebuild a result was rejected because current state
  may differ and a read must not repeat effects.
- Returning the raw response as one unpaged body was rejected because it
  recreates the response-budget failure and weakens bounded transport behavior.
- Adding a duplicate operation-result table was rejected because
  `tool_invocations.response_json` is already the exact immutable replay source.
- Storing the response as an artifact shortcut was rejected because artifacts
  have different ownership, lifecycle, retention, and authority semantics.

## Relevant implementation

- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs):
  common invocation validation and public method dispatch boundaries.
- [`crates/volicord-store/src/core_pipeline.rs`](../../../../crates/volicord-store/src/core_pipeline.rs):
  replay-row persistence and exact response retrieval boundary.
- [`crates/volicord-mcp/src/stdio.rs`](../../../../crates/volicord-mcp/src/stdio.rs):
  canonical mutation projection, bounded recovery, and MCP response wrapping.
- [`crates/volicord-mcp/src/tool_registry.rs`](../../../../crates/volicord-mcp/src/tool_registry.rs):
  public MCP tool metadata and discovery.

## Related tests and Reference owners

Tests should cover exact multi-page reconstruction, UTF-8 boundaries, stable
replay references, missing and corrupt rows, cross-result cursors, actor and
connection isolation, no partial failure body, host-mediated judgment privacy,
and the pre-effect staging bound.

Contract owners are [`volicord.get_operation_result`](../../reference/api/method-get-operation-result.md),
[API Schema Core](../../reference/api/schema-core.md),
[MCP Transport](../../reference/mcp-transport.md),
[Agent Connection](../../reference/agent-connection.md),
[Security](../../reference/security.md), [Storage Records](../../reference/storage-records.md),
[Storage Effects](../../reference/storage-effects.md), and
[Storage Versioning](../../reference/storage-versioning.md).
