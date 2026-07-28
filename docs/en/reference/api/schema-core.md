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
- `OperationResultRef`
- the common `response_kind` and `effect_kind` fields
- the canonical public result body before transport-specific wrapping

This document does not own:

- method behavior; see the [API Methods](methods.md) and method owner documents
- state and current-position schemas; see [API State Schemas](schema-state.md)
- artifact schemas; see [API Artifact Schemas](schema-artifacts.md)
- user-owned judgment schemas; see [API Judgment Schemas](schema-judgment.md)
- supported method names, `response_kind` values, `effect_kind` values,
  `FailureCategory` values, operation categories, or other enum-like values;
  see [API Value Sets](schema-value-sets.md)
- public error codes, precedence, or error semantics; see [API error codes](error-codes.md) and [API error precedence](error-precedence.md)
- product-wide failure-category meanings; see the
  [Failure Model](../failure-model.md)
- storage records or effects; see [Storage Records](../storage-records.md) and [Storage Effects](../storage-effects.md)
- MCP revision-specific tool definitions, result carriers, or error flags; see
  [MCP transport](../mcp-transport.md)

`volicord-types` implements the adapter-neutral public schema family described
here. Its generated public method schemas contain no MCP request wrappers,
MCP error identities, structured-content unions, tool-definition envelopes, or
JSON-RPC fields. Those generated wire schemas and exact serialization rules
belong to `volicord-mcp-wire`; semantic profile selection belongs to
`volicord-mcp-protocol`.

For Core-owned MCP tools, the canonical `AgentToolId` identity reuses the
existing `MethodName` domain and projects its stable MCP wire name. Adapter
utilities belong to the same closed identity catalog. The operational
`ToolVerificationRole::ManagedHostRoundTrip` binding to
`AgentToolId::LIST_PROJECTS` is compile-time metadata on that catalog; it does
not define another Core method identity.

`volicord.begin_integration_verification`, `volicord.guard_probe`, and
`volicord.get_integration_verification` are the catalog's
Connection-integration members. Their public request/result schemas are owned
by [MCP Transport](../mcp-transport.md#in-chat-integration-verification-schemas),
including the shared tagged `IntegrationVerificationWorkflowState`, its typed
tool-directed alternatives, phase observations, and bounded findings. They are
not owned by `ToolEnvelope`, a Core method schema, or Task state. Their adapter
effects are owned by
[Storage Effects](../storage-effects.md#connection-integration-verification-effects).

## Schema notation

Meaning:
- Schema blocks in this page are contract notation for public API shapes.
- They describe field presence and nesting, not method-specific behavior.

Does not imply:
- Schema blocks are not generated code.

Notation:
- `string` identifies the JSON scalar shape only. It does not by itself mean free-form text.
- `T | null` means the field is required to be present and may contain either `T` or JSON `null`.
- An optional field may be omitted only when the owning schema or method field note explicitly marks it optional.
- Optionality and nullability are independent properties.
- `Type[]` means an array of that type.
- JSON Schema validation and typed decoder behavior must accept and reject the same payloads.
- Canonical idempotency hashing uses the successfully decoded typed request, not raw JSON formatting.

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
- All fields shown in `ToolEnvelope` are required envelope members. Members typed as `T | null` must still be present and may contain JSON `null`.

Does not imply:
- It does not override narrower method-specific request rules.
- It does not carry `actor_source`, `operation_category`, verification basis or other invocation provenance.

Owner links:
- Method-specific request rules: method owner documents routed from [API Methods](methods.md).

```schema
ToolEnvelope:
  project_id: string
  task_id: string | null
  request_id: string
  idempotency_key: string | null
  expected_state_version: integer | null
  dry_run: boolean
  locale: string | null
```

Meaning:
- `task_id` is a nullable request-level `Task` selector; the field is present and the value may be null.
- `expected_state_version` is the request-level optimistic-concurrency field for a `project_state.state_version` value.
- `idempotency_key` is a nullable opaque identifier; method owners define when a non-null value is required.
- `expected_state_version` is nullable; method and storage owners define when a non-null value is required.
- When a projected `NextActionSummary.expected_state_version` is non-null, it maps directly to this field for that action's owner-method invocation. A null next-action value does not waive a non-null token requirement that the method owner otherwise imposes.
- `StateRecordRef.produced_at_state_version` is projection-freshness metadata and never substitutes for `ToolEnvelope.expected_state_version`.
- `project_id`, `task_id`, `request_id`, and `idempotency_key` are opaque identifiers when non-null.
- `locale` is a nullable locale tag string, not a Volicord-controlled value set.
- Actor provenance and operation category are derived by adapter/Core logic described by [Agent Connection](../agent-connection.md), not by public request fields.

Does not imply:
- This field list does not define conflict behavior, storage versioning, or method-specific selector precedence.

Owner links:
- actor-source values: [actor source values](schema-value-sets.md#actor-source-values)
- projected next-action tokens and state-reference freshness: [API State Schemas](schema-state.md#state-references) and [current-position display shapes](schema-state.md#current-position-display-shapes)
- method-specific request behavior: method owner documents routed from [API Methods](methods.md)
- conflict behavior: [state version conflict](error-precedence.md#state-conflict-behavior)
- storage version behavior: [Storage Versioning](../storage-versioning.md)

<a id="common-response"></a>
## Common response branches

These response schemas define the canonical public result body independently
of its transport carrier. An MCP adapter retains that same object and projects
it into the carrier permitted by the selected protocol profile. `toolResult`,
`content`, `structuredContent`, and MCP `isError` are transport fields owned by
[MCP transport](../mcp-transport.md); they do not add, remove, or reinterpret
the fields in the API result body and do not change Core branch semantics.

Every public method response uses exactly one branch:

- a method-specific `MethodResult`
- `ToolRejectedResponse`
- `ToolDryRunResponse` when the method owner defines a `dry_run` preview branch

Meaning:
- `MethodResult` is the method-specific result branch defined by method owner documents routed from [API Methods](methods.md).
- Every concrete method result carries `base: ToolResultBase` and then only that method's result fields.
- Each method has one exact closed response family: `MethodResult |
  ToolRejectedResponse`, or that union plus `ToolDryRunResponse` when the
  method owner defines preview behavior. The generated schema and public
  decoder for that method contain exactly those branches.
- `response_kind=result` and a successful transport do not authorize a
  completion claim. Task-scoped workflow results use the current
  `AuthorityReceipt.completion_claim_allowed`; rejected, dry-run, refresh-
  failed, and no-active-Task branches are never completion authority.

Does not imply:
- `MethodResult` is not a single concrete schema.
- The presence of `ToolEnvelope.dry_run` does not add a preview response branch
  to a method that does not define one.

<!-- BEGIN GENERATED: contract-structures api.schema.core[schema_object.ToolResultBase] api.schema.core[schema_object.ToolRejectedResponse] api.schema.core[schema_object.ToolDryRunResponse] -->
<!-- Generated by `cargo run -p xtask -- docs-sync`; do not edit this region. -->

### `ToolResultBase` fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `disclosure` | yes | no | `GuaranteeDisclosure` |
| `dry_run` | yes | no | `boolean` |
| `effect_kind` | yes | no | `EffectKind` |
| `events` | yes | no | `EventRef[]` |
| `response_kind` | yes | no | `ResponseKind` |
| `state_version` | no | yes | `integer` |

### `ToolRejectedResponse` rejection fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `base` | yes | no | `ToolResultBase` |
| `errors` | yes | no | `ToolError[]` |

### `ToolDryRunResponse` preview fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `base` | yes | no | `ToolResultBase` |
| `dry_run_summary` | yes | no | `DryRunSummary` |
<!-- END GENERATED: contract-structures api.schema.core[schema_object.ToolResultBase] api.schema.core[schema_object.ToolRejectedResponse] api.schema.core[schema_object.ToolDryRunResponse] -->

Meaning:
- Method-specific result fields belong only to the method result branch.
- `ToolResultBase.disclosure` is the public machine-readable guarantee and non-guarantee disclosure for interpreting the response branch.

Does not imply:
- `ToolRejectedResponse` and `ToolDryRunResponse` do not carry result-only fields such as `task_ref`, `run_summary`, `staged_artifact_handle`, `write_ticket_ref`, `user_action_resolution_ref`, `decision`, or `close_state`.
- `ToolResultBase.disclosure` does not create OS sandboxing, network isolation, malware defense, tamper-proof audit logging, full write prevention, full filesystem monitoring, actor attribution proof, correctness proof, test sufficiency proof, or a replacement for human review.
- Neither `effect_kind` nor display text can override
  `completion_claim_allowed=false`, a close blocker, or missing authority.

Owner links:
- supported `response_kind` and `effect_kind` values: [response and effect values](schema-value-sets.md#response-and-effect-values)
- disclosure shape: [API State Schemas](schema-state.md#close-readiness-and-validation-shapes)
- disclosure value sets: [API Value Sets](schema-value-sets.md#state-and-blocker-values)
- shared branch reading: [common response branches](#common-response)
- method-specific state effects: method owner documents
- public error precedence: [API error precedence](error-precedence.md)

## `dry_run` summary shapes

Meaning:
- `DryRunSummary`, `PlannedEffect`, and `PlannedBlocker` are common `dry_run` branch support shapes.
- They are descriptive preview-data shapes only.

Does not imply:
- This page does not define record creation, ref reservation, handle consumption, replay rows, or `state_version` effects.

```schema
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

`PlannedEffect.target_kind` and `PlannedEffect.action` are opaque preview classification strings unless a method owner narrows them for a specific `dry_run` branch. `PlannedEffect.description` and `DryRunSummary.diagnostics[]` entries are free-form display strings.

`PlannedBlocker.category` uses the category set for the blocker family named by `PlannedBlocker.source_kind`: write-decision categories for `source_kind=write_decision`, and close-readiness blocker categories for `source_kind=close_readiness`. `PlannedBlocker.code` is an opaque preview reason code unless the method owner explicitly defines a narrower local code list. `PlannedBlocker.message` is a free-form display string.

<a id="shared-support-shapes"></a>

## Shared support shapes

```schema
ToolError:
  category: FailureCategory
  code: ErrorCode
  message: string
  retryable: boolean
  details: object | null

EventRef:
  event_id: string
  event_kind: string

OperationResultRef:
  project_id: string
  source_method: string
  source_idempotency_key: string
  committed_state_version: integer
  response_sha256: string
  response_size_bytes: integer
```

Meaning:
- `ToolError` is the shape used by `ToolRejectedResponse.errors` and previewable `DryRunSummary.would_errors`.
- Every field shown for `ToolError` is required. `details` must be present and
  may be null.
- `ToolError.category` is a supported `FailureCategory` identifier. It
  classifies the failure independently of the narrower public code and domain
  reason.
- `ToolError.code` is a public `ErrorCode` value.
- `ToolError.message` is a free-form display string.
- `EventRef.event_id` is an opaque event identifier.
- `EventRef.event_kind` is an opaque event classification string. It is stable enough to carry and route, but this document does not publish an exhaustive public `event_kind` value set.

<a id="operation-result-retrieval"></a>

`OperationResultRef` meaning:

- `source_method` is the exact public mutation method name whose committed
  replay row stores the response.
- `source_idempotency_key` is the opaque idempotency identity of that committed
  invocation; it is not accepted as the idempotency key of a new mutation.
- `response_sha256` uses the literal `sha256:` prefix followed by 64 lowercase
  hexadecimal digits over the exact stored UTF-8 response bytes.
- `response_size_bytes` is the exact byte length of those same UTF-8 bytes.
- The complete shape is a non-bearer lookup locator. Access is rechecked for
  every page by
  [`volicord.get_operation_result`](method-get-operation-result.md#volicordget_operation_result).
- An `OperationResultRef` is not a `StateRecordRef`, authority receipt, write
  ticket, artifact or evidence reference, retry credential, or authorization
  token. It does not claim that its historical response or state version is
  current.

Owner links:
- `FailureCategory` values: [failure category values](schema-value-sets.md#failure-category-values)
- failure-category meanings and selection boundaries: [Failure Model](../failure-model.md)
- public error code set: [API error codes](error-codes.md)
- error details semantics: [API error details](error-details.md)
- primary-error precedence: [API error precedence](error-precedence.md)
- `EventRef.event_kind` opaque boundary: [opaque and method-scoped string fields](schema-value-sets.md#opaque-and-method-scoped-string-fields)
- `OperationResultRef` storage immutability and exact-byte source: [Storage Versioning](../storage-versioning.md#exact-operation-result-retrieval)
