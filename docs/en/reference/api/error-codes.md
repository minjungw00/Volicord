# API error codes

This document owns public `ErrorCode` identifiers, meanings, and occurrence summaries for Volicord API responses.

Use it to answer what a public code means and where that code may appear. Use adjacent owners for selection order, branch routing, schemas, storage effects, security guarantees, and display wording.

## Owner boundaries

Owned here:

- The public `ErrorCode` value set.
- Public meanings and allowed public occurrence paths for each code.
- Whether a code may appear in `ToolRejectedResponse.errors[]` or owner-defined result paths.

Adjacent owners:

- Primary-code selection and state-version conflict behavior; see [API error precedence](error-precedence.md).
- Rejected-response, blocked-result, and `dry_run` branch routing; see [API error routing](error-routing.md).
- Close-readiness blocker/API response boundary; see [API blocker routing](blocker-routing.md).
- Method-specific behavior; see [`volicord.close_task`](method-close-task.md) and other method owners.
- `ToolError.details` fields and helper values; see [API error details](error-details.md).
- Common response branch shapes; see [API Schema Core](schema-core.md).
- Display wording only; see [Template Bodies](../template-bodies.md).
- Storage effects; see [Storage Effects](../storage-effects.md).

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

## Occurrence path summary

| Occurrence path | Rule |
|---|---|
| Rejected-response errors | Public `ErrorCode` values may appear in `ToolRejectedResponse.errors[]` for rejected public API requests. Branch routing belongs to [API error routing](error-routing.md). |
| Owner-defined result paths | A method, schema, or close-readiness owner may define whether a public error-code family appears on an owner-defined result path. That use does not change the public meaning owned here. |
| Error/blocker boundary | See [API blocker routing](blocker-routing.md) for the owner boundary between public API errors and `CloseReadinessBlocker` data. |

<a id="errorcode-validation-failed"></a>
### `VALIDATION_FAILED`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- Invalid payload shape, enum value, activation rule, profile validation, or artifact input shape.

<a id="errorcode-state-version-conflict"></a>
### `STATE_VERSION_CONFLICT`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- A public freshness or idempotency conflict is present. Stale `expected_state_version` is the request-state form.

Notes:
- Stale `WriteAuthorization.basis_state_version` and idempotency request-hash conflicts are covered in [State version conflict](error-precedence.md#state-conflict-behavior).

<a id="errorcode-mcp-unavailable"></a>
### `MCP_UNAVAILABLE`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- Required Core, MCP, store, typed owner state, or surface reachability is unavailable. This includes corrupt or unreadable persisted typed owner state that a public method needs in order to evaluate authority, lifecycle, scope, evidence, completion, close readiness, or write compatibility.

<a id="errorcode-local-access-mismatch"></a>
### `LOCAL_ACCESS_MISMATCH`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- Reachable local access does not match the registered transport, session, binding, project, or surface instance, or access was revoked.

<a id="errorcode-no-active-task"></a>
### `NO_ACTIVE_TASK`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- A `Task` is required but no current or addressed `Task` is available.

<a id="errorcode-no-active-change-unit"></a>
### `NO_ACTIVE_CHANGE_UNIT`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- A write-capable or close-relevant operation lacks a current Change Unit with scope.

<a id="errorcode-baseline-stale"></a>
### `BASELINE_STALE`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- The baseline no longer matches the repository state required by the operation.

<a id="errorcode-scope-required"></a>
### `SCOPE_REQUIRED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Scope confirmation is required before the requested write or action can proceed.

<a id="errorcode-scope-violation"></a>
### `SCOPE_VIOLATION`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Intended or observed paths or sensitive categories exceed current scope or stored authorized scope.

<a id="errorcode-write-authorization-required"></a>
### `WRITE_AUTHORIZATION_REQUIRED`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- A write-capable Run lacks a required `Write Authorization`.

<a id="errorcode-write-authorization-invalid"></a>
### `WRITE_AUTHORIZATION_INVALID`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- Supplied `Write Authorization` is expired, revoked, consumed, or incompatible for a non-version reason.

Notes:
- Expired `Write Authorization` use stays on this code with `ToolError.details.authorization_reason=expired`.
- Stale `WriteAuthorization.basis_state_version` is routed through `STATE_VERSION_CONFLICT`, not this code.

<a id="errorcode-approval-denied"></a>
### `APPROVAL_DENIED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Required sensitive-action approval was denied.

<a id="errorcode-approval-expired"></a>
### `APPROVAL_EXPIRED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Required sensitive-action approval expired or drifted from scope or baseline.

<a id="errorcode-approval-required"></a>
### `APPROVAL_REQUIRED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Sensitive-action approval is required before proceeding.

<a id="errorcode-decision-unresolved"></a>
### `DECISION_UNRESOLVED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- A relevant user-owned judgment is pending, deferred without coverage, rejected, stale, superseded, incompatible, or otherwise cannot satisfy the owner-defined decision requirement.

<a id="errorcode-autonomy-boundary-exceeded"></a>
### `AUTONOMY_BOUNDARY_EXCEEDED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- The intended operation exceeds the current Change Unit Autonomy Boundary.

<a id="errorcode-decision-required"></a>
### `DECISION_REQUIRED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- A blocking user-owned judgment is required before proceeding.

<a id="errorcode-capability-insufficient"></a>
### `CAPABILITY_INSUFFICIENT`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- The surface is recognized but lacks a required access class, observation, capture, guarantee support, or supported behavior.

<a id="errorcode-evidence-insufficient"></a>
### `EVIDENCE_INSUFFICIENT`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Required evidence coverage is absent, partial, stale, or blocked.

<a id="errorcode-residual-risk-not-visible"></a>
### `RESIDUAL_RISK_NOT_VISIBLE`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Known close-relevant residual risk has not been made visible before final acceptance or close.

<a id="errorcode-acceptance-required"></a>
### `ACCEPTANCE_REQUIRED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Required final acceptance is pending, rejected, or incompatible with the visible result basis.

<a id="errorcode-projection-stale"></a>
### `PROJECTION_STALE`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- A requested readable status or view is stale or failed.

<a id="errorcode-artifact-missing"></a>
### `ARTIFACT_MISSING`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- A referenced persistent artifact is missing, unavailable, unusable for the close basis, or failed integrity/metadata checks.

<a id="errorcode-validator-failed"></a>
### `VALIDATOR_FAILED`

Used in:
- `ToolRejectedResponse.errors[]`
- Owner-defined result paths

Condition:
- Fallback when a required validator or blocker check failed and no more specific typed code applies.
