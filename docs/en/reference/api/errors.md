# API Errors

## What this document helps you do

Use this reference for active current MVP public error codes, primary-error precedence, blocked and dry-run behavior, `tool_invocations` replay, state conflict behavior, close blocker behavior, and user-facing label guidance.

This document describes future Harness Server behavior for planning and review. It does not mean the current documentation repository implements an MCP server.

## Current MVP Guarantee Display And Profile-Gated Claim Taxonomy

`guarantee_display.level` uses the current MVP values `cooperative` and `detective` unless a promoted profile explicitly supports a profile-gated display value. Security meaning is owned by [Security Reference: Honest guarantee display](../security.md#honest-guarantee-display), and the exact value-set boundary is owned by [API Schema Core](schema-core.md#current-mvp-value-sets).

Requesting or displaying a profile-gated guarantee display value without profile support is a claim-boundary error, not evidence that the guarantee claim is supported. Use `CAPABILITY_INSUFFICIENT` when the surface lacks the needed blocking, isolation, observation, or proof-path support, including command, network, or secret-access observation. Use `VALIDATION_FAILED` when the requested value is not valid for the active profile or request shape. Neither error implies current runtime enforcement in this documentation-only repository.

| Level or name | Error/status meaning |
|---|---|
| `cooperative` | Harness can check and record when the agent or tool follows the documented path. It is not OS permission, sandboxing, tamper-proof storage, or pre-execution blocking. |
| `detective` | Harness or the connected surface can detect, record, or report a supported observable mismatch after or during the action, after the relevant capability check has passed. It is not prevention. |
| `preventive` | Profile-gated display value name. Without promoted pre-tool blocking support for the covered operation, return a capability or validation error and lower the displayed `guarantee_display.level` value. |
| `isolated` | Profile-gated display value name. Without promoted isolation support for the named boundary, return a capability or validation error and lower the displayed `guarantee_display.level` value. |

Active MVP behavior defaults to cooperative checks with limited detective reporting only where the connected surface can honestly observe facts and the relevant capability check has passed. These security non-claims are boundary statements, not runtime errors or enforced capabilities. Close blockers are separate structured task-readiness results about user judgment, evidence, residual-risk visibility, and residual-risk acceptance state; they are not runtime proof of preventive blocking, isolation, sandboxing, or tamper-proof storage.

| Condition | Public path | Agent rule |
|---|---|---|
| `core_or_surface_unavailable` | `MCP_UNAVAILABLE` | Do not invent Harness state. Hold Harness-dependent writes, artifact body reads, and close until Core and the required surface path are reachable, or until the user explicitly chooses to proceed outside Harness. This corresponds to `VerifiedSurfaceContext.failure_reason=unavailable`. |
| `local_access_mismatch` | `LOCAL_ACCESS_MISMATCH` | Do not guess local file or command facts and do not trust a copied `surface_id`. Use the registered local transport/session/binding, repair local access registration through the owner path, or label input unverified. This corresponds to `failure_reason=mismatch` or `revoked`. |
| `missing_capability` | `CAPABILITY_INSUFFICIENT` | Use a capable surface, reduce the operation, or choose a path that does not require the missing observation, capture, local access class, blocking/isolation claim, or active behavior. Baseline `reference-local-mcp` requests that require command, network, secret-access, native artifact-capture, pre-tool-blocking, or isolation guarantees belong here unless the payload shape is invalid. This corresponds to `failure_reason=insufficient_capability`. |
| `stale_state` | `STATE_CONFLICT`, `BASELINE_STALE`, `PROJECTION_STALE`, stale `WRITE_AUTHORIZATION_INVALID` | Refresh current state, baseline, readable view, scope-update result, or pre-write check before relying on it. |
| `unsupported_surface` | `CAPABILITY_INSUFFICIENT` or `VALIDATION_FAILED` | Reduce the request, move to a capable surface, or return a blocker. Do not emulate unsupported authority with prose. |
| `out_of_scope` | `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `NO_ACTIVE_CHANGE_UNIT`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` | Hold the affected action, show the mismatch, narrow to current scope, request the specific user-owned scope judgment, or apply the resolved scope change through `harness.update_scope`. |
| `missing_judgment` | `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `ACCEPTANCE_REQUIRED` | Ask or resolve the focused active `UserJudgment`. Do not collapse product, technical, scope, sensitive approval, final acceptance, residual-risk acceptance, cancellation, or later/reserved QA waiver and verification-risk routes into one broad approval. |
| `missing_evidence` | `EVIDENCE_INSUFFICIENT`, `ARTIFACT_MISSING` | Show the affected claim, refs, evidence status, and smallest unblocker. Do not invent test results, artifact integrity, or evidence sufficiency. |
| `close_blocked` | `CloseTaskResponse.close_state=blocked` plus the primary `ErrorCode` | Return structured blockers and next actions. Do not mark the Task terminal. |
| `residual_risk_present` | `RESIDUAL_RISK_NOT_VISIBLE`, `DECISION_REQUIRED`, or `DECISION_UNRESOLVED` | Show the risk and ask `judgment_kind=residual_risk_acceptance` only when the active close or acceptance path requires it. |

<a id="error-taxonomy"></a>

## Error Taxonomy

| Code | Meaning |
|---|---|
| `VALIDATION_FAILED` | Payload shape, enum value, activation rule, or profile-specific validation failed before mutation. |
| `STATE_CONFLICT` | `expected_state_version` is stale against `project_state.state_version`, state lock ownership changed, or the same idempotency key was reused with a different canonical request. |
| `NO_ACTIVE_TASK` | A Task is required but none is active or addressed. |
| `NO_ACTIVE_CHANGE_UNIT` | A write-capable or close-relevant operation has no active scoped Change Unit. |
| `SCOPE_REQUIRED` | Scope confirmation is required before the requested write or action can proceed. |
| `SCOPE_VIOLATION` | Intended or observed product-file paths or sensitive categories exceed active scope or stored `AuthorizedAttemptScope`. |
| `WRITE_AUTHORIZATION_REQUIRED` | A write-capable Run is missing a required Write Authorization from `harness.prepare_write`. |
| `WRITE_AUTHORIZATION_INVALID` | The supplied Write Authorization is missing, expired, stale, revoked, consumed outside replay, or incompatible. |
| `DECISION_REQUIRED` | A blocking user-owned judgment must be requested before the action can proceed. |
| `DECISION_UNRESOLVED` | A relevant user judgment is pending, deferred without coverage, rejected, blocked, stale, superseded, or incompatible. |
| `AUTONOMY_BOUNDARY_EXCEEDED` | The intended operation exceeds the active Change Unit Autonomy Boundary. |
| `APPROVAL_REQUIRED` | Sensitive-action approval is required before proceeding. |
| `APPROVAL_DENIED` | The relevant sensitive-action approval was denied. |
| `APPROVAL_EXPIRED` | The relevant sensitive-action approval expired or drifted from scope/baseline. |
| `CAPABILITY_INSUFFICIENT` | The surface is recognized but cannot satisfy a required access class, observation, capture, blocking/isolation condition, guarantee claim, or active behavior. |
| `MCP_UNAVAILABLE` | Required MCP/Core or surface reachability itself is unavailable or unreachable, so the server cannot derive a usable local surface context. |
| `LOCAL_ACCESS_MISMATCH` | Registered local access expectations do not match the reachable transport/session/binding, `surface_id`/project/surface-instance pairing, or local access was revoked. |
| `EVIDENCE_INSUFFICIENT` | Required evidence coverage is absent, partial, stale, or blocked. |
| `ACCEPTANCE_REQUIRED` | Required final acceptance is pending, rejected, or not compatible with the visible result basis. |
| `PROJECTION_STALE` | A requested readable status/view is stale or failed. It is not Core state and is not a close blocker by itself. |
| `RESIDUAL_RISK_NOT_VISIBLE` | Known close-relevant residual risk has not been made visible before final acceptance or close. |
| `ARTIFACT_MISSING` | A referenced artifact is missing or failed integrity/metadata checks. |
| `BASELINE_STALE` | Baseline no longer matches the repository state required by the operation. |
| `VALIDATOR_FAILED` | Fallback when a required active validator or blocker check failed and no more specific typed code applies. In the current MVP, this is not a design-policy error. Design-quality concerns must route through an active judgment, blocker, evidence, capability, or residual-risk path, or remain advisory. |

`ToolError.details.authorization_reason` uses exactly:

```text
missing | expired | stale | revoked | consumed | incompatible
```

Use `WRITE_AUTHORIZATION_REQUIRED` with `authorization_reason=missing` when no required authorization is supplied. Use `WRITE_AUTHORIZATION_INVALID` for an existing authorization that cannot be consumed.

Use the local-access codes narrowly and keep them distinguishable. `MCP_UNAVAILABLE` is for unavailable MCP/Core or surface reachability itself, including `VerifiedSurfaceContext.failure_reason=unavailable`. `LOCAL_ACCESS_MISMATCH` is for a reachable local transport/session/binding that does not match the registered project surface, or for revoked local access, including `failure_reason=mismatch` or `revoked`. `CAPABILITY_INSUFFICIENT` is for a recognized active surface that lacks the capability needed by the requested access class or guarantee claim, including `failure_reason=insufficient_capability`. `surface_id` alone never resolves any of these errors. Do not substitute a surface-specific `UNAUTHORIZED` code for these public paths.

<a id="primary-error-code-precedence"></a>

## Primary Error Code Precedence

When `ToolResponseBase.errors` is non-empty, `errors[0]` is the primary error selected by this order unless a method section defines a stricter order. Secondary blockers may still appear in method-specific fields and `ToolError.details`.

| Precedence | Primary `ErrorCode` |
|---:|---|
| 1 | `VALIDATION_FAILED` |
| 2 | `STATE_CONFLICT` |
| 3 | `MCP_UNAVAILABLE` |
| 4 | `LOCAL_ACCESS_MISMATCH` |
| 5 | `NO_ACTIVE_TASK` |
| 6 | `NO_ACTIVE_CHANGE_UNIT` |
| 7 | `BASELINE_STALE` |
| 8 | `SCOPE_REQUIRED` |
| 9 | `SCOPE_VIOLATION` |
| 10 | `WRITE_AUTHORIZATION_REQUIRED` |
| 11 | `WRITE_AUTHORIZATION_INVALID` |
| 12 | `APPROVAL_DENIED` |
| 13 | `APPROVAL_EXPIRED` |
| 14 | `APPROVAL_REQUIRED` |
| 15 | `DECISION_UNRESOLVED` |
| 16 | `AUTONOMY_BOUNDARY_EXCEEDED` |
| 17 | `DECISION_REQUIRED` |
| 18 | `CAPABILITY_INSUFFICIENT` |
| 19 | `EVIDENCE_INSUFFICIENT` |
| 20 | `RESIDUAL_RISK_NOT_VISIBLE` |
| 21 | `ACCEPTANCE_REQUIRED` |
| 22 | `PROJECTION_STALE` |
| 23 | `ARTIFACT_MISSING` |
| 24 | `VALIDATOR_FAILED` |

<a id="blocked-and-dry-run-behavior"></a>

## Blocked And Dry-Run Behavior

A blocked response is not the same as a pre-commit failure. Core may commit a blocked response only where the method-state-effect matrix in [MVP API](mvp-api.md#active-mvp-method-behavior) explicitly allows a committed blocked response. A committed blocked response may update `blockers`, events, state version, and `tool_invocations` replay, but it must not create the authority that the blocker says is missing.

Read-only calls, including `harness.status` and `harness.close_task intent=check`, may compute and return blockers or close blockers. Those blockers are response fields only: Core must not store them, append events, create `tool_invocations` replay rows, or increment state version for the read.

`dry_run=true` is always non-authoritative. It may validate and return diagnostics, candidate blockers, or a would-change summary, but it must not create or update current records, events, artifacts, evidence summaries, Write Authorizations, close state, committed `tool_invocations` replay rows, or state-version increments. A subsequent non-dry-run call must be validated against current state.

<a id="idempotency"></a>

## Idempotency

Every committed state-changing method requires `idempotency_key`. Read-only calls do not create replay rows and do not reserve keys. Keys are scoped to `(project_id, tool_name, idempotency_key)`.

`request_hash` is computed from canonical JSON over the tool name, schema-normalized request body, and every `ToolEnvelope` field except `request_id` and `idempotency_key`.

If a committed replay row exists with the same key and same request hash, Core returns the original committed response without re-running freshness checks, appending events, registering artifacts, consuming authorization, updating blockers, or changing the replay row. If the same key is reused with a different request hash, Core returns `STATE_CONFLICT` and preserves the original replay row.

Dry-run calls and pre-commit failures do not create or reserve replay rows.

<a id="state-conflict-behavior"></a>

## State Conflict Behavior

For a new state-changing attempt with no committed replay row, Core may resolve the primary Task before freshness checking so it can select owner records. Resolution order is tool-specific `task_id`, `ToolEnvelope.task_id`, then active Task. That resolution does not select a separate state clock.

Every fresh non-dry-run state mutation compares `ToolEnvelope.expected_state_version` with the current project-wide `project_state.state_version`. Mismatch returns `STATE_CONFLICT` and creates no current records, events, artifacts, evidence summaries, Write Authorizations, close state, replay rows, or state-version increments. `tasks.state_version` is not an active conflict or concurrency basis.

`STATE_CONFLICT.details` should include:

```yaml
state_clock: project_state.state_version
current_state_version: integer
expected_state_version: integer
project_id: string
task_id: string | null
```

`WriteAuthorization.basis_state_version` is the project-wide compatibility basis for the allow decision. Stale Write Authorization detection compares it with current `project_state.state_version`; no Task-local clock participates.

<a id="harnessclose_task-close-blockers"></a>

## `harness.close_task` Close Blockers

`CloseTaskResponse.blockers` must use structured `CloseBlocker` objects from [API Schema Core](schema-core.md#current-position-display-schemas). Prose-only status text, report text, rendered views, or agent summaries are not close-blocker results.

For `harness.close_task intent=complete`, close blockers are ordered by the deterministic matrix in [Core Model](../core-model.md#close_task). Public error precedence still selects between public `ErrorCode` values when a method needs one primary error, but it must not reorder the complete blocker matrix or hide earlier blockers behind later acceptance or risk checks. Evidence blockers normally use `EVIDENCE_INSUFFICIENT`; artifact availability blockers use `ARTIFACT_MISSING`; unresolved user judgment blockers use `DECISION_REQUIRED` or `DECISION_UNRESOLVED`; sensitive-action permission blockers use the `APPROVAL_*` codes; scope blockers use the scope and baseline codes.

`intent=cancel` and `intent=supersede` are not successful completion. Their blocked responses are limited to the conditions that make that terminal transition invalid, such as task identity or lifecycle, local access, recovery constraints, cancellation conflict, and supersession validity. They must not require evidence sufficiency, final acceptance, or residual-risk acceptance and must not use those missing conditions as blockers for cancellation or supersession.

Known close-relevant risk that has not been shown uses `RESIDUAL_RISK_NOT_VISIBLE`. Visible but unaccepted close-relevant risk is not hidden under that code: if residual-risk acceptance is required, the close blocker uses category `residual_risk_acceptance` and `required_judgment_kind=residual_risk_acceptance`, with `DECISION_REQUIRED` or `DECISION_UNRESOLVED`.

`PROJECTION_STALE` is a readable-view freshness error, not an active close-blocker category by itself.

Run failures, violations, projection failures, artifact integrity failures, validator failures, evidence gaps, and blockers must not be converted into terminal `Task.result=failed`. Keep them in the typed status, error, evidence, artifact, or blocker record that explains what is blocked or must be repaired.

## User-Facing Label Guidance

These labels are display guidance, not new public error codes.

| API condition | User-facing label | Smallest unblocker |
|---|---|---|
| `VALIDATION_FAILED` | invalid request | Fix the payload, enum value, activation rule, or field set before retrying. |
| `STATE_CONFLICT` | state conflict | Refresh current status and retry with the current state version, or replay the original idempotent request. |
| `MCP_UNAVAILABLE` | Core or surface unavailable | Reconnect or diagnose MCP/Core and surface reachability before claiming state changes, gate updates, write compatibility, artifact body access, or close. |
| `LOCAL_ACCESS_MISMATCH` | local access mismatch | Use the registered local transport/session/binding or repair local access registration through the owner path before relying on Harness state. |
| `CAPABILITY_INSUFFICIENT` | unsupported or insufficient surface | Use a capable surface, reduce the operation, or choose a path that does not require the missing capability. |
| `NO_ACTIVE_TASK` | no active Task | Select or create a Task before a Task-scoped action. |
| `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` | scope, boundary, or baseline issue | Confirm or narrow scope, use `harness.update_scope` to update the Change Unit or baseline when the scope change is valid, or request the needed user judgment. |
| `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID` | missing or stale pre-write scope check | Call or retry `harness.prepare_write` for the exact operation, current scope, and current state. |
| `DECISION_REQUIRED`, `DECISION_UNRESOLVED` | judgment needed | Show or resolve the focused `UserJudgment` with kind, refs, options, and consequences. |
| `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED` | sensitive-action approval needed or not usable | Request, resolve, or renew a `judgment_kind=sensitive_approval` user judgment. |
| `EVIDENCE_INSUFFICIENT` | evidence needed | Record or rerun the missing check, or show the evidence gap and smallest unblocker. |
| `ACCEPTANCE_REQUIRED` | final acceptance needed | Request or resolve `judgment_kind=final_acceptance` for the visible result basis. |
| `RESIDUAL_RISK_NOT_VISIBLE` | residual risk not visible | Show the close-relevant risk before final acceptance or close. |
| `PROJECTION_STALE` | stale readable view | Refresh the readable view before relying on it; do not treat it as canonical close state. |
| `ARTIFACT_MISSING` | artifact issue | Reattach, regenerate, or replace the missing or failed artifact before relying on it. |
| `VALIDATOR_FAILED` | check or blocker failed | Show the specific validator or blocker when available; use this fallback only when no typed blocker applies. Do not use it as a design-policy blocker. |
