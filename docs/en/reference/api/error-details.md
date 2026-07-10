# API error details

This document owns machine-readable `ToolError.details` semantics, detail fields, helper values, and detail constraints for Volicord API errors.

Use it for diagnostic keys and helper values under `ToolError.details`. Use adjacent owners for branch routing, public code meaning, schema shape, and display wording.

## Owner boundaries

Owned here:

- Semantics for known `ToolError.details` fields and nested detail keys.
- Helper values used under `ToolError.details`.
- Constraints that keep machine-readable details separate from display wording and sensitive request bodies.

Adjacent owners:

- The `ToolError` shape; see [API Schema Core](schema-core.md#shared-support-shapes).
- Public `ErrorCode` values and meanings; see [API error codes](error-codes.md).
- Primary-code precedence and conflict selection; see [API error precedence](error-precedence.md).
- API response branch routing; see [API error routing](error-routing.md).
- Close-readiness blocker routing; see [API blocker routing](blocker-routing.md).
- Display wording only; see [Template Bodies](../template-bodies.md).
- Storage effects; see [Storage Effects](../storage-effects.md).

<a id="machine-readable-error-details"></a>

## Machine-readable detail constraints

`ToolError.details` is machine-readable diagnostic data. It is not display wording and does not replace the public `ToolError.code`.

Detail keys and helper values are exact identifiers.

Condition:
- Detail keys and helper values may be reused as blocker codes only when an owning method or schema explicitly allows that exact use.

Required behavior:
- Preserve detail keys and helper values as machine-readable identifiers.

Not allowed:
- Do not localize detail keys or helper values.
- Do not render them as user-facing display wording.
- Do not reuse them as blocker codes without owning method or schema support.

Detail data must stay limited to stable diagnostic facts. It must not expose sensitive request bodies, duplicate method payloads, raw stored JSON, secrets, SQL text, sensitive absolute paths, or define storage effects.

<a id="state-conflict-detail-fields"></a>

## State conflict detail fields

Stale `expected_state_version` details:
- Include `state_clock: project_state.state_version`, `current_state_version`, `expected_state_version`, `project_id`, and `task_id` when available.

Stale write-ticket basis details:
- Identify both the stale `WriteTicket.basis_state_version` value and the current `project_state.state_version` value.

Idempotency request-hash conflict details:
- Identify the `idempotency_key` and request-hash mismatch without exposing sensitive request bodies.

<a id="owner-state-corruption-detail-fields"></a>

## Owner-state corruption detail fields

When corrupt typed owner state is reported through the structured store/runtime-unavailability rejection, details may identify:

- `owner_state_error.table`
- `owner_state_error.record_ref`
- `owner_state_error.logical_column`
- `owner_state_error.corruption_category`

These diagnostics must not include raw stored JSON, secrets, SQL text, or sensitive absolute paths. They do not make malformed JSON equivalent to absence.

<a id="error-detail-helper-values"></a>

## Error detail helper values

<a id="authorization-reason"></a>

### `write_ticket_reason`

`ToolError.details.write_ticket_reason` uses:

```text
missing
expired
stale
revoked
consumed
incompatible
task_mismatch
change_unit_mismatch
product_write_flag_mismatch
baseline_mismatch
sensitive_category_mismatch
path_mismatch
```

The mismatch-specific values identify the incompatible record or scope fact
while keeping the public code `WRITE_TICKET_INVALID`.

Expired write-ticket use sets `write_ticket_reason=expired` with public code
`WRITE_TICKET_INVALID`. A stale `WriteTicket.basis_state_version` uses
`STATE_VERSION_CONFLICT`, not `WRITE_TICKET_INVALID`.

<a id="artifact-input-error-reason"></a>

### `artifact_input_error.reason`

`ToolError.details.artifact_input_error.reason` uses these detail helper values. They are not top-level public `ErrorCode` values; staged-handle validation failures keep the public code `VALIDATION_FAILED` unless the actual failure is a request-level invocation-context, actor-source, or Product Repository path-boundary mismatch.

| `artifact_input_error.reason` | Meaning |
|---|---|
| `staged_handle_expired` | The staged handle is past its usable lifetime. |
| `staged_handle_consumed` | The staged handle was already consumed. |
| `staged_handle_project_mismatch` | The staged handle belongs to a different project. |
| `staged_handle_task_mismatch` | The staged handle belongs to a different Task. |
| `staged_handle_actor_source_mismatch` | The staged handle provenance does not match the verified actor source. |
| `staged_handle_checksum_mismatch` | The staged bytes do not match the expected checksum. |
| `staged_handle_size_mismatch` | The staged bytes do not match the expected size. |
| `staged_handle_not_found` | The staged handle cannot be found. |
