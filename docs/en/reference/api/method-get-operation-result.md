<a id="volicordget_operation_result"></a>

# `volicord.get_operation_result` reference

## Owned here

This page owns the baseline behavior of `volicord.get_operation_result`:

- read-only retrieval of one exact historical Core mutation response
- fixed-size paging and opaque cursor behavior
- method-specific access, unavailable-result, and no-effect rules
- the top-level request and result fields

Shared `OperationResultRef` and page-coordinate field shapes are owned by
[API Schema Core](schema-core.md#operation-result-retrieval). Exact replay-row
storage is owned by [Storage Versioning](../storage-versioning.md#exact-operation-result-retrieval).

## Purpose

An MCP mutation projection can omit an exact result to stay within its response
budget after the mutation has committed. Every eligible agent-workflow Core
commit and exact replay therefore exposes an `operation_result_ref` that locates
the immutable response already stored for idempotent replay.

`volicord.get_operation_result` reads that historical JSON text in bounded
pages. Concatenating `chunk_utf8` values in cursor order reproduces the stored
response JSON byte-for-byte. Retrieval does not replay the mutation, recompute
the response, or claim that historical state is current.

## Request

```yaml
GetOperationResultRequest:
  envelope: ToolEnvelope
  operation_result_ref: OperationResultRef
  cursor: string | null
```

Rules:

- `envelope.project_id` must equal `operation_result_ref.project_id`.
- The verified invocation uses `operation_category=read`, `dry_run=false`,
  `idempotency_key=null`, and `expected_state_version=null`.
- `cursor=null` selects the first page. A non-null cursor is an opaque value
  returned by the preceding page and must be echoed without interpretation.
- A cursor is bound to the complete reference, response checksum, and next byte
  offset. Malformed, altered, or cross-result cursors are rejected without
  returning a response fragment.

## Access requirements

The method rechecks the currently enabled Agent Connection and Connection
Projects membership for every page. The selected project and current verified
`actor_source` must match the stored agent-workflow invocation. Possession of a
reference or cursor alone grants no access.

User-only results, including the exact `volicord.record_user_judgment` body and
free-form user note, are not exposed to an Agent Connection. A host-mediated
Judgment flow keeps the original agent-owned
`volicord.request_user_judgment` reference. Its exact-result lookup therefore
reconstructs the original pending response. The separately owned MCP outcome
projection may report the selection, but it never substitutes the user-only
reference or exposes the user note or exact user-only response body.

## Result

```yaml
GetOperationResultResult:
  base: ToolResultBase
  operation_result_ref: OperationResultRef
  start_offset_bytes: integer
  end_offset_bytes: integer
  chunk_utf8: string
  next_cursor: string | null
  complete: boolean
  historical: true
  current_authority_refresh_required: true
```

Successful pages use `base.response_kind=result`,
`base.effect_kind=read_only`, and at most 16,384 source UTF-8 bytes in
`chunk_utf8`. Page boundaries never split a UTF-8 code point.
`start_offset_bytes=0` for the first page, and each returned
`end_offset_bytes` is the next page's start. `complete=true` if and only if
`next_cursor=null` and the page reaches
`operation_result_ref.response_size_bytes`.

The method verifies the stored byte length and SHA-256 value against the
reference before returning any page. A missing result, integrity mismatch,
invalid bound cursor, or unavailable eligible row returns
`OPERATION_RESULT_UNAVAILABLE`. Actor or project-context incompatibility returns
`INVOCATION_CONTEXT_MISMATCH`. Malformed request or cursor syntax returns
`VALIDATION_FAILED`. Store reachability or corrupt owner state follows the
normal `MCP_UNAVAILABLE` boundary. No failure returns partial historical bytes.

## State and authority effects

Retrieval is read-only. It creates no event, replay row, Task or Change Unit
change, artifact effect, write-ticket effect, or `project_state.state_version`
increment. Later state changes do not change the immutable historical bytes or
invalidate a well-formed cursor, but current authority must be read separately
with `volicord.status`.

`OperationResultRef` is a lookup locator, not a mutation retry credential,
`StateRecordRef`, `ArtifactRef`, `AuthorityReceipt`, write ticket, evidence, or
authorization token.

## Non-Core staging boundary

`volicord.stage_artifact` creates transient staging outside the normal Core
commit/replay transaction and therefore has no `OperationResultRef`. Before it
creates staging state, Core must prove that its complete serialized result is
within the owner-defined staging-result bound. Its compact MCP result preserves
every actionable staging field, including the handle and expiry. A result that
would exceed the pre-effect bound is rejected before any staged handle or bytes
are created.

## Related owners

- [MCP Transport](../mcp-transport.md#mutation-authority-receipt-projection)
- [Security](../security.md#historical-operation-result-access)
- [Agent Connection](../agent-connection.md#operation-result-retrieval)
- [Storage Records](../storage-records.md)
- [Storage Effects](../storage-effects.md)
- [Storage Versioning](../storage-versioning.md#exact-operation-result-retrieval)
