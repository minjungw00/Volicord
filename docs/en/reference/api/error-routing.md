# API error routing

This document owns API response branch routing for rejected responses, blocked results, and `dry_run` previews.

It does not define public `ErrorCode` meanings, primary-code precedence, `ToolError.details`, response branch shapes, display labels, close-readiness meaning, or detailed close-readiness blocker routing.

## Owner boundaries

This document owns:

- The boundary between `ToolRejectedResponse.errors[]`, method-specific blocked results, and `ToolDryRunResponse` preview diagnostics.
- Rejected-response routing for request, precondition, state, idempotency, and pre-preview failures.
- Blocked-result branch routing, including the distinction between `PrepareWriteResult` blocked decisions and `CloseTaskResult(close_state=blocked)`.
- `dry_run` branch routing for valid read-only calls, valid previews, preview blockers, and pre-commit failures.

This document does not own:

- Public code meanings; see [API error codes](error-codes.md).
- Primary public-error selection; see [API error precedence](error-precedence.md).
- Machine-readable error details; see [API error details](error-details.md).
- `CloseReadinessBlocker`, `WriteDecisionReason`, `PlannedBlocker`, and common branch shapes; see [API State Schemas](schema-state.md), [API Value Sets](schema-value-sets.md), and [API Schema Core](schema-core.md).
- Close-readiness meaning and non-substitution rules; see [Core Model close readiness](../core-model.md#close_task).
- Close-readiness blocker routing, forbidden public-error-as-blocker representation, and `harness.close_task` blocker mapping; see [API blocker routing](blocker-routing.md).

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
- Boundary: Blocked result data is not a public transport or schema error.
- State effect: Only the method owner may allow a committed blocked result or read-only blocker data.

<a id="error-vs-blocker-dry-run-preview"></a>
Dry-run preview:
- Public shape: `ToolDryRunResponse` with `DryRunSummary.would_errors[]` or `DryRunSummary.would_blockers[]`.
- Meaning: Previewable diagnostics for a valid dry-run request.
- State effect: Not a committed write and not stored blocker state.

`ErrorCode` values are public API identifiers. Operation-specific blocker-code routing and close-readiness blocker mapping belong to [API blocker routing](blocker-routing.md).

Rendered labels and messages are display text owned by [Template Bodies](../template-bodies.md). They do not define API error or blocker semantics and must not be used as `ErrorCode` values, blocker-code values, or machine-readable `ToolError.details` keys.

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

Result boundary:
- Method-specific result-only fields are not part of this rejected response.

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

Routing boundary:
- The conflict is not a blocker.

<a id="rejected-dry-run-pre-preview-failure"></a>
### `dry_run=true` pre-preview failure

Condition:
- A `dry_run=true` request fails before a read result or dry-run preview can be produced.

Route:
- `ToolRejectedResponse` with `dry_run=true`.

State effect:
- No committed operation or dry-run preview is produced.

Preview boundary:
- The rejection is not represented as `DryRunSummary.would_errors[]` or `PlannedBlocker`.

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

Result boundary:
- `PrepareWriteResult` blocked decisions do not return `CloseReadinessBlocker`.

<a id="blocked-close-task-result"></a>
### `CloseTaskResult(close_state=blocked)`

Condition:
- A valid close-readiness evaluation returns close blockers.

Route:
- `blockers: CloseReadinessBlocker[]`.

State effect:
- Only the close-task method owner may define any committed blocked-result effect.

Result data:
- Close-readiness blocker routing belongs to [API blocker routing](blocker-routing.md).

Public-code boundary:
- `CloseTaskResult(close_state=blocked)` does not use `STATE_VERSION_CONFLICT`.

<a id="blocked-read-only-observation"></a>
### Read-only close-blocker observation

Condition:
- `StatusResult.close_blockers` or `harness.close_task intent=check` returns blocker observation data.

Route:
- Read-only `CloseReadinessBlocker` observation data.

State effect:
- No stored blocker and no state-version increment for the read.

Blocked result means the method may have returned an operation-specific blocked outcome. It is not a public transport/schema error. Any committed blocked result and any state effect must be allowed by the relevant method owner listed in [API Methods](methods.md) and [Storage Effects](../storage-effects.md).

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

Branch boundary:
- `dry_run=true` is not a synonym for `ToolDryRunResponse`.

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

Preview boundary:
- Preview blockers are not stored `CloseReadinessBlocker` objects.
- `PlannedBlocker.code` must not be `STATE_VERSION_CONFLICT`.

<a id="dry-run-pre-commit-failure"></a>
### Pre-commit failure with `dry_run=true`

Condition:
- A `dry_run=true` request has a pre-commit failure.

Response path:
- `ToolRejectedResponse`.

Preview boundary:
- The failure is not represented as dry-run preview data.
- Stale state is rejected before preview.

<a id="forbidden-stale-state-blocker-code"></a>
<a id="forbidden-pre-commit-public-error-copy"></a>
<a id="forbidden-public-code-reuse"></a>
<a id="forbidden-user-facing-label-identifier"></a>
<a id="forbidden-dry-run-stale-state-preview"></a>
<a id="harnessclose_task-close-blockers"></a>
<a id="close-task-preflight-failure"></a>
<a id="close-task-intent-check"></a>
<a id="close-task-intent-complete-blocked"></a>
<a id="close-task-intent-complete-closed"></a>
<a id="close-task-invalid-terminal-transition"></a>
<a id="close-mapping-evidence-gap"></a>
<a id="close-mapping-artifact-issue"></a>
<a id="close-mapping-final-acceptance"></a>
<a id="close-mapping-residual-risk-not-visible"></a>
<a id="close-mapping-unaccepted-residual-risk"></a>
<a id="close-mapping-unresolved-user-judgment"></a>
<a id="close-mapping-sensitive-approval"></a>
<a id="close-mapping-scope-boundary-baseline"></a>
<a id="close-mapping-readable-view-freshness"></a>
<a id="close-mapping-stale-state-rejected"></a>

## Close-readiness blocker routing

Detailed close-readiness blocker routing, forbidden public-error-as-blocker representation, and `harness.close_task` blocker mapping belong to [API blocker routing](blocker-routing.md).
