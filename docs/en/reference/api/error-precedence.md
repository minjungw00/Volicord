# API error precedence

This document owns primary public-error selection when more than one public error candidate exists. It also owns public stale-state and idempotency conflict behavior for `STATE_VERSION_CONFLICT`.

Use it to choose the primary public code for an error-bearing branch. Use adjacent owners for code meanings, branch routing, schemas, storage, and display wording.

## Owner boundaries

Owned here:

- The primary `errors[0]` selection order for error-bearing branches.
- The `STATE_VERSION_CONFLICT` exclusion from result and blocker code paths.
- Public stale `expected_state_version`, stale `WriteAuthorization.basis_state_version`, and idempotency request-hash conflict behavior.

Adjacent owners:

- Public code meanings outside precedence selection; see [API error codes](error-codes.md).
- API response branch routing; see [API error routing](error-routing.md).
- Close-readiness blocker/API response boundary; see [API blocker routing](blocker-routing.md).
- Method-specific behavior; see [`harness.close_task`](method-close-task.md) and other method owners.
- Machine-readable conflict detail fields; see [API error details](error-details.md#state-conflict-detail-fields).
- Storage replay rows and state clocks; see [Storage Versioning](../storage-versioning.md).
- Display wording only; see [Template Bodies](../template-bodies.md).

<a id="primary-error-code-precedence"></a>

## Error precedence

When an error-bearing branch has non-empty `errors`, `errors[0]` is the primary public code selected by this order unless a method owner defines a stricter method-specific order. This table defines order only; public code meanings stay in [API error codes](error-codes.md).

| Precedence | Primary `ErrorCode` | Meaning owner |
|---:|---|---|
| <a id="precedence-validation-failed"></a>1 | `VALIDATION_FAILED` | [`VALIDATION_FAILED`](error-codes.md#errorcode-validation-failed) |
| 2 | `STATE_VERSION_CONFLICT` | [`STATE_VERSION_CONFLICT` conflict selection](#state-version-conflict-precedence-exclusion) |
| <a id="precedence-mcp-unavailable"></a>3 | `MCP_UNAVAILABLE` | [`MCP_UNAVAILABLE`](error-codes.md#errorcode-mcp-unavailable) |
| <a id="precedence-local-access-mismatch"></a>4 | `LOCAL_ACCESS_MISMATCH` | [`LOCAL_ACCESS_MISMATCH`](error-codes.md#errorcode-local-access-mismatch) |
| <a id="precedence-no-active-task"></a>5 | `NO_ACTIVE_TASK` | [`NO_ACTIVE_TASK`](error-codes.md#errorcode-no-active-task) |
| <a id="precedence-no-active-change-unit"></a>6 | `NO_ACTIVE_CHANGE_UNIT` | [`NO_ACTIVE_CHANGE_UNIT`](error-codes.md#errorcode-no-active-change-unit) |
| <a id="precedence-baseline-stale"></a>7 | `BASELINE_STALE` | [`BASELINE_STALE`](error-codes.md#errorcode-baseline-stale) |
| <a id="precedence-scope-required"></a>8 | `SCOPE_REQUIRED` | [`SCOPE_REQUIRED`](error-codes.md#errorcode-scope-required) |
| <a id="precedence-scope-violation"></a>9 | `SCOPE_VIOLATION` | [`SCOPE_VIOLATION`](error-codes.md#errorcode-scope-violation) |
| <a id="precedence-write-authorization-required"></a>10 | `WRITE_AUTHORIZATION_REQUIRED` | [`WRITE_AUTHORIZATION_REQUIRED`](error-codes.md#errorcode-write-authorization-required) |
| <a id="precedence-write-authorization-invalid"></a>11 | `WRITE_AUTHORIZATION_INVALID` | [`WRITE_AUTHORIZATION_INVALID`](error-codes.md#errorcode-write-authorization-invalid) |
| <a id="precedence-approval-denied"></a>12 | `APPROVAL_DENIED` | [`APPROVAL_DENIED`](error-codes.md#errorcode-approval-denied) |
| <a id="precedence-approval-expired"></a>13 | `APPROVAL_EXPIRED` | [`APPROVAL_EXPIRED`](error-codes.md#errorcode-approval-expired) |
| <a id="precedence-approval-required"></a>14 | `APPROVAL_REQUIRED` | [`APPROVAL_REQUIRED`](error-codes.md#errorcode-approval-required) |
| <a id="precedence-decision-unresolved"></a>15 | `DECISION_UNRESOLVED` | [`DECISION_UNRESOLVED`](error-codes.md#errorcode-decision-unresolved) |
| <a id="precedence-autonomy-boundary-exceeded"></a>16 | `AUTONOMY_BOUNDARY_EXCEEDED` | [`AUTONOMY_BOUNDARY_EXCEEDED`](error-codes.md#errorcode-autonomy-boundary-exceeded) |
| <a id="precedence-decision-required"></a>17 | `DECISION_REQUIRED` | [`DECISION_REQUIRED`](error-codes.md#errorcode-decision-required) |
| <a id="precedence-capability-insufficient"></a>18 | `CAPABILITY_INSUFFICIENT` | [`CAPABILITY_INSUFFICIENT`](error-codes.md#errorcode-capability-insufficient) |
| <a id="precedence-evidence-insufficient"></a>19 | `EVIDENCE_INSUFFICIENT` | [`EVIDENCE_INSUFFICIENT`](error-codes.md#errorcode-evidence-insufficient) |
| <a id="precedence-residual-risk-not-visible"></a>20 | `RESIDUAL_RISK_NOT_VISIBLE` | [`RESIDUAL_RISK_NOT_VISIBLE`](error-codes.md#errorcode-residual-risk-not-visible) |
| <a id="precedence-acceptance-required"></a>21 | `ACCEPTANCE_REQUIRED` | [`ACCEPTANCE_REQUIRED`](error-codes.md#errorcode-acceptance-required) |
| <a id="precedence-projection-stale"></a>22 | `PROJECTION_STALE` | [`PROJECTION_STALE`](error-codes.md#errorcode-projection-stale) |
| <a id="precedence-artifact-missing"></a>23 | `ARTIFACT_MISSING` | [`ARTIFACT_MISSING`](error-codes.md#errorcode-artifact-missing) |
| <a id="precedence-validator-failed"></a>24 | `VALIDATOR_FAILED` | [`VALIDATOR_FAILED`](error-codes.md#errorcode-validator-failed) |

<a id="state-version-conflict-precedence-exclusion"></a>
### `STATE_VERSION_CONFLICT` precedence exclusion

Selection condition:
- A rejected response is selected because a stale `expected_state_version`, stale `WriteAuthorization.basis_state_version`, or idempotency request-hash conflict prevents the method from proceeding.

Selection boundary:
- `STATE_VERSION_CONFLICT` is not selected as `MethodResult.base.errors[0]`, `CloseTaskResult(close_state=blocked).errors[0]`, `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, or `PlannedBlocker.code`.

Related owner:
- Machine-readable fields for these conflicts belong to [API error details](error-details.md#state-conflict-detail-fields).

<a id="idempotency"></a>
<a id="state-conflict-behavior"></a>

## State version conflict

| Conflict case | Detail section |
|---|---|
| stale `expected_state_version` | [Stale `expected_state_version`](#state-conflict-expected-state-version) |
| stale `WriteAuthorization.basis_state_version` | [Stale `Write Authorization` basis](#state-conflict-write-authorization-basis) |
| idempotency request-hash conflict | [Idempotency request-hash conflict](#state-conflict-idempotency-hash) |

For precedence, these conflict cases select `STATE_VERSION_CONFLICT` as a project-wide pre-commit freshness or idempotency conflict.

Conflict routing boundary:

| Boundary | This document's rule | Neighbor owner |
|---|---|---|
| Conflict selection | Select `STATE_VERSION_CONFLICT` for the conflict cases below. | Public code meanings: [API error codes](error-codes.md). |
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

Detail fields:
- Use [State conflict detail fields](error-details.md#state-conflict-detail-fields).

<a id="state-conflict-write-authorization-basis"></a>
### Stale `Write Authorization` basis

Condition:
- `WriteAuthorization.basis_state_version` is stale before consumption.

Public code:
- `STATE_VERSION_CONFLICT`

Response path:
- `ToolRejectedResponse.errors[]`

Consumption boundary:
- The stale `Write Authorization` is not consumed.

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

Detail fields:
- Use [State conflict detail fields](error-details.md#state-conflict-detail-fields).
