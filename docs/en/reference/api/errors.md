# API errors

This document describes Harness Server behavior for planning and review. It does not mean this documentation repository implements an MCP server or any runtime behavior.

## Owns / Does not own

This document owns:

- Public `ErrorCode` identifiers: the public code set, public meanings, and allowed public paths.
- Error precedence: how to choose `errors[0]` when a response branch carries more than one public error.
- Error vs blocker routing: where a condition belongs across rejected responses, blocked results, and dry-run previews.
- `STATE_VERSION_CONFLICT`: public stale-state and idempotency-conflict behavior.
- User-facing labels: display guidance for public errors.

This document does not own:

- Method payload schemas, response field shapes, and common envelopes:
  - [API Schema Core](schema-core.md)
  - method owner documents routed from [API Methods](methods.md)
  - API schema owners
- Core gates, user judgments, and close-readiness order:
  - [Core Model](../core-model.md)
  - [User-judgment methods](method-user-judgment.md)
  - [Close-task method](method-close-task.md)
- `CloseReadinessBlocker`, `WriteDecisionReason`, `PlannedBlocker`, and value-set field definitions:
  - [API State Schemas](schema-state.md)
  - [API Schema Core](schema-core.md)
  - [API Value Sets](schema-value-sets.md)
- Storage rows, replay rows, DDL, locks, migrations, and storage effects:
  - [Storage Records](../storage-records.md)
  - [Storage Effects](../storage-effects.md)
  - [Storage Versioning](../storage-versioning.md)
- Security guarantee wording and access-boundary claims:
  - [Security](../security.md)

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

<a id="error-taxonomy"></a>

## Public `ErrorCode` summary

| Public `ErrorCode` | Detail section |
|---|---|
| `VALIDATION_FAILED` | [`VALIDATION_FAILED`](#errorcode-validation-failed) |
| `STATE_VERSION_CONFLICT` | [`STATE_VERSION_CONFLICT`](#errorcode-state-version-conflict) |
| `MCP_UNAVAILABLE` | [`MCP_UNAVAILABLE`](#errorcode-mcp-unavailable) |
| `LOCAL_ACCESS_MISMATCH` | [`LOCAL_ACCESS_MISMATCH`](#errorcode-local-access-mismatch) |
| `NO_ACTIVE_TASK` | [`NO_ACTIVE_TASK`](#errorcode-no-active-task) |
| `NO_ACTIVE_CHANGE_UNIT` | [`NO_ACTIVE_CHANGE_UNIT`](#errorcode-no-active-change-unit) |
| `BASELINE_STALE` | [`BASELINE_STALE`](#errorcode-baseline-stale) |
| `SCOPE_REQUIRED` | [`SCOPE_REQUIRED`](#errorcode-scope-required) |
| `SCOPE_VIOLATION` | [`SCOPE_VIOLATION`](#errorcode-scope-violation) |
| `WRITE_AUTHORIZATION_REQUIRED` | [`WRITE_AUTHORIZATION_REQUIRED`](#errorcode-write-authorization-required) |
| `WRITE_AUTHORIZATION_INVALID` | [`WRITE_AUTHORIZATION_INVALID`](#errorcode-write-authorization-invalid) |
| `APPROVAL_DENIED` | [`APPROVAL_DENIED`](#errorcode-approval-denied) |
| `APPROVAL_EXPIRED` | [`APPROVAL_EXPIRED`](#errorcode-approval-expired) |
| `APPROVAL_REQUIRED` | [`APPROVAL_REQUIRED`](#errorcode-approval-required) |
| `DECISION_UNRESOLVED` | [`DECISION_UNRESOLVED`](#errorcode-decision-unresolved) |
| `AUTONOMY_BOUNDARY_EXCEEDED` | [`AUTONOMY_BOUNDARY_EXCEEDED`](#errorcode-autonomy-boundary-exceeded) |
| `DECISION_REQUIRED` | [`DECISION_REQUIRED`](#errorcode-decision-required) |
| `CAPABILITY_INSUFFICIENT` | [`CAPABILITY_INSUFFICIENT`](#errorcode-capability-insufficient) |
| `EVIDENCE_INSUFFICIENT` | [`EVIDENCE_INSUFFICIENT`](#errorcode-evidence-insufficient) |
| `RESIDUAL_RISK_NOT_VISIBLE` | [`RESIDUAL_RISK_NOT_VISIBLE`](#errorcode-residual-risk-not-visible) |
| `ACCEPTANCE_REQUIRED` | [`ACCEPTANCE_REQUIRED`](#errorcode-acceptance-required) |
| `PROJECTION_STALE` | [`PROJECTION_STALE`](#errorcode-projection-stale) |
| `ARTIFACT_MISSING` | [`ARTIFACT_MISSING`](#errorcode-artifact-missing) |
| `VALIDATOR_FAILED` | [`VALIDATOR_FAILED`](#errorcode-validator-failed) |

<a id="errorcode-validation-failed"></a>
### `VALIDATION_FAILED`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- Invalid payload shape, enum value, activation rule, profile validation, or artifact input shape.

State effect:
- No committed operation proceeds.
- No owner state mutation occurs.

Not allowed:
- Do not use this as a blocker code for request rejection.

<a id="errorcode-state-version-conflict"></a>
### `STATE_VERSION_CONFLICT`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- `expected_state_version` is stale.

State effect:
- No committed operation proceeds.
- No owner state mutation occurs.

Not allowed:
- Do not use this as a close-readiness blocker code.

Related conflict details:
- Stale `WriteAuthorization.basis_state_version` and idempotency request-hash conflicts are covered in [State version conflict](#state-conflict-behavior).

<a id="errorcode-mcp-unavailable"></a>
### `MCP_UNAVAILABLE`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- Required Core, MCP, or surface reachability is unavailable.

State effect:
- No committed operation proceeds.
- No owner state mutation occurs.

Not allowed:
- Do not use this as a blocker code for request rejection.

<a id="errorcode-local-access-mismatch"></a>
### `LOCAL_ACCESS_MISMATCH`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- Reachable local access does not match the registered transport, session, binding, project, or surface instance, or access was revoked.

State effect:
- No committed operation proceeds.
- No owner state mutation occurs.

Not allowed:
- Do not use this as a blocker code for request rejection.

<a id="errorcode-no-active-task"></a>
### `NO_ACTIVE_TASK`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- A Task is required but none is active or addressed.

State effect:
- No committed operation proceeds.
- No owner state mutation occurs.

Not allowed:
- Do not use this as a blocker code unless the canonical method or schema owner explicitly allows that use.

<a id="errorcode-no-active-change-unit"></a>
### `NO_ACTIVE_CHANGE_UNIT`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- A write-capable or close-relevant operation lacks an active scoped Change Unit.

State effect:
- Rejection path: no committed operation proceeds and no owner state mutation occurs.
- Owner-defined result paths: only the owning method or schema may define committed result effects.

Not allowed:
- Do not use this as a blocker code unless the canonical method or schema owner explicitly allows that use.

<a id="errorcode-baseline-stale"></a>
### `BASELINE_STALE`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- The baseline no longer matches the repository state required by the operation.

State effect:
- Rejection path: no committed operation proceeds and no owner state mutation occurs.
- Owner-defined result paths: only the owning method or schema may define committed result effects.

Not allowed:
- Do not use this as a blocker code unless the canonical method or schema owner explicitly allows that use.

<a id="errorcode-scope-required"></a>
### `SCOPE_REQUIRED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Scope confirmation is required before the requested write or action can proceed.

State effect:
- Rejection path: no committed operation proceeds and no owner state mutation occurs.
- Owner-defined result paths: only the owning method or schema may define committed result effects.

Not allowed:
- Do not use this as a blocker code unless the canonical method or schema owner explicitly allows that use.

<a id="errorcode-scope-violation"></a>
### `SCOPE_VIOLATION`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Intended or observed paths or sensitive categories exceed active scope or stored authorized scope.

State effect:
- Rejection path: no committed operation proceeds and no owner state mutation occurs.
- Owner-defined result paths: only the owning method or schema may define committed result effects.

Not allowed:
- Do not use this as a blocker code unless the canonical method or schema owner explicitly allows that use.

<a id="errorcode-write-authorization-required"></a>
### `WRITE_AUTHORIZATION_REQUIRED`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- A write-capable Run lacks a required Write Authorization.

State effect:
- No committed operation proceeds.
- No owner state mutation occurs.

Not allowed:
- Do not use this as a blocker code unless the canonical method or schema owner explicitly allows that use.

<a id="errorcode-write-authorization-invalid"></a>
### `WRITE_AUTHORIZATION_INVALID`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- Supplied Write Authorization is expired, revoked, consumed, or incompatible for a non-version reason.

State effect:
- No committed operation proceeds.
- No owner state mutation occurs.

Not allowed:
- Do not use this as a blocker code unless the canonical method or schema owner explicitly allows that use.

<a id="errorcode-approval-denied"></a>
### `APPROVAL_DENIED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Required sensitive-action approval was denied.

State effect:
- Rejection path: no committed operation proceeds and no owner state mutation occurs.
- Owner-defined result paths: only the owning method or schema may define committed result effects.

Not allowed:
- Do not use this as a blocker code unless the canonical method or schema owner explicitly allows that use.

<a id="errorcode-approval-expired"></a>
### `APPROVAL_EXPIRED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Required sensitive-action approval expired or drifted from scope or baseline.

State effect:
- Rejection path: no committed operation proceeds and no owner state mutation occurs.
- Owner-defined result paths: only the owning method or schema may define committed result effects.

Not allowed:
- Do not use this as a blocker code unless the canonical method or schema owner explicitly allows that use.

<a id="errorcode-approval-required"></a>
### `APPROVAL_REQUIRED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Sensitive-action approval is required before proceeding.

State effect:
- Rejection path: no committed operation proceeds and no owner state mutation occurs.
- Owner-defined result paths: only the owning method or schema may define committed result effects.

Not allowed:
- Do not use this as a blocker code unless the canonical method or schema owner explicitly allows that use.

<a id="errorcode-decision-unresolved"></a>
### `DECISION_UNRESOLVED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- A relevant user judgment is pending, deferred without coverage, rejected, blocked, stale, superseded, or incompatible.

State effect:
- Rejection path: no committed operation proceeds and no owner state mutation occurs.
- Owner-defined result paths: only the owning method or schema may define committed result effects.

Not allowed:
- Do not use this as a blocker code unless the canonical method or schema owner explicitly allows that use.

<a id="errorcode-autonomy-boundary-exceeded"></a>
### `AUTONOMY_BOUNDARY_EXCEEDED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- The intended operation exceeds the active Change Unit Autonomy Boundary.

State effect:
- Rejection path: no committed operation proceeds and no owner state mutation occurs.
- Owner-defined result paths: only the owning method or schema may define committed result effects.

Not allowed:
- Do not use this as a blocker code unless the canonical method or schema owner explicitly allows that use.

<a id="errorcode-decision-required"></a>
### `DECISION_REQUIRED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- A blocking user-owned judgment must be requested before proceeding.

State effect:
- Rejection path: no committed operation proceeds and no owner state mutation occurs.
- Owner-defined result paths: only the owning method or schema may define committed result effects.

Not allowed:
- Do not use this as a blocker code unless the canonical method or schema owner explicitly allows that use.

<a id="errorcode-capability-insufficient"></a>
### `CAPABILITY_INSUFFICIENT`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- The surface is recognized but lacks a required access class, observation, capture, guarantee support, or active behavior.

State effect:
- Rejection path: no committed operation proceeds and no owner state mutation occurs.
- Owner-defined result paths: only the owning method or schema may define committed result effects.

Not allowed:
- Do not use this as a blocker code unless the canonical method or schema owner explicitly allows that use.

<a id="errorcode-evidence-insufficient"></a>
### `EVIDENCE_INSUFFICIENT`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Required evidence coverage is absent, partial, stale, or blocked.

State effect:
- Rejection path: no committed operation proceeds and no owner state mutation occurs.
- Owner-defined result paths: only the owning method or schema may define committed result effects.

Not allowed:
- Do not use this as a blocker code unless the close-readiness owner explicitly allows that mapping.

<a id="errorcode-residual-risk-not-visible"></a>
### `RESIDUAL_RISK_NOT_VISIBLE`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Known close-relevant residual risk has not been made visible before final acceptance or close.

State effect:
- Rejection path: no committed operation proceeds and no owner state mutation occurs.
- Owner-defined result paths: only the owning method or schema may define committed result effects.

Not allowed:
- Do not use this as a blocker code unless the close-readiness owner explicitly allows that mapping.

<a id="errorcode-acceptance-required"></a>
### `ACCEPTANCE_REQUIRED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Required final acceptance is pending, rejected, or incompatible with the visible result basis.

State effect:
- Rejection path: no committed operation proceeds and no owner state mutation occurs.
- Owner-defined result paths: only the owning method or schema may define committed result effects.

Not allowed:
- Do not use this as a blocker code unless the close-readiness owner explicitly allows that mapping.

<a id="errorcode-projection-stale"></a>
### `PROJECTION_STALE`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- A requested readable status or view is stale or failed.

State effect:
- No committed operation proceeds.
- No owner state mutation occurs.

Not allowed:
- Do not use this by itself as a close-readiness blocker code.

<a id="errorcode-artifact-missing"></a>
### `ARTIFACT_MISSING`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- A referenced persistent artifact is missing, unavailable, unusable for the close basis, or failed integrity/metadata checks.

State effect:
- Rejection path: no committed operation proceeds and no owner state mutation occurs.
- Owner-defined result paths: only the owning method or schema may define committed result effects.

Not allowed:
- Do not use this as a blocker code unless the close-readiness owner explicitly allows that mapping.

<a id="errorcode-validator-failed"></a>
### `VALIDATOR_FAILED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Fallback when a required active validator or blocker check failed and no more specific typed code applies.

State effect:
- Rejection path: no committed operation proceeds and no owner state mutation occurs.
- Owner-defined result paths: only the owning method or schema may define committed result effects.

Not allowed:
- Do not use this fallback when a more specific active code applies.
- Do not use this as a blocker code outside the owning method or schema fallback.

`ToolError.details.authorization_reason` uses `missing`, `expired`, `stale`, `revoked`, `consumed`, or `incompatible`. A stale `WriteAuthorization.basis_state_version` uses `STATE_VERSION_CONFLICT`, not `WRITE_AUTHORIZATION_INVALID`.

`ToolError.details.artifact_input_error.reason` uses these detail helper values. They are not top-level public `ErrorCode` values; staged-handle validation failures keep the public code `VALIDATION_FAILED` unless the actual failure is request-level local access or capability verification.

| `artifact_input_error.reason` | Meaning |
|---|---|
| `staged_handle_expired` | The staged handle is past its usable lifetime. |
| `staged_handle_consumed` | The staged handle was already consumed. |
| `staged_handle_project_mismatch` | The staged handle belongs to a different project. |
| `staged_handle_task_mismatch` | The staged handle belongs to a different Task. |
| `staged_handle_surface_mismatch` | The staged handle provenance does not match the verified surface. |
| `staged_handle_checksum_mismatch` | The staged bytes do not match the expected checksum. |
| `staged_handle_size_mismatch` | The staged bytes do not match the expected size. |
| `staged_handle_not_found` | The staged handle cannot be found. |

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
- Typed fallback when no more specific active code applies.

<a id="state-version-conflict-precedence-exclusion"></a>
### `STATE_VERSION_CONFLICT` precedence exclusion

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- A rejected response is selected because stale `expected_state_version` prevents the method from proceeding.

State effect:
- No committed operation proceeds.
- No owner state mutation occurs.

Not allowed:
- Do not select `STATE_VERSION_CONFLICT` as `MethodResult.base.errors[0]`, `CloseTaskResult(close_state=blocked).errors[0]`, `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, or `PlannedBlocker.code`.

Related conflict details:
- Stale `WriteAuthorization.basis_state_version` and idempotency request-hash conflicts are covered in [State version conflict](#state-conflict-behavior).

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

<a id="idempotency"></a>
<a id="state-conflict-behavior"></a>

## State version conflict

| Conflict case | Detail section |
|---|---|
| stale `expected_state_version` | [Stale `expected_state_version`](#state-conflict-expected-state-version) |
| stale `WriteAuthorization.basis_state_version` | [Stale Write Authorization basis](#state-conflict-write-authorization-basis) |
| idempotency request-hash conflict | [Idempotency request-hash conflict](#state-conflict-idempotency-hash) |

`STATE_VERSION_CONFLICT` has one active current MVP meaning: a project-wide pre-commit freshness or idempotency conflict.

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

Detail guidance:
- Include `state_clock: project_state.state_version`, `current_state_version`, `expected_state_version`, `project_id`, and `task_id` when available.

Not allowed:
- Do not use this as a blocker code.

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

Detail guidance:
- Identify the stale authorization basis and current `project_state.state_version`.

Not allowed:
- Do not use this as a blocker code.

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

Detail guidance:
- Identify the `idempotency_key` and request-hash mismatch without exposing sensitive request bodies.

Not allowed:
- Do not use this as a blocker code.
- Do not represent this as dry-run preview data, `MethodResult.decision`, `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, or `PlannedBlocker.code`.

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
| Unaccepted residual risk | [Unaccepted residual risk](#close-mapping-unaccepted-residual-risk) |
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
### Unaccepted residual risk

Condition:
- Residual risk is visible but not accepted.

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
- Full close-readiness evaluation order: [Core Model close readiness](../core-model.md#close_task)
- Method behavior: [`harness.close_task`](method-close-task.md)
- `CloseReadinessBlocker` shape and categories: [API State Schemas](schema-state.md) and [API Value Sets](schema-value-sets.md)

## User-facing labels

User-facing labels may differ from public error identifiers. Labels are display text, not new public codes.

| Public condition | Label detail |
|---|---|
| `VALIDATION_FAILED` | [`VALIDATION_FAILED`](#label-validation-failed) |
| `STATE_VERSION_CONFLICT` | [`STATE_VERSION_CONFLICT`](#label-state-version-conflict) |
| `MCP_UNAVAILABLE` | [`MCP_UNAVAILABLE`](#label-mcp-unavailable) |
| `LOCAL_ACCESS_MISMATCH` | [`LOCAL_ACCESS_MISMATCH`](#label-local-access-mismatch) |
| `CAPABILITY_INSUFFICIENT` | [`CAPABILITY_INSUFFICIENT`](#label-capability-insufficient) |
| `NO_ACTIVE_TASK` | [`NO_ACTIVE_TASK`](#label-no-active-task) |
| scope, boundary, or baseline codes | [Scope, boundary, or baseline label](#label-scope-boundary-baseline) |
| `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID` | [Write Authorization label](#label-write-authorization) |
| `DECISION_REQUIRED`, `DECISION_UNRESOLVED` | [Judgment label](#label-judgment) |
| `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED` | [Sensitive-action approval label](#label-sensitive-approval) |
| `EVIDENCE_INSUFFICIENT` | [`EVIDENCE_INSUFFICIENT`](#label-evidence-insufficient) |
| `ACCEPTANCE_REQUIRED` | [`ACCEPTANCE_REQUIRED`](#label-acceptance-required) |
| `RESIDUAL_RISK_NOT_VISIBLE` | [`RESIDUAL_RISK_NOT_VISIBLE`](#label-residual-risk-not-visible) |
| `PROJECTION_STALE` | [`PROJECTION_STALE`](#label-projection-stale) |
| `ARTIFACT_MISSING` | [`ARTIFACT_MISSING`](#label-artifact-missing) |
| `VALIDATOR_FAILED` | [`VALIDATOR_FAILED`](#label-validator-failed) |

<a id="label-validation-failed"></a>
### `VALIDATION_FAILED` label

Suggested label:
- invalid request.

Smallest unblocker:
- Fix the payload, enum value, activation rule, profile value, or field set before retrying.

<a id="label-state-version-conflict"></a>
### `STATE_VERSION_CONFLICT` label

Suggested label:
- state version conflict.

Smallest unblocker:
- Refresh current state and retry with the current `project_state.state_version`, or replay the original idempotent request.

<a id="label-mcp-unavailable"></a>
### `MCP_UNAVAILABLE` label

Suggested label:
- Core or surface unavailable.

Smallest unblocker:
- Reconnect or diagnose Core, MCP, and surface reachability.

<a id="label-local-access-mismatch"></a>
### `LOCAL_ACCESS_MISMATCH` label

Suggested label:
- local access mismatch.

Smallest unblocker:
- Use the registered local transport, session, or binding.
- Repair local access registration when needed.

<a id="label-capability-insufficient"></a>
### `CAPABILITY_INSUFFICIENT` label

Suggested label:
- insufficient surface capability.

Smallest unblocker:
- Use a capable surface.
- Reduce the operation or avoid the missing capability.

<a id="label-no-active-task"></a>
### `NO_ACTIVE_TASK` label

Suggested label:
- no active Task.

Smallest unblocker:
- Select or create a Task before a Task-scoped action.

<a id="label-scope-boundary-baseline"></a>
### Scope, boundary, or baseline label

Public condition:
- `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, or `BASELINE_STALE`.

Suggested label:
- scope, boundary, or baseline issue.

Smallest unblocker:
- Confirm or narrow scope.
- Update valid scope or baseline through the owner path.
- Request the needed user judgment.

<a id="label-write-authorization"></a>
### Write Authorization label

Public condition:
- `WRITE_AUTHORIZATION_REQUIRED` or `WRITE_AUTHORIZATION_INVALID`.

Suggested label:
- missing or unusable pre-write check.

Smallest unblocker:
- Call or retry `harness.prepare_write` for the exact operation, current scope, and current state.

<a id="label-judgment"></a>
### Judgment label

Public condition:
- `DECISION_REQUIRED` or `DECISION_UNRESOLVED`.

Suggested label:
- judgment needed.

Smallest unblocker:
- Request or resolve the focused `UserJudgment`.

<a id="label-sensitive-approval"></a>
### Sensitive-action approval label

Public condition:
- `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, or `APPROVAL_EXPIRED`.

Suggested label:
- sensitive-action approval needed or not usable.

Smallest unblocker:
- Request, resolve, or renew `judgment_kind=sensitive_approval`.

<a id="label-evidence-insufficient"></a>
### `EVIDENCE_INSUFFICIENT` label

Suggested label:
- evidence needed.

Smallest unblocker:
- Record, rerun, or show the missing evidence and smallest unblocker.

<a id="label-acceptance-required"></a>
### `ACCEPTANCE_REQUIRED` label

Suggested label:
- final acceptance needed.

Smallest unblocker:
- Request or resolve `judgment_kind=final_acceptance` for the visible result basis.

<a id="label-residual-risk-not-visible"></a>
### `RESIDUAL_RISK_NOT_VISIBLE` label

Suggested label:
- residual risk not visible.

Smallest unblocker:
- Show the close-relevant residual risk before final acceptance or close.

<a id="label-projection-stale"></a>
### `PROJECTION_STALE` label

Suggested label:
- stale readable view.

Smallest unblocker:
- Refresh the view before relying on it.

<a id="label-artifact-missing"></a>
### `ARTIFACT_MISSING` label

Suggested label:
- artifact issue.

Smallest unblocker:
- Restore, regenerate, replace, or reconnect the missing or unusable artifact.

<a id="label-validator-failed"></a>
### `VALIDATOR_FAILED` label

Suggested label:
- check failed.

Smallest unblocker:
- Show the specific validator or blocker when available.
- Use this fallback only when no typed code applies.

<a id="documentation-smoke-error-coverage"></a>

## Owner links

- Public `ErrorCode` values, meanings, and precedence:
  - This document.
- Response branch shape:
  - [API Schema Core](schema-core.md)
  - Applies to `ToolRejectedResponse`, `ToolDryRunResponse`, `ToolError`, `ToolResultBase`, and `DryRunSummary`.
- Method behavior, branch selection, and method-specific payloads:
  - method owner documents routed from [API Methods](methods.md)
- State and close-readiness data shapes:
  - [API State Schemas](schema-state.md)
  - Applies to `WriteDecisionReason`, `CloseReadinessBlocker`, and state summaries.
- Enum-like API values:
  - [API Value Sets](schema-value-sets.md)
  - Applies to `response_kind`, `effect_kind`, `PlannedBlocker.source_kind`, and blocker categories.
- Artifact input and reference shapes:
  - [API Artifact Schemas](schema-artifacts.md)
  - Applies to `ArtifactInput`, `ArtifactRef`, and `StagedArtifactHandle`.
- Staged-handle storage validation and artifact promotion lifecycle:
  - [Artifact Storage](../storage-artifacts.md)
- User judgments, approvals, acceptance, and residual-risk acceptance shapes:
  - [API Judgment Schemas](schema-judgment.md)
  - [Core Model](../core-model.md)
- Full close-readiness evaluation order and non-substitution rules:
  - [Core Model close readiness](../core-model.md#close_task)
- Storage effects, replay rows, state clocks, and DDL:
  - [Storage Effects](../storage-effects.md)
  - [Storage Versioning](../storage-versioning.md)
  - [Storage Records](../storage-records.md)
- Security guarantee wording and access-boundary claims:
  - [Security](../security.md)
