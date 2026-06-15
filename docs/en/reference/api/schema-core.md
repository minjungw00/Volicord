# API schema core

This document owns the common API envelope (`ToolEnvelope`) and shared schema elements used by the baseline public API, including the common response branch model, shared support shapes, and schema notation conventions below.

Neighboring contracts stay with their owners: method behavior routes through [API Methods](methods.md), storage effects through [Storage Effects](../storage-effects.md), Core authority through [Core Model](../core-model.md), runtime boundaries through [Runtime Boundaries](../runtime-boundaries.md), and display wording or template text through [Template Bodies](../template-bodies.md).

## Owns / Does not own

This document owns:

- schema notation conventions for API schema owner documents
- `ToolEnvelope`
- the common method result branch model
- `ToolResultBase`
- `ToolRejectedResponse`
- `ToolDryRunResponse`
- `ToolError`
- `EventRef`
- the common `response_kind` and `effect_kind` fields

This document does not own:

- method behavior; see the [API Methods](methods.md) and method owner documents
- state and current-position schemas; see [API State Schemas](schema-state.md)
- artifact schemas; see [API Artifact Schemas](schema-artifacts.md)
- user-owned judgment schemas; see [API Judgment Schemas](schema-judgment.md)
- supported method names, `response_kind` values, `effect_kind` values, access classes, or other enum-like values; see [API Value Sets](schema-value-sets.md)
- public error codes, precedence, or error semantics; see [API error codes](error-codes.md) and [API error precedence](error-precedence.md)
- storage records or effects; see [Storage Records](../storage-records.md) and [Storage Effects](../storage-effects.md)

## Schema notation

Meaning:
- Schema blocks in this page are contract notation for public API shapes.
- They describe field presence and nesting, not method-specific behavior.

Does not imply:
- Schema blocks are not generated code.

Notation:
- `string` identifies the JSON scalar shape only. It does not by itself mean free-form text.
- `string | null` means the field is present and may be null.
- `Type[]` means an array of that type.

String-like field classes:
- A controlled value string must use the supported values from its linked value-set owner.
- An opaque identifier or classification string is stable enough to carry, compare, correlate, or route to a narrower owner, but it is not an exhaustive public enum unless the owner publishes a value list.
- A free-form display string is human-facing text. It is not a canonical schema value, error code, blocker code, or storage identifier.

Owner links:
- Controlled value strings: [API Value Sets](schema-value-sets.md), unless a schema or method owner links to a narrower owner.
- Public error codes: [API error codes](error-codes.md).
- API examples must use supported enum-like values from [API Value Sets](schema-value-sets.md) unless the relevant schema owner explicitly defines the field as free-form display text, an opaque identifier, or an opaque classification string.

<a id="tool-envelope"></a>
## `ToolEnvelope`

Meaning:
- `ToolEnvelope` is the common request envelope used by public methods.

Does not imply:
- It does not override narrower method-specific request rules.

Owner links:
- Method-specific request rules: method owner documents routed from [API Methods](methods.md).

```yaml
ToolEnvelope:
  project_id: string
  task_id: string | null
  actor_kind: string
  surface_id: string
  request_id: string
  idempotency_key: string | null
  expected_state_version: integer | null
  dry_run: boolean
  locale: string | null
```

Meaning:
- `task_id` is an optional request-level Task selector.
- `actor_kind` is a controlled value string.
- `expected_state_version` is the request-level field for a project-wide state clock value.
- `project_id`, `task_id`, `surface_id`, `request_id`, and `idempotency_key` are opaque identifiers.
- `locale` is a locale tag string, not a Harness-controlled value set.

Does not imply:
- This field list does not define conflict behavior, storage versioning, or method-specific selector precedence.

Owner links:
- `actor_kind` values: [actor values](schema-value-sets.md#actor-values)
- method-specific request behavior: method owner documents routed from [API Methods](methods.md)
- conflict behavior: [state version conflict](error-precedence.md#state-conflict-behavior)
- storage version behavior: [Storage Versioning](../storage-versioning.md)

<a id="common-response"></a>
## Common response branches

Every public method response uses exactly one branch:

- a method-specific `MethodResult`
- `ToolRejectedResponse`
- `ToolDryRunResponse` when the method owner defines a dry-run preview branch

Meaning:
- `MethodResult` is the method-specific result branch defined by method owner documents routed from [API Methods](methods.md).
- Every concrete method result carries `base: ToolResultBase` and then only that method's result fields.

Does not imply:
- `MethodResult` is not a single concrete schema.

```yaml
ToolResultBase:
  response_kind: string
  effect_kind: string
  dry_run: boolean
  state_version: integer | null
  events: EventRef[]

ToolRejectedResponse:
  base: ToolResultBase
  errors: ToolError[]

ToolDryRunResponse:
  base: ToolResultBase
  dry_run_summary: DryRunSummary
```

Meaning:
- Method-specific result fields belong only to the method result branch.

Does not imply:
- `ToolRejectedResponse` and `ToolDryRunResponse` do not carry result-only fields such as `task_ref`, `run_summary`, `staged_artifact_handle`, `write_authorization_ref`, `user_judgment_ref`, `decision`, or `close_state`.

Owner links:
- supported `response_kind` and `effect_kind` values: [response and effect values](schema-value-sets.md#response-and-effect-values)
- shared branch reading: [common response branches](#common-response)
- method-specific state effects: method owner documents
- public error precedence: [API error precedence](error-precedence.md)

## Dry-run summary shapes

Meaning:
- `DryRunSummary`, `PlannedEffect`, and `PlannedBlocker` are common dry-run branch support shapes.
- They are descriptive preview-data shapes only.

Does not imply:
- This page does not define record creation, ref reservation, handle consumption, replay rows, or `state_version` effects.

```yaml
DryRunSummary:
  planned_effects: PlannedEffect[]
  would_blockers: PlannedBlocker[]
  would_errors: ToolError[]
  next_actions: NextActionSummary[]
  diagnostics: string[]

PlannedEffect:
  target_kind: string
  action: string
  description: string

PlannedBlocker:
  source_kind: string
  category: string
  code: string
  message: string
  related_refs: StateRecordRef[]
```

Owner links:
- `NextActionSummary` and `StateRecordRef`: [API State Schemas](schema-state.md)
- `PlannedBlocker.source_kind` values: [state and blocker values](schema-value-sets.md#state-and-blocker-values)
- `PlannedBlocker.category` value routing: [state and blocker values](schema-value-sets.md#state-and-blocker-values)
- public `ErrorCode` values used in `ToolError.code`: [API error codes](error-codes.md)

`PlannedEffect.target_kind` and `PlannedEffect.action` are opaque preview classification strings unless a method owner narrows them for a specific dry-run branch. `PlannedEffect.description` and `DryRunSummary.diagnostics[]` entries are free-form display strings.

`PlannedBlocker.category` uses the category set for the blocker family named by `PlannedBlocker.source_kind`: write-decision categories for `source_kind=write_decision`, and close-readiness blocker categories for `source_kind=close_readiness`. `PlannedBlocker.code` is an opaque preview reason code unless the method owner explicitly defines a narrower local code list. `PlannedBlocker.message` is a free-form display string.

<a id="shared-support-shapes"></a>

## Shared support shapes

```yaml
ToolError:
  code: string
  message: string
  retryable: boolean
  details: object | null

EventRef:
  event_id: string
  event_kind: string
```

Meaning:
- `ToolError` is the shape used by `ToolRejectedResponse.errors` and previewable `DryRunSummary.would_errors`.
- `ToolError.code` is a public `ErrorCode` value.
- `ToolError.message` is a free-form display string.
- `EventRef.event_id` is an opaque event identifier.
- `EventRef.event_kind` is an opaque event classification string. It is stable enough to carry and route, but this document does not publish an exhaustive public event-kind value set.

Owner links:
- public error code set: [API error codes](error-codes.md)
- error details semantics: [API error details](error-details.md)
- primary-error precedence: [API error precedence](error-precedence.md)
- `EventRef.event_kind` opaque boundary: [opaque and method-scoped string fields](schema-value-sets.md#opaque-and-method-scoped-string-fields)
