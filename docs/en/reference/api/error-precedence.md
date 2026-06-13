# API error precedence

This document owns primary public-error selection when more than one public error candidate exists. It also owns public stale-state and idempotency conflict behavior for `STATE_VERSION_CONFLICT`.

It does not define the public `ErrorCode` value set, response branch routing, close-readiness blocker routing, `harness.close_task` method behavior, machine-readable detail fields, response branch shapes, storage replay rows, or rendered labels.

## Owner boundaries

This document owns:

- The primary `errors[0]` selection order for error-bearing branches.
- The `STATE_VERSION_CONFLICT` exclusion from result and blocker code paths.
- Public stale `expected_state_version`, stale `WriteAuthorization.basis_state_version`, and idempotency request-hash conflict behavior.

This document does not own:

- Public code meanings outside precedence selection; see [API error codes](error-codes.md).
- API response branch routing; see [API error routing](error-routing.md).
- Close-readiness blocker/API response boundary; see [API blocker routing](blocker-routing.md).
- `harness.close_task` method-specific blocker behavior; see [`harness.close_task`](method-close-task.md).
- Machine-readable conflict detail fields; see [API error details](error-details.md#state-conflict-detail-fields).
- Storage replay rows and state clocks; see [Storage Versioning](../storage-versioning.md).

<a id="primary-error-code-precedence"></a>

## Error precedence

When an error-bearing branch has non-empty `errors`, `errors[0]` is the primary public code selected by this order unless a method owner defines a stricter method-specific order.

| Precedence | Primary `ErrorCode` | Detail section |
|---:|---|---|
| 1 | `VALIDATION_FAILED` | [`VALIDATION_FAILED`](#precedence-validation-failed) |
| 2 | `STATE_VERSION_CONFLICT` | [`STATE_VERSION_CONFLICT`](#state-version-conflict-precedence-exclusion) |
| 3 | `MCP_UNAVAILABLE` | [`MCP_UNAVAILABLE`](#precedence-mcp-unavailable) |
| 4 | `LOCAL_ACCESS_MISMATCH` | [`LOCAL_ACCESS_MISMATCH`](#precedence-local-access-mismatch) |
| 5 | `NO_ACTIVE_TASK` | [`NO_ACTIVE_TASK`](#precedence-no-active-task) |
| 6 | `NO_ACTIVE_CHANGE_UNIT` | [`NO_ACTIVE_CHANGE_UNIT`](#precedence-no-active-change-unit) |
| 7 | `BASELINE_STALE` | [`BASELINE_STALE`](#precedence-baseline-stale) |
| 8 | `SCOPE_REQUIRED` | [`SCOPE_REQUIRED`](#precedence-scope-required) |
| 9 | `SCOPE_VIOLATION` | [`SCOPE_VIOLATION`](#precedence-scope-violation) |
| 10 | `WRITE_AUTHORIZATION_REQUIRED` | [`WRITE_AUTHORIZATION_REQUIRED`](#precedence-write-authorization-required) |
| 11 | `WRITE_AUTHORIZATION_INVALID` | [`WRITE_AUTHORIZATION_INVALID`](#precedence-write-authorization-invalid) |
| 12 | `APPROVAL_DENIED` | [`APPROVAL_DENIED`](#precedence-approval-denied) |
| 13 | `APPROVAL_EXPIRED` | [`APPROVAL_EXPIRED`](#precedence-approval-expired) |
| 14 | `APPROVAL_REQUIRED` | [`APPROVAL_REQUIRED`](#precedence-approval-required) |
| 15 | `DECISION_UNRESOLVED` | [`DECISION_UNRESOLVED`](#precedence-decision-unresolved) |
| 16 | `AUTONOMY_BOUNDARY_EXCEEDED` | [`AUTONOMY_BOUNDARY_EXCEEDED`](#precedence-autonomy-boundary-exceeded) |
| 17 | `DECISION_REQUIRED` | [`DECISION_REQUIRED`](#precedence-decision-required) |
| 18 | `CAPABILITY_INSUFFICIENT` | [`CAPABILITY_INSUFFICIENT`](#precedence-capability-insufficient) |
| 19 | `EVIDENCE_INSUFFICIENT` | [`EVIDENCE_INSUFFICIENT`](#precedence-evidence-insufficient) |
| 20 | `RESIDUAL_RISK_NOT_VISIBLE` | [`RESIDUAL_RISK_NOT_VISIBLE`](#precedence-residual-risk-not-visible) |
| 21 | `ACCEPTANCE_REQUIRED` | [`ACCEPTANCE_REQUIRED`](#precedence-acceptance-required) |
| 22 | `PROJECTION_STALE` | [`PROJECTION_STALE`](#precedence-projection-stale) |
| 23 | `ARTIFACT_MISSING` | [`ARTIFACT_MISSING`](#precedence-artifact-missing) |
| 24 | `VALIDATOR_FAILED` | [`VALIDATOR_FAILED`](#precedence-validator-failed) |

<a id="precedence-validation-failed"></a>
### Precedence 1: `VALIDATION_FAILED`

Applies to:
- Rejected request shape or validation failure.

<a id="precedence-mcp-unavailable"></a>
### Precedence 3: `MCP_UNAVAILABLE`

Applies to:
- Rejected Core, MCP, or surface reachability failure.

<a id="precedence-local-access-mismatch"></a>
### Precedence 4: `LOCAL_ACCESS_MISMATCH`

Applies to:
- Rejected local-access binding mismatch or revocation.

<a id="precedence-no-active-task"></a>
### Precedence 5: `NO_ACTIVE_TASK`

Applies to:
- Rejected missing Task identity.

<a id="precedence-no-active-change-unit"></a>
### Precedence 6: `NO_ACTIVE_CHANGE_UNIT`

Applies to:
- Missing active Change Unit.

<a id="precedence-baseline-stale"></a>
### Precedence 7: `BASELINE_STALE`

Applies to:
- Stale baseline.

<a id="precedence-scope-required"></a>
### Precedence 8: `SCOPE_REQUIRED`

Applies to:
- Missing required scope confirmation.

<a id="precedence-scope-violation"></a>
### Precedence 9: `SCOPE_VIOLATION`

Applies to:
- Scope or authorized-attempt violation.

<a id="precedence-write-authorization-required"></a>
### Precedence 10: `WRITE_AUTHORIZATION_REQUIRED`

Applies to:
- Missing required Write Authorization.

<a id="precedence-write-authorization-invalid"></a>
### Precedence 11: `WRITE_AUTHORIZATION_INVALID`

Applies to:
- Non-version invalid Write Authorization.

<a id="precedence-approval-denied"></a>
### Precedence 12: `APPROVAL_DENIED`

Applies to:
- Denied sensitive-action approval.

<a id="precedence-approval-expired"></a>
### Precedence 13: `APPROVAL_EXPIRED`

Applies to:
- Expired or drifted sensitive-action approval.

<a id="precedence-approval-required"></a>
### Precedence 14: `APPROVAL_REQUIRED`

Applies to:
- Missing sensitive-action approval.

<a id="precedence-decision-unresolved"></a>
### Precedence 15: `DECISION_UNRESOLVED`

Applies to:
- Existing user judgment is not usable.

<a id="precedence-autonomy-boundary-exceeded"></a>
### Precedence 16: `AUTONOMY_BOUNDARY_EXCEEDED`

Applies to:
- Autonomy boundary exceeded.

<a id="precedence-decision-required"></a>
### Precedence 17: `DECISION_REQUIRED`

Applies to:
- New user-owned judgment required.

<a id="precedence-capability-insufficient"></a>
### Precedence 18: `CAPABILITY_INSUFFICIENT`

Applies to:
- Missing surface capability.

<a id="precedence-evidence-insufficient"></a>
### Precedence 19: `EVIDENCE_INSUFFICIENT`

Applies to:
- Evidence coverage insufficient.

<a id="precedence-residual-risk-not-visible"></a>
### Precedence 20: `RESIDUAL_RISK_NOT_VISIBLE`

Applies to:
- Close-relevant risk not visible.

<a id="precedence-acceptance-required"></a>
### Precedence 21: `ACCEPTANCE_REQUIRED`

Applies to:
- Final acceptance required or incompatible.

<a id="precedence-projection-stale"></a>
### Precedence 22: `PROJECTION_STALE`

Applies to:
- Readable view stale or failed.

<a id="precedence-artifact-missing"></a>
### Precedence 23: `ARTIFACT_MISSING`

Applies to:
- Persistent artifact missing, unavailable, unusable, or failed.

<a id="precedence-validator-failed"></a>
### Precedence 24: `VALIDATOR_FAILED`

Applies to:
- Typed fallback when no more specific supported code applies.

<a id="state-version-conflict-precedence-exclusion"></a>
### `STATE_VERSION_CONFLICT` precedence exclusion

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- A rejected response is selected because stale `expected_state_version` prevents the method from proceeding.

State effect:
- No committed operation proceeds.
- No owner state mutation occurs.

Selection boundary:
- `STATE_VERSION_CONFLICT` is not selected as `MethodResult.base.errors[0]`, `CloseTaskResult(close_state=blocked).errors[0]`, `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, or `PlannedBlocker.code`.

Related conflict details:
- Stale `WriteAuthorization.basis_state_version` and idempotency request-hash conflicts are covered in [State version conflict](#state-conflict-behavior).

<a id="idempotency"></a>
<a id="state-conflict-behavior"></a>

## State version conflict

| Conflict case | Detail section |
|---|---|
| stale `expected_state_version` | [Stale `expected_state_version`](#state-conflict-expected-state-version) |
| stale `WriteAuthorization.basis_state_version` | [Stale Write Authorization basis](#state-conflict-write-authorization-basis) |
| idempotency request-hash conflict | [Idempotency request-hash conflict](#state-conflict-idempotency-hash) |

`STATE_VERSION_CONFLICT` has one baseline meaning: a project-wide pre-commit freshness or idempotency conflict.

Conflict routing boundary:

| Boundary | This document's rule | Neighbor owner |
|---|---|---|
| Public code meaning | Select `STATE_VERSION_CONFLICT` for the conflict cases below. | Public code meanings: [API error codes](error-codes.md). |
| Response path | Use `ToolRejectedResponse.errors[]` for these conflicts. | Response branch routing: [API error routing](error-routing.md). |
| Result, blocker, and close-readiness boundary paths | Do not use `STATE_VERSION_CONFLICT` as a blocker code, dry-run preview, `MethodResult.decision`, `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, or `PlannedBlocker.code`. | Boundary routing: [API blocker routing](blocker-routing.md). Method behavior: [`harness.close_task`](method-close-task.md). |
| Detail fields | Use the state-conflict detail-field family for these conflicts. | Machine-readable fields: [API error details](error-details.md#state-conflict-detail-fields). |

<a id="state-conflict-expected-state-version"></a>
### Stale `expected_state_version`

Condition:
- `ToolEnvelope.expected_state_version` is older than `project_state.state_version`.

Public code:
- `STATE_VERSION_CONFLICT`

Response path:
- `ToolRejectedResponse.errors[]`

State effect:
- No committed operation proceeds.
- No owner state mutation occurs.

Detail fields:
- Use [State conflict detail fields](error-details.md#state-conflict-detail-fields).

<a id="state-conflict-write-authorization-basis"></a>
### Stale Write Authorization basis

Condition:
- `WriteAuthorization.basis_state_version` is stale before consumption.

Public code:
- `STATE_VERSION_CONFLICT`

Response path:
- `ToolRejectedResponse.errors[]`

State effect:
- No committed operation proceeds.
- No owner state mutation occurs.
- The Write Authorization is not consumed.

Detail fields:
- Use [State conflict detail fields](error-details.md#state-conflict-detail-fields).

<a id="state-conflict-idempotency-hash"></a>
### Idempotency request-hash conflict

Condition:
- The same `idempotency_key` is reused with a different request hash.

Public code:
- `STATE_VERSION_CONFLICT`

Response path:
- `ToolRejectedResponse.errors[]`

State effect:
- No committed operation proceeds.
- No owner state mutation occurs.

Detail fields:
- Use [State conflict detail fields](error-details.md#state-conflict-detail-fields).
