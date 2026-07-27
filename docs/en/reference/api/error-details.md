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
- Product-wide failure-category meanings; see the
  [Failure Model](../failure-model.md).

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

<a id="platform-diagnostic-detail-field"></a>

## Platform diagnostic detail field

A typed platform-boundary Store failure has this exact detail shape:

```yaml
diagnostic_code: string
```

`diagnostic_code` is the canonical namespaced `platform.*` code selected by the
typed platform diagnostic kind. Unsupported platform kinds route through
`VALIDATION_FAILED`; unavailable-observation kinds route through
`MCP_UNAVAILABLE`. Other Store failures use `store_failure_category` for their
Store-owned classification.

<a id="state-conflict-detail-fields"></a>

## State conflict detail fields

Stale `expected_state_version` details:
- Include `state_clock: project_state.state_version`, `current_state_version`, `expected_state_version`, `project_id`, and `task_id` when available.

`WriteTicket.basis_state_version` is not a state-conflict detail field. It is
audit ordering metadata and its mismatch alone produces no error.

Idempotency request-hash conflict details:
- Identify the `idempotency_key` and request-hash mismatch without exposing sensitive request bodies.

<a id="owner-state-corruption-detail-fields"></a>

## Owner-state corruption detail fields

When corrupt typed owner state is reported with
`code=PERSISTED_DATA_CORRUPT` and `category=corrupt`, details may identify:

- `owner_state_error.table`
- `owner_state_error.record_ref`
- `owner_state_error.logical_column`
- `owner_state_error.corruption_category`

These diagnostics must not include raw stored JSON, secrets, SQL text, or sensitive absolute paths. They do not make malformed JSON equivalent to absence.

<a id="error-detail-helper-values"></a>

## Error detail helper values

<a id="reason"></a>

### `reason`

`ToolError.details.reason` is an exact domain-specific identifier. The
following code and domain combinations require the listed value:

| Public code and domain | `ToolError.category` | `details.reason` | Meaning owner |
|---|---|---|---|
| `NO_ACTIVE_CHANGE_UNIT` from `volicord.prepare_write` | `rejected` | `current_change_unit_required` | [`volicord.prepare_write`](method-prepare-write.md) |

The reason narrows the domain cause; it does not replace or change the required
failure category or public code. These values are not display text, fallback
selectors, aliases, or permission to decode a different contract. Other detail
families use their named nested fields, such as `write_ticket_reason` and
`artifact_input_error.reason`, rather than overloading this field.

<a id="authorization-reason"></a>

### `write_ticket_reason`

`ToolError.details.write_ticket_reason` uses:

<a id="write-ticket-reason"></a>

```text
missing
revoked
consumed
incompatible
task_mismatch
change_unit_mismatch
scope_revision_changed
change_unit_changed
baseline_changed
workspace_changed
approval_basis_changed
idle_timeout
task_closed
explicit_revoke
product_write_flag_mismatch
baseline_mismatch
operation_mismatch
workspace_mismatch
approval_basis_mismatch
policy_authority_mismatch
sensitive_category_mismatch
path_mismatch
```

The `*_changed`, `idle_timeout`, `task_closed`, and `explicit_revoke` values are
stable recorded invalidation reasons. The `*_mismatch`, `consumed`, `revoked`,
and `incompatible` values identify attempt-time incompatibility. They keep the
public code `WRITE_TICKET_INVALID`. A global `basis_state_version` mismatch has
no helper value because it is not invalidity.
`policy_authority_mismatch` means the active ticket binding is missing or does
not equal the current normalized project write-authority fingerprint.

<a id="artifact-input-error-reason"></a>

### `artifact_input_error.reason`

`ToolError.details.artifact_input_error.reason` uses these detail helper values. They are not top-level public `ErrorCode` values; staged-handle validation failures keep the public code `VALIDATION_FAILED` unless the actual failure is a request-level invocation-context, actor-source, or Product Repository path-boundary mismatch.

| `artifact_input_error.reason` | Meaning |
|---|---|
| `staged_handle_expired` | The staged handle is past its usable lifetime. |
| `staged_handle_consumed` | The staged handle was already consumed. |
| `staged_handle_project_mismatch` | The staged handle belongs to a different project. |
| `staged_handle_task_mismatch` | The staged handle belongs to a different `Task`. |
| `staged_handle_actor_source_mismatch` | The staged handle provenance does not match the verified actor source. |
| `staged_handle_checksum_mismatch` | The staged bytes do not match the expected checksum. |
| `staged_handle_size_mismatch` | The staged bytes do not match the expected size. |
| `staged_handle_not_found` | The staged handle cannot be found. |
