# API error routing

This document owns API response branch routing for rejected responses, blocked results, and `dry_run` previews.

Use it to choose the Volicord API response branch after the [canonical error/blocker decision flow](error-precedence.md#canonical-error-blocker-decision-flow) has distinguished transport or adapter failures from Core responses. Do not use it to map individual close-readiness blockers, define blocker categories or codes, or decide method-specific behavior.

Owned here:

- The branch boundary between `ToolRejectedResponse.errors[]`, method-specific blocked results, and `ToolDryRunResponse` preview diagnostics.
- Rejected-response routing for request, precondition, state, idempotency, and pre-preview failures.
- Blocked-result branch routing, including the distinction between `PrepareWriteResult` blocked decisions and `CloseTaskResult(close_state=blocked)`.
- `dry_run` branch routing for valid read-only calls, valid previews, preview blockers, and pre-commit failures.

Adjacent owners:

- Public code meanings; see [API error codes](error-codes.md).
- Canonical error/blocker decision flow and primary public-error selection; see [API error precedence](error-precedence.md#canonical-error-blocker-decision-flow).
- Machine-readable error details; see [API error details](error-details.md).
- `CloseReadinessBlocker`, `WriteDecisionReason`, `PlannedBlocker`, and common branch shapes; see [API State Schemas](schema-state.md) and [API Schema Core](schema-core.md). Category and enum-like values are owned by [API Value Sets](schema-value-sets.md).
- Close-readiness meaning and non-substitution rules; see [Core Model close readiness](../core-model.md#close_task).
- Close-readiness blocker/API response boundary and the public-code-to-blocker boundary; see [API blocker routing](blocker-routing.md).
- Method-specific behavior; see [`volicord.close_task`](method-close-task.md) and other method owners.
- Display wording only; see [Template Bodies](../template-bodies.md).
- Product-wide failure-category meanings and selection boundaries; see the
  [Failure Model](../failure-model.md).

## Error vs blocker

| Concept | Public shape | Detail section |
|---|---|---|
| Rejected response | `ToolRejectedResponse.errors[]` | [Rejected response](#error-vs-blocker-rejected-response) |
| Blocked result | method-specific result fields | [Blocked result](#error-vs-blocker-blocked-result) |
| Dry-run preview | `ToolDryRunResponse` | [Dry-run preview](#error-vs-blocker-dry-run-preview) |

<a id="error-vs-blocker-rejected-response"></a>
Rejected response:
- Public shape: `ToolRejectedResponse.errors[]` with required
  `ToolError.category: FailureCategory` and `ToolError.code: ErrorCode`.
- Meaning: The method did not proceed to the committed operation.
- Condition: A typed Volicord request reached Core and failed request validation, freshness, invocation context, `actor_source`, `operation_category`, or another precondition before the method-owned result branch.
- State effect: No committed operation and no state change.

Transport and adapter failures that happen before Core execution are outside this branch. Route them through [MCP transport](../mcp-transport.md) or the owning transport or adapter.

<a id="error-vs-blocker-blocked-result"></a>
Blocked result:
- Public shape: Method-specific result fields such as `write_decision_reasons` or `blockers`.
- Meaning: The method may have returned an operation-specific blocked outcome.
- Boundary: Blocked result data is not a public transport or schema error.
- State effect: It may be response-only or committed. Only the method owner with [Storage Effects](../storage-effects.md) may allow a committed blocker-shaped result.

<a id="error-vs-blocker-dry-run-preview"></a>
Dry-run preview:
- Public shape: `ToolDryRunResponse` with `DryRunSummary.would_errors[]` or `DryRunSummary.would_blockers[]`.
- Meaning: Previewable diagnostics for a valid dry-run request.
- State effect: Not a committed write and not stored blocker state.

`ErrorCode` values are public API identifiers. Close-readiness blocker/API response boundaries and public-code-to-blocker boundaries belong to [API blocker routing](blocker-routing.md).

Display wording belongs to [Template Bodies](../template-bodies.md) only. It does not define API error or blocker semantics and must not be used as `ErrorCode` values, blocker-code values, or machine-readable `ToolError.details` keys.

### Failure-category branch boundary

| `FailureCategory` | API branch rule |
|---|---|
| `rejected` | A structural request or required-context failure before policy evaluation uses `ToolRejectedResponse`. |
| `not_allowed` | When a method owner defines a non-allow result after policy evaluation, use that method-specific result rather than `ToolRejectedResponse` for the same condition. The method and storage-effect owners decide whether it commits. |
| `unavailable` | If the required operation or read cannot continue, use `ToolRejectedResponse` with the applicable unavailable public code. |
| `degraded` | If the core operation truthfully continues, expose the incomplete auxiliary component in the successful method result's owner-defined diagnostic. Do not reject that same operation with a `ToolError(category=degraded)`. |
| `corrupt` | A dependent operation uses `ToolRejectedResponse` with `PERSISTED_DATA_CORRUPT` and fails closed before policy or effects. |

The category is required machine-readable classification. It does not replace
the public code or domain `details.reason` owned by
[API error details](error-details.md#reason).

<a id="blocked-and-dry-run-behavior"></a>

## Rejected response behavior

| Condition | Detail section |
|---|---|
| request validation fails before proceed | [Request validation failure](#rejected-request-validation-failure) |
| persisted owner data is corrupt | [Precondition failure](#rejected-precondition-failure) |
| precondition fails before commit | [Precondition failure](#rejected-precondition-failure) |
| state or idempotency conflict | [State or idempotency conflict](#rejected-state-or-idempotency-conflict) |
| `dry_run=true` pre-preview failure | [`dry_run=true` pre-preview failure](#rejected-dry-run-pre-preview-failure) |

<a id="rejected-request-validation-failure"></a>
### Request validation failure

Condition:
- Request shape, schema, profile, or staged-handle validation fails before the method can proceed.

Boundary:
- This is malformed untrusted input with `category=rejected`. Persisted trusted
  owner-data corruption uses its distinct category and code instead.

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
- Core, MCP, invocation context, `actor_source`/`operation_category`
  compatibility, state lookup, Task identity, persisted owner-data validation,
  exact-contract selection, or another required precondition fails before
  commit.

Route:
- `ToolRejectedResponse.errors[]`.

State effect:
- No records, replay rows, artifacts, events, write-ticket consumption, close-state mutation, or state-version increment.

<a id="rejected-state-or-idempotency-conflict"></a>
### State or idempotency conflict

Condition:
- `expected_state_version` is stale or the idempotency request hash conflicts. A write-ticket audit `basis_state_version` mismatch is not a conflict.

Route:
- `ToolRejectedResponse.errors[]` with `STATE_VERSION_CONFLICT`.

State effect:
- No committed operation proceeds.
- No owner state mutation occurs.

Routing boundary:
- The conflict is not a blocker.
- State-bound write-ticket invalidity on a consuming method instead returns
  `WRITE_TICKET_INVALID`; an unresolved invalidated ticket discovered during
  close readiness is method-owned blocker data. Unrelated state reads and
  writes do not invalidate the ticket.

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

<a id="blocked-result-behavior"></a>

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

Failure category boundary:
- This method-defined post-policy non-allow result is `NotAllowed`; it is not
  the structural `Rejected` branch. Missing current Change Unit remains the
  earlier `ToolRejectedResponse` with `category=rejected`,
  `code=NO_ACTIVE_CHANGE_UNIT`, and
  `details.reason=current_change_unit_required`.

Route:
- `write_decision_reasons: WriteDecisionReason[]`.

State effect:
- The method owner and [Storage Effects](../storage-effects.md) define the committed non-allow effect.

Result data:
- Uses method-owned decision reasons.

Result boundary:
- `PrepareWriteResult` blocked decisions do not return `CloseReadinessBlocker`.

<a id="blocked-close-task-result"></a>
### `CloseTaskResult(close_state=blocked)`

Branch condition:
- A valid `CloseTaskResult(close_state=blocked)` is returned under the `volicord.close_task` method contract.

Response branch:
- The method result carries `blockers: CloseReadinessBlocker[]`.

State effect:
- The close-task method owner and [Storage Effects](../storage-effects.md) define whether a blocked close-task result is response-only or committed.

Result data boundary:
- Close-readiness blocker/API response routing and public-code-to-blocker routing belong to [API blocker routing](blocker-routing.md).
- `CloseReadinessBlocker` shape and category values stay with [API State Schemas](schema-state.md) and [API Value Sets](schema-value-sets.md).

Public-code boundary:
- `CloseTaskResult(close_state=blocked)` does not use `STATE_VERSION_CONFLICT`.

<a id="blocked-read-only-observation"></a>
### Read-only close-blocker observation

Branch condition:
- A read-only status or check result exposes close-blocker observation data.

Response branch:
- Read-only `CloseReadinessBlocker` observation data.

State effect:
- No stored blocker and no state-version increment for the read.

Blocked result means the method may have returned an operation-specific blocked outcome. It is not a public transport/schema error and it does not use `ToolRejectedResponse.errors[]`. Any committed blocker-shaped result and any state effect must be allowed by the relevant method owner listed in [API Methods](methods.md) and [Storage Effects](../storage-effects.md).

<a id="dry-run-behavior"></a>

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
