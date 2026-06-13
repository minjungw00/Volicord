# API error details

This document owns machine-readable `ToolError.details` semantics, detail fields, helper values, and detail constraints for Harness API errors.

It does not define the `ToolError` object shape, public `ErrorCode` meanings, precedence selection, blocker routing, display labels, or storage effects.

## Owner boundaries

This document owns:

- Semantics for known `ToolError.details` fields and nested detail keys.
- Helper values used under `ToolError.details`.
- Constraints that keep machine-readable details separate from display labels and sensitive request bodies.

This document does not own:

- The `ToolError` shape; see [API Schema Core](schema-core.md#shared-support-shapes).
- Public `ErrorCode` values and meanings; see [API error codes](error-codes.md).
- Primary-code precedence and conflict selection; see [API error precedence](error-precedence.md).
- Error versus blocker routing; see [API error routing](error-routing.md).
- Rendered labels and message wording; see [Template Bodies](../template-bodies.md).

<a id="machine-readable-error-details"></a>

## Machine-readable detail constraints

`ToolError.details` is machine-readable diagnostic data. It is not display text and does not replace the public `ToolError.code`.

Detail keys and helper values are exact identifiers. They must not be localized, rendered as user-facing labels, or reused as blocker codes unless an owning method or schema explicitly allows that use.

Detail data must stay limited to stable diagnostic facts. It must not expose sensitive request bodies, duplicate method payloads, or define storage effects.

<a id="state-conflict-detail-fields"></a>

## State conflict detail fields

Stale `expected_state_version` details:
- Include `state_clock: project_state.state_version`, `current_state_version`, `expected_state_version`, `project_id`, and `task_id` when available.

Stale Write Authorization basis details:
- Identify the stale authorization basis and current `project_state.state_version`.

Idempotency request-hash conflict details:
- Identify the `idempotency_key` and request-hash mismatch without exposing sensitive request bodies.

<a id="error-detail-helper-values"></a>

## Error detail helper values

<a id="authorization-reason"></a>

### `authorization_reason`

`ToolError.details.authorization_reason` uses `missing`, `expired`, `stale`, `revoked`, `consumed`, or `incompatible`. A stale `WriteAuthorization.basis_state_version` uses `STATE_VERSION_CONFLICT`, not `WRITE_AUTHORIZATION_INVALID`.

<a id="artifact-input-error-reason"></a>

### `artifact_input_error.reason`

`ToolError.details.artifact_input_error.reason` uses these detail helper values. They are not top-level public `ErrorCode` values; staged-handle validation failures keep the public code `VALIDATION_FAILED` unless the actual failure is request-level local access or capability verification.

| `artifact_input_error.reason` | Meaning |
|---|---|
| `staged_handle_expired` | The staged handle is past its usable lifetime. |
| `staged_handle_consumed` | The staged handle was already consumed. |
| `staged_handle_project_mismatch` | The staged handle belongs to a different project. |
| `staged_handle_task_mismatch` | The staged handle belongs to a different Task. |
| `staged_handle_surface_mismatch` | The staged handle provenance does not match the verified surface. |
| `staged_handle_checksum_mismatch` | The staged bytes do not match the expected checksum. |
| `staged_handle_size_mismatch` | The staged bytes do not match the expected size. |
| `staged_handle_not_found` | The staged handle cannot be found. |
