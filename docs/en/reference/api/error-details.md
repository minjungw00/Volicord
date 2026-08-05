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
- Descriptor-derived MCP argument issues and their branch-local detail fields;
  see [MCP Transport](../mcp-transport.md#public-argument-projection). They are
  not `ToolError.details`.

<a id="machine-readable-error-details"></a>

## Machine-readable detail constraints

`ToolError.details` is machine-readable diagnostic data. It is not display wording and does not replace the public `ToolError.code`.

Every `ToolError` contains the `details` field. Its value is `null` when no
machine-readable detail object is available; otherwise it is an object
containing the owner-defined diagnostic facts described below. The enclosing
`ToolError` object is closed. Exact requiredness, nullability, and field types
come from the generated structure in [API Schema Core](schema-core.md#common-response).

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

The fields of `McpToolErrorIssue` and `McpToolErrorResponse` belong to the MCP
transport carrier. They do not populate `ToolError.details`, select an API
response branch, or turn a pre-Core schema-contract failure into a public API
error.

<a id="workflow-rejection-detail-fields"></a>
## Workflow rejection detail fields

`RUN_KIND_INCOMPATIBLE`, `TASK_PHASE_TRANSITION_REQUIRED`,
`SHAPING_CHECKPOINT_REQUIRED`, `SHAPING_CHECKPOINT_STALE`,
`USER_DECISION_UNRESOLVED`, `CHANGE_UNIT_REQUIRED`, `CHANGE_UNIT_STALE`,
`WORKSPACE_BASIS_STALE`, and `WORKFLOW_ACTION_NOT_ALLOWED` require this one
closed non-null detail shape:

```schema
TransitionRejection:
  attempted_action_key: WorkflowActionKey
  reason: TransitionRejectionReason
  state_change_applied: false
  retryable: boolean
  recovery_action_key: WorkflowActionKey | null
  incompatible_submitted_paths: string[]
  baseline_compatibility: BaselineTransitionCompatibility | null
  blocking_refs: StateRecordRef[]
  current_workflow_kind: WorkflowStateKind

BaselineTransitionCompatibility:
  current_baseline_canonical: boolean
  submitted_baseline_canonical: boolean
  submitted_baseline_matches_current: boolean
  submitted_baseline_compatible_with_transition: boolean
```

`attempted_action_key` preserves the exact method and semantic variant that
failed admission. `reason` is one of `action_not_current`,
`variant_not_current`, `authority_basis_mismatch`,
`implementation_authority_would_be_invalidated`, `user_authority_missing`,
`checkpoint_stale`, `change_unit_stale`, `workspace_basis_stale`, or
`close_precondition_missing`. `blocking_refs` is canonical, duplicate-free
current authority context.

`recovery_action_key` is non-null only when that exact key is a member of the
same current `WorkflowTransitionCatalog`; Core never fabricates a method-only
or message-derived recovery. `retryable` repeats the semantic retry fact used
by `ToolError.retryable`; neither permits replaying an applied mutation. The
enclosing rejection remains `no_effect`.

`incompatible_submitted_paths` contains only Core-identified request paths.
`baseline_compatibility` is non-null only when Core assessed baseline
compatibility for this exact transition. Canonicality of the current and
submitted values, equality with current authority, and compatibility with the
selected transition remain four independent facts. MCP and other renderers copy
them and do not infer them from `field`, message text, or a method name.

<a id="platform-diagnostic-detail-field"></a>

## Platform diagnostic detail field

A typed platform-boundary Store failure has this exact detail shape:

```schema
diagnostic_code: string
```

`diagnostic_code` is the canonical namespaced `platform.*` code selected by the
typed platform diagnostic kind. Unsupported platform kinds route through
`VALIDATION_FAILED`. An unavailable platform observation is a typed Core
operational failure outside `ToolError` and therefore has no
`ToolError.details`. Other Store failures use `store_failure_category` only
when their Store-owned classification selects a public method rejection.

<a id="authority-basis-mismatch-detail-fields"></a>

## Authority basis mismatch detail fields

A schema-valid request whose authority coordinate differs from current Task
authority uses:

```schema
AuthorityBasisMismatch:
  field: string
  expected: null | string | unsigned integer
  received: null | string | unsigned integer
  state_change_applied: false
```

`expected` and `received` preserve the authority coordinate's JSON primitive
type, including actual null. For `baseline_ref`, expected null plus a non-null received
`BaselineRef` says: `Expected baseline_ref=null. Received a non-null
BaselineRef. The Task state is valid; retry against the current action form.`
This detail is request recovery, not owner-state corruption, and never by
itself recommends Task repair.

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
`code=PERSISTED_DATA_CORRUPT` and `category=corrupt`,
`owner_state_error` has exactly one of these shapes:

- Field-local corruption identifies `table`, `record_ref`, `logical_column`,
  and `corruption_category`.
- Cross-field aggregate corruption identifies `aggregate`, `record_ref`,
  `invariant`, and `corruption_category`.

The field-local shape uses `corruption_category=corrupt_stored_json` or
`corruption_category=corrupt_stored_value` and names the actual invalid
physical field. The aggregate shape uses
`corruption_category=corrupt_aggregate_invariant` and does not include `table`
or `logical_column`.

For `aggregate=write_ticket`, `invariant` is one of:

- `task_identity_agreement`
- `change_unit_identity_agreement`
- `scope_revision_agreement`
- `baseline_agreement`
- `timestamp_order`
- `duplicate_intended_paths`
- `duplicate_allowed_paths`
- `duplicate_denied_paths`
- `allowed_denied_path_disjointness`
- `intended_path_coverage`
- `product_file_write_intent_agreement`
- `status_lifecycle_agreement`

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
