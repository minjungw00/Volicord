# API schema core

This document owns the common API envelope and response-branch schemas for the baseline scope.

It does not define method behavior, storage effects, state snapshots, artifact lifecycle, user-judgment meaning, public error semantics, or supported value sets.

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

Not implied:
- Schema blocks are not generated code.

Notation:
- `string | null` means the field is present and may be null.
- `Type[]` means an array of that type.

Owner links:
- Field value sets: [API Value Sets](schema-value-sets.md), unless this page says the field is free-form text or an opaque identifier.

<a id="tool-envelope"></a>
## `ToolEnvelope`

Meaning:
- `ToolEnvelope` is the common request envelope used by public methods.

Not implied:
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
- `expected_state_version` names the project-wide state clock used by state-changing methods.

Precedence:
- Method-specific `task_id` fields, when present, take precedence as described by the affected method owner document.

Owner links:
- conflict behavior: [state version conflict](error-precedence.md#state-conflict-behavior)
- storage version behavior: [Storage Versioning](../storage-versioning.md)

<a id="common-response"></a>
## Common response branches

Every public method response uses exactly one branch:

- a method-specific `MethodResult`
- `ToolRejectedResponse`
- `ToolDryRunResponse` when the selected state-effecting or storage-staging operation has a valid preview branch

Meaning:
- `MethodResult` is the method-specific successful or committed result branch defined by method owner documents routed from [API Methods](methods.md).
- Every concrete method result carries `base: ToolResultBase` and then only that method's result fields.

Not implied:
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

Not implied:
- `ToolRejectedResponse` and `ToolDryRunResponse` do not carry result-only fields such as `task_ref`, `run_summary`, `staged_artifact_handle`, `write_authorization_ref`, `user_judgment_ref`, `decision`, or `close_state`.

Owner links:
- supported `response_kind` and `effect_kind` values: [response and effect values](schema-value-sets.md#response-and-effect-values)
- shared branch reading: [common response branches](#common-response)
- method-specific state effects: method owner documents
- public error precedence: [API error precedence](error-precedence.md)

## Dry-run summary shapes

Meaning:
- `DryRunSummary`, `PlannedEffect`, and `PlannedBlocker` are common dry-run branch support shapes.
- They are descriptive preview data only.

Not implied:
- They do not create records.
- They do not reserve refs.
- They do not consume handles.
- They do not create replay rows.
- They do not increment `state_version`.

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
- public `ErrorCode` values used in `ToolError.code`: [API error codes](error-codes.md)

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

Owner links:
- public error code set: [API error codes](error-codes.md)
- error details semantics: [API error details](error-details.md)
- primary-error precedence: [API error precedence](error-precedence.md)
