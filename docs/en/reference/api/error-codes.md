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
- Product-wide failure-category meanings; see the
  [Failure Model](../failure-model.md).

<a id="error-taxonomy"></a>

## Public `ErrorCode` summary

| Public `ErrorCode` | Detail section |
|---|---|
| `VALIDATION_FAILED` | [`VALIDATION_FAILED`](#errorcode-validation-failed) |
| `UNSUPPORTED_CONTRACT` | [`UNSUPPORTED_CONTRACT`](#errorcode-unsupported-contract) |
| `PERSISTED_DATA_CORRUPT` | [`PERSISTED_DATA_CORRUPT`](#errorcode-persisted-data-corrupt) |
| `STATE_VERSION_CONFLICT` | [`STATE_VERSION_CONFLICT`](#errorcode-state-version-conflict) |
| `MCP_UNAVAILABLE` | [`MCP_UNAVAILABLE`](#errorcode-mcp-unavailable) |
| `INVOCATION_CONTEXT_MISMATCH` | [`INVOCATION_CONTEXT_MISMATCH`](#errorcode-invocation-context-mismatch) |
| `OPERATION_RESULT_UNAVAILABLE` | [`OPERATION_RESULT_UNAVAILABLE`](#errorcode-operation-result-unavailable) |
| `NO_ACTIVE_TASK` | [`NO_ACTIVE_TASK`](#errorcode-no-active-task) |
| `NO_ACTIVE_CHANGE_UNIT` | [`NO_ACTIVE_CHANGE_UNIT`](#errorcode-no-active-change-unit) |
| `BASELINE_STALE` | [`BASELINE_STALE`](#errorcode-baseline-stale) |
| `SCOPE_REQUIRED` | [`SCOPE_REQUIRED`](#errorcode-scope-required) |
| `SCOPE_VIOLATION` | [`SCOPE_VIOLATION`](#errorcode-scope-violation) |
| `WRITE_TICKET_REQUIRED` | [`WRITE_TICKET_REQUIRED`](#errorcode-write-ticket-required) |
| `WRITE_TICKET_INVALID` | [`WRITE_TICKET_INVALID`](#errorcode-write-ticket-invalid) |
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

Required failure category:
- `rejected`

Boundary:
- Malformed untrusted request data uses this code. A supported persisted owner
  contract whose stored data is invalid uses `PERSISTED_DATA_CORRUPT`; an exact
  external contract or host artifact that is not supported uses
  `UNSUPPORTED_CONTRACT`.

<a id="errorcode-unsupported-contract"></a>
### `UNSUPPORTED_CONTRACT`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- An exact external contract descriptor or other boundary format is not an
  exact registered supported contract. No fallback adapter, decoder, version,
  or closest-known format is selected.

Required failure category:
- `unsupported_contract`

Required details:
- Use the applicable `ToolError.details.reason` value from
  [API error details](error-details.md#reason), including
  `unsupported_external_contract` for that domain.

<a id="errorcode-persisted-data-corrupt"></a>
### `PERSISTED_DATA_CORRUPT`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- Persisted or trusted owner data claims a supported contract but violates its
  schema, type, canonical encoding, or cross-field invariants. A dependent
  operation fails closed before authority derivation, policy evaluation,
  successful effects, or dependent state mutation.

Required failure category:
- `corrupt`

Required details:
- Use the applicable `ToolError.details.reason` value from
  [API error details](error-details.md#reason).

<a id="errorcode-state-version-conflict"></a>
### `STATE_VERSION_CONFLICT`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- A public freshness or idempotency conflict is present. Stale `expected_state_version` is the request-state form.

Notes:
- Idempotency request-hash conflicts are covered in [State version conflict](error-precedence.md#state-conflict-behavior). `WriteTicket.basis_state_version` is audit metadata and never selects this error by itself.

<a id="errorcode-mcp-unavailable"></a>
### `MCP_UNAVAILABLE`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- Required Core, MCP, Store, owner-state read, or Agent Connection reachability
  is currently unavailable, and the available data does not establish corrupt
  persisted data or an unsupported contract.

Required failure category:
- `unavailable`

Boundary:
- Known persisted-data corruption uses `PERSISTED_DATA_CORRUPT`. An unknown
  exact boundary contract uses `UNSUPPORTED_CONTRACT`.

<a id="errorcode-invocation-context-mismatch"></a>
### `INVOCATION_CONTEXT_MISMATCH`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- The invocation context is incompatible with the requested method, replay record, `actor_source`, `operation_category`, or adapter-derived execution context.

<a id="errorcode-operation-result-unavailable"></a>
### `OPERATION_RESULT_UNAVAILABLE`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- The exact historical operation result selected by an otherwise valid
  `OperationResultRef` and cursor is missing, ineligible, or otherwise
  unavailable for bounded retrieval without evidence of persisted-data
  corruption.

Required failure category:
- `unavailable`

Notes:
- Access-context incompatibility remains `INVOCATION_CONTEXT_MISMATCH` and is
  selected before this code so the unavailable-result branch does not grant or
  describe access.
- Store reachability remains `MCP_UNAVAILABLE`; known corrupt typed owner state
  or stored-byte integrity failure uses `PERSISTED_DATA_CORRUPT`.

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

Required failure category for the structural rejected-response path:
- `rejected`

Method-specific detail:
- `volicord.prepare_write` preserves this public code and uses
  `ToolError.details.reason=current_change_unit_required` before policy
  evaluation. It is not converted to `VALIDATION_FAILED` or a synthetic Change
  Unit.

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

<a id="errorcode-write-ticket-required"></a>
### `WRITE_TICKET_REQUIRED`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- A product-write Run or an effective `sensitive` Run lacks its required write ticket.

<a id="errorcode-write-ticket-invalid"></a>
### `WRITE_TICKET_INVALID`

Used in:
- `ToolRejectedResponse.errors[]`

Condition:
- A supplied ticket is consumed, revoked, explicitly invalidated, outside its allowed path prefixes, missing the current normalized project write-authority binding, or incompatible with that authority, its state-bound Task, Change Unit, scope revision, baseline, workspace, approval basis, or optional idle timeout.

Notes:
- State-bound invalidation and attempt mismatch stay on this code with stable `ToolError.details.write_ticket_reason` values.
- `basis_state_version` mismatch and unrelated state-version increments do not invalidate a ticket and do not select an error.

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
- The invocation context is recognized, but the requested operation, observation, capture, guarantee display, or supported behavior is not available for that context.

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
