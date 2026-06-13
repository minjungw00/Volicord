# API error routing

This document owns API error versus blocker routing for rejected responses, blocked results, `dry_run` previews, forbidden blocker-code use, and `close_task` blocker mapping.

It does not define public `ErrorCode` meanings, primary-code precedence, `ToolError.details`, response branch shapes, display labels, or close-readiness meaning.

## Owner boundaries

This document owns:

- The boundary between `ToolRejectedResponse.errors[]`, method-specific blocked results, and `ToolDryRunResponse` preview diagnostics.
- Rules that keep pre-commit public errors out of blocker-code arrays.
- `close_task` mappings between close-readiness findings and public error-code families where the mapping is needed.

This document does not own:

- Public code meanings; see [API error codes](error-codes.md).
- Primary public-error selection; see [API error precedence](error-precedence.md).
- Machine-readable error details; see [API error details](error-details.md).
- `CloseReadinessBlocker`, `WriteDecisionReason`, `PlannedBlocker`, and common branch shapes; see [API State Schemas](schema-state.md), [API Value Sets](schema-value-sets.md), and [API Schema Core](schema-core.md).
- Close-readiness meaning and non-substitution rules; see [Core Model close readiness](../core-model.md#close_task).

## Error vs blocker

| Concept | Public shape | Detail section |
|---|---|---|
| Rejected response | `ToolRejectedResponse.errors[]` | [Rejected response](#error-vs-blocker-rejected-response) |
| Blocked result | method-specific result fields | [Blocked result](#error-vs-blocker-blocked-result) |
| Dry-run preview | `ToolDryRunResponse` | [Dry-run preview](#error-vs-blocker-dry-run-preview) |

<a id="error-vs-blocker-rejected-response"></a>
Rejected response:
- Public shape: `ToolRejectedResponse.errors[]` with `ToolError.code: ErrorCode`.
- Meaning: The method did not proceed to the committed operation.
- Condition: The failure is public transport, request, freshness, local-access, capability, or precondition rejection.
- State effect: No committed operation and no state change.

<a id="error-vs-blocker-blocked-result"></a>
Blocked result:
- Public shape: Method-specific result fields such as `write_decision_reasons` or `blockers`.
- Meaning: The method may have returned an operation-specific blocked outcome.
- Non-claim: This is not a public transport or schema error.
- State effect: Only the method owner may allow a committed blocked result or read-only blocker data.

<a id="error-vs-blocker-dry-run-preview"></a>
Dry-run preview:
- Public shape: `ToolDryRunResponse` with `DryRunSummary.would_errors[]` or `DryRunSummary.would_blockers[]`.
- Meaning: Previewable diagnostics for a valid dry-run request.
- State effect: Not a committed write and not stored blocker state.

`ErrorCode` values are public API identifiers. Blocker codes are operation-specific result values. A public `ErrorCode` must not be reused as a blocker code unless the canonical method or schema owner explicitly allows that use.

Rendered labels and messages are display text owned by [Template Bodies](../template-bodies.md). They must not be used as `ErrorCode` values, blocker-code values, or machine-readable `ToolError.details` keys.

<a id="blocked-and-dry-run-behavior"></a>

## Rejected response behavior

| Condition | Detail section |
|---|---|
| request validation fails before proceed | [Request validation failure](#rejected-request-validation-failure) |
| precondition fails before commit | [Precondition failure](#rejected-precondition-failure) |
| state or idempotency conflict | [State or idempotency conflict](#rejected-state-or-idempotency-conflict) |
| `dry_run=true` pre-preview failure | [`dry_run=true` pre-preview failure](#rejected-dry-run-pre-preview-failure) |

<a id="rejected-request-validation-failure"></a>
### Request validation failure

Condition:
- Request shape, schema, profile, or staged-handle validation fails before the method can proceed.

Route:
- `ToolRejectedResponse.errors[]`.

State effect:
- No committed operation proceeds.
- No owner state mutation occurs.

Not allowed:
- Do not include method-specific result-only fields.

<a id="rejected-precondition-failure"></a>
### Precondition failure

Condition:
- Core, MCP, local access, surface capability, state lookup, Task identity, or a required precondition fails before commit.

Route:
- `ToolRejectedResponse.errors[]`.

State effect:
- No records, replay rows, artifacts, events, Write Authorization consumption, close-state mutation, or state-version increment.

<a id="rejected-state-or-idempotency-conflict"></a>
### State or idempotency conflict

Condition:
- `expected_state_version`, `WriteAuthorization.basis_state_version`, or idempotency request hash is stale or conflicting.

Route:
- `ToolRejectedResponse.errors[]` with `STATE_VERSION_CONFLICT`.

State effect:
- No committed operation proceeds.
- No owner state mutation occurs.

Not allowed:
- The conflict is not a blocker.

<a id="rejected-dry-run-pre-preview-failure"></a>
### `dry_run=true` pre-preview failure

Condition:
- A `dry_run=true` request fails before a read result or dry-run preview can be produced.

Route:
- `ToolRejectedResponse` with `dry_run=true`.

State effect:
- No committed operation or dry-run preview is produced.

Not allowed:
- Do not represent the rejection as `DryRunSummary.would_errors[]` or `PlannedBlocker`.

Rejected response means the method did not proceed to the committed operation. It is not a blocked result and does not create the authority, evidence, acceptance, or close state that the request lacked.

## Blocked result behavior

| Blocked path | Detail section |
|---|---|
| `PrepareWriteResult` blocked decision | [`PrepareWriteResult` blocked decision](#blocked-prepare-write-result) |
| `CloseTaskResult(close_state=blocked)` | [`CloseTaskResult(close_state=blocked)`](#blocked-close-task-result) |
| read-only close-blocker observation | [Read-only close-blocker observation](#blocked-read-only-observation) |

<a id="blocked-prepare-write-result"></a>
### `PrepareWriteResult` blocked decision

Condition:
- `PrepareWriteResult` has `decision=blocked`, `decision=approval_required`, or `decision=decision_required`.

Route:
- `write_decision_reasons: WriteDecisionReason[]`.

State effect:
- Only the method owner may define any committed blocked-result effect.

Result data:
- Uses method-owned decision reasons.

Not allowed:
- Does not return `CloseReadinessBlocker`.

<a id="blocked-close-task-result"></a>
### `CloseTaskResult(close_state=blocked)`

Condition:
- A valid close-readiness evaluation returns close blockers.

Route:
- `blockers: CloseReadinessBlocker[]`.

State effect:
- Only the close-task method owner may define any committed blocked-result effect.

Result data:
- Uses close-readiness blocker mapping.

Not allowed:
- Must not use `STATE_VERSION_CONFLICT`.

<a id="blocked-read-only-observation"></a>
### Read-only close-blocker observation

Condition:
- `StatusResult.close_blockers` or `harness.close_task intent=check` returns blocker observation data.

Route:
- Read-only `CloseReadinessBlocker` observation data.

Not allowed:
- No stored blocker and no state-version increment for the read.

Blocked result means the method may have returned an operation-specific blocked outcome. It is not a public transport/schema error. Any committed blocked result and any state effect must be allowed by the relevant method owner routed from [API Methods](methods.md) and [Storage Effects](../storage-effects.md).

## Dry-run behavior

| Dry-run case | Detail section |
|---|---|
| valid read-only call | [Valid read-only `dry_run=true`](#dry-run-valid-read-only) |
| valid state-effecting or staging preview | [Valid dry-run preview](#dry-run-valid-preview) |
| expected blockers in preview | [Expected blockers in dry-run preview](#dry-run-expected-blockers) |
| pre-commit failure | [Pre-commit failure with `dry_run=true`](#dry-run-pre-commit-failure) |

<a id="dry-run-valid-read-only"></a>
### Valid read-only `dry_run=true`

Condition:
- A valid read-only call sets `dry_run=true`.

Response path:
- Method-specific result with `base.dry_run=true` and `base.effect_kind=read_only`.

Not allowed:
- Do not treat `dry_run=true` as a synonym for `ToolDryRunResponse`.

<a id="dry-run-valid-preview"></a>
### Valid dry-run preview

Condition:
- A valid state-effecting or storage-owned staging operation sets `dry_run=true`.

Response path:
- `ToolDryRunResponse` with `DryRunSummary`.

State effect:
- The dry-run preview is not a committed write.

<a id="dry-run-expected-blockers"></a>
### Expected blockers in dry-run preview

Condition:
- A valid dry-run preview has expected blockers.

Response path:
- `DryRunSummary.would_blockers: PlannedBlocker[]`.

Not allowed:
- Preview blockers are not stored `CloseReadinessBlocker` objects.
- `PlannedBlocker.code` must not be `STATE_VERSION_CONFLICT`.

<a id="dry-run-pre-commit-failure"></a>
### Pre-commit failure with `dry_run=true`

Condition:
- A `dry_run=true` request has a pre-commit failure.

Response path:
- `ToolRejectedResponse`.

Not allowed:
- Do not represent the failure as dry-run preview data.
- Stale state is rejected before preview.

## Forbidden blocker-code rules

| Forbidden use | Detail section |
|---|---|
| stale-state public error used as a blocker code | [Stale-state blocker code](#forbidden-stale-state-blocker-code) |
| pre-commit public error copied into blocker arrays | [Pre-commit public error copy](#forbidden-pre-commit-public-error-copy) |
| public `ErrorCode` reused without owner permission | [Public code reuse](#forbidden-public-code-reuse) |
| user-facing label used as API identifier | [User-facing label identifier](#forbidden-user-facing-label-identifier) |
| dry-run stale-state conflict previewed | [Dry-run stale-state preview](#forbidden-dry-run-stale-state-preview) |

<a id="forbidden-stale-state-blocker-code"></a>
### Stale-state blocker code

Not allowed:
- Do not use `STATE_VERSION_CONFLICT` as `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, `PlannedBlocker.code`, `MethodResult.decision`, or a committed blocked-result primary code.

Use instead:
- Return `ToolRejectedResponse.errors[]` with `effect_kind=no_effect`.

<a id="forbidden-pre-commit-public-error-copy"></a>
### Pre-commit public error copy

Not allowed:
- Do not copy pre-commit public errors into blocker arrays.

Use instead:
- Return `ToolRejectedResponse.errors[]`.

<a id="forbidden-public-code-reuse"></a>
### Public code reuse

Not allowed:
- Do not reuse a public `ErrorCode` as a blocker code without explicit canonical owner permission.

Use instead:
- Use the method/schema owner's blocker code or result reason.

<a id="forbidden-user-facing-label-identifier"></a>
### User-facing label identifier

Not allowed:
- Do not use a user-facing label as an API identifier.

Use instead:
- Keep the public `ErrorCode` unchanged and localize only display text.

<a id="forbidden-dry-run-stale-state-preview"></a>
### Dry-run stale-state preview

Not allowed:
- Do not represent a dry-run stale-state conflict in `DryRunSummary.would_errors[]` or `DryRunSummary.would_blockers[]`.

Use instead:
- Reject the request with `STATE_VERSION_CONFLICT`.

<a id="harnessclose_task-close-blockers"></a>

## `close_task` blocker mapping

- Preflight failure before close-readiness evaluation:
  - [Preflight failure](#close-task-preflight-failure)
- `intent=check` with a valid read:
  - [`intent=check`](#close-task-intent-check)
- `intent=complete` with close-readiness blockers:
  - [`intent=complete` blocked](#close-task-intent-complete-blocked)
- `intent=complete` with no close blockers:
  - [`intent=complete` closed](#close-task-intent-complete-closed)
- Invalid `intent=cancel` or `intent=supersede` terminal transition:
  - [Invalid terminal transition](#close-task-invalid-terminal-transition)

<a id="close-task-preflight-failure"></a>
### Preflight failure

Condition:
- Stale state, stale Write Authorization basis, idempotency conflict, validation failure, local-access failure, capability failure, unreadable Core state, or unresolved project/Task identity occurs before close-readiness evaluation.

Response path:
- `ToolRejectedResponse.errors[]`

Public-code rule:
- `STATE_VERSION_CONFLICT` and other pre-commit errors stay in the rejected response.

Not allowed:
- Do not return `CloseReadinessBlocker` entries.

<a id="close-task-intent-check"></a>
### `intent=check`

Condition:
- The request is a valid read.

Response path:
- `CloseTaskResult` read-only result

Allowed:
- May return `CloseReadinessBlocker` observation data.

State effect:
- No stored blocker and no state-version increment.

<a id="close-task-intent-complete-blocked"></a>
### `intent=complete` blocked

Condition:
- A valid evaluation finds close-readiness blockers.

Response path:
- `CloseTaskResult(close_state=blocked)`

Allowed:
- May return `CloseReadinessBlocker[]`.

Not allowed:
- Do not use `STATE_VERSION_CONFLICT`.

<a id="close-task-intent-complete-closed"></a>
### `intent=complete` closed

Condition:
- No remaining owner-defined close blockers exist.

Response path:
- `CloseTaskResult(close_state=closed)`

Public-code rule:
- No close blockers.

<a id="close-task-invalid-terminal-transition"></a>
### Invalid terminal transition

Condition:
- `intent=cancel` or `intent=supersede` has an invalid terminal transition.

Response path:
- Method-owned result or rejection path

Public-code rule:
- Blockers are limited to transition validity.

Not allowed:
- Do not require evidence sufficiency, final acceptance, or residual-risk acceptance for cancellation or supersession.

### Close-readiness finding code summary

These rows summarize public error-code families for close-readiness findings. They do not turn public `ErrorCode` values into blocker codes.

| Close-readiness finding | Detail section |
|---|---|
| Evidence gap | [Evidence gap](#close-mapping-evidence-gap) |
| Persistent artifact issue | [Persistent artifact issue](#close-mapping-artifact-issue) |
| Final acceptance issue | [Final acceptance issue](#close-mapping-final-acceptance) |
| Residual risk not visible | [Residual risk not visible](#close-mapping-residual-risk-not-visible) |
| Residual risk missing acceptance | [Residual risk missing acceptance](#close-mapping-unaccepted-residual-risk) |
| Unresolved judgment | [Unresolved user-owned judgment](#close-mapping-unresolved-user-judgment) |
| Sensitive approval issue | [Sensitive-action approval issue](#close-mapping-sensitive-approval) |
| Scope, boundary, or baseline | [Scope, boundary, or baseline blocker](#close-mapping-scope-boundary-baseline) |
| Readable view freshness | [Readable view freshness issue](#close-mapping-readable-view-freshness) |
| Stale state rejection | [Stale state is rejected](#close-mapping-stale-state-rejected) |

<a id="close-mapping-evidence-gap"></a>
### Evidence gap

Condition:
- Close-readiness evaluation finds an evidence gap.

Public code mapping:
- `EVIDENCE_INSUFFICIENT`

<a id="close-mapping-artifact-issue"></a>
### Persistent artifact issue

Condition:
- A close-relevant persistent artifact is missing, unavailable, unusable for the close basis, or failed.

Public code mapping:
- `ARTIFACT_MISSING`

<a id="close-mapping-final-acceptance"></a>
### Final acceptance issue

Condition:
- Required final acceptance is missing or incompatible.

Public code mapping:
- `ACCEPTANCE_REQUIRED`

<a id="close-mapping-residual-risk-not-visible"></a>
### Residual risk not visible

Condition:
- Known close-relevant residual risk is not visible.

Public code mapping:
- `RESIDUAL_RISK_NOT_VISIBLE`

<a id="close-mapping-unaccepted-residual-risk"></a>
### Residual risk missing acceptance

Condition:
- Residual risk is visible and lacks a recorded acceptance.

Public code mapping:
- `DECISION_REQUIRED` or `DECISION_UNRESOLVED` with `category=residual_risk_acceptance`

<a id="close-mapping-unresolved-user-judgment"></a>
### Unresolved user-owned judgment

Condition:
- A user-owned judgment is unresolved.

Public code mapping:
- `DECISION_REQUIRED` or `DECISION_UNRESOLVED`

<a id="close-mapping-sensitive-approval"></a>
### Sensitive-action approval issue

Condition:
- Sensitive-action approval is missing, denied, expired, or drifted.

Public code mapping:
- `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, or `APPROVAL_EXPIRED`

<a id="close-mapping-scope-boundary-baseline"></a>
### Scope, boundary, or baseline blocker

Condition:
- A valid evaluation finds a scope, autonomy boundary, or baseline blocker.

Public code mapping:
- `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, or `BASELINE_STALE`

Not allowed:
- Do not use this mapping unless the owner permits it.

<a id="close-mapping-readable-view-freshness"></a>
### Readable view freshness issue

Condition:
- A readable view freshness issue is present.

Public code mapping:
- `PROJECTION_STALE`

Not allowed:
- Do not use `PROJECTION_STALE` by itself as a close blocker.

<a id="close-mapping-stale-state-rejected"></a>
### Stale state is rejected

Condition:
- Project-wide state or `WriteAuthorization.basis_state_version` is stale.

Response path:
- `ToolRejectedResponse.errors[]` with `STATE_VERSION_CONFLICT`

Not allowed:
- Do not use this as a close blocker.

Owner links:
- Close-readiness meaning and non-substitution rules: [Core Model close readiness](../core-model.md#close_task)
- Method behavior and close-readiness evaluation order: [`harness.close_task`](method-close-task.md)
- `CloseReadinessBlocker` shape and categories: [API State Schemas](schema-state.md) and [API Value Sets](schema-value-sets.md)
