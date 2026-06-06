# API Errors

## What this document helps you do

Use this reference for active current MVP public error codes, primary-error precedence, blocked and dry-run behavior, idempotency replay, state conflict behavior, close blocker behavior, and user-facing label guidance.

This document describes future Harness Server behavior for planning and review. It does not mean the current documentation repository implements an MCP server.

<a id="active-mvp-guarantee-and-status-taxonomy"></a>

## Current MVP Guarantee And Profile-Gated Claim Taxonomy

`guarantee_display.level` uses the current MVP values `cooperative` and `detective` unless a promoted profile explicitly supports a profile-gated display value. Security meaning is owned by [Security Reference: Honest guarantee display](../security.md#honest-guarantee-display), and the exact value-set boundary is owned by [API Schema Core](schema-core.md#current-mvp-value-sets).

Unsupported requests to require or display a profile-gated guarantee are claim-boundary errors. Use `CAPABILITY_INSUFFICIENT` when the surface lacks the needed blocking, isolation, observation, or proof support. Use `VALIDATION_FAILED` when the requested value is not valid for the active profile or request shape. Neither error proves that the stronger guarantee exists.

| Level or name | Error/status meaning |
|---|---|
| `cooperative` | Harness can check and record when the agent or tool follows the documented path. It is not OS permission, sandboxing, tamper-proof storage, or pre-execution blocking. |
| `detective` | Harness or the connected surface can detect, record, or report an observable mismatch after or during the action. It is not prevention. |
| `preventive` | Profile-gated display value name. Without promoted pre-tool blocking support for the covered operation, return a capability or validation error and lower the displayed guarantee. |
| `isolated` | Profile-gated display value name. Without promoted isolation support for the named boundary, return a capability or validation error and lower the displayed guarantee. |

Active MVP behavior defaults to cooperative checks with limited detective reporting where the connected surface can honestly observe facts.

| Condition | Public path | Agent rule |
|---|---|---|
| `core_unavailable` | `MCP_UNAVAILABLE` | Do not invent Harness state. Hold Harness-dependent writes and close until Core is reachable or the user explicitly chooses to proceed outside Harness. |
| `local_access_denied` | `LOCAL_ACCESS_MISMATCH` or `CAPABILITY_INSUFFICIENT` | Do not guess local file or command facts. Use a capable local surface, repair capability registration, narrow scope, or label input unverified. |
| `stale_state` | `STATE_CONFLICT`, `BASELINE_STALE`, `PROJECTION_STALE`, stale `WRITE_AUTHORIZATION_INVALID` | Refresh current state, baseline, readable view, or pre-write check before relying on it. |
| `unsupported_surface` | `CAPABILITY_INSUFFICIENT` or `VALIDATION_FAILED` | Reduce the request, move to a capable surface, or return a blocker. Do not emulate unsupported authority with prose. |
| `out_of_scope` | `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `NO_ACTIVE_CHANGE_UNIT`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` | Hold the affected action, show the mismatch, narrow to current scope, or request the specific user-owned scope judgment. |
| `missing_judgment` | `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `ACCEPTANCE_REQUIRED` | Ask or resolve the focused `UserJudgment`. Do not collapse product, technical, scope, sensitive approval, final acceptance, residual-risk acceptance, QA waiver, verification-risk acceptance, or cancellation into one broad approval. |
| `missing_evidence` | `EVIDENCE_INSUFFICIENT`, `ARTIFACT_MISSING` | Show the affected claim, refs, evidence status, and smallest unblocker. Do not invent test results, artifact integrity, or evidence sufficiency. |
| `close_blocked` | `CloseTaskResponse.close_state=blocked` plus the primary `ErrorCode` | Return structured blockers and next actions. Do not mark the Task terminal. |
| `residual_risk_present` | `RESIDUAL_RISK_NOT_VISIBLE`, `DECISION_REQUIRED`, or `DECISION_UNRESOLVED` | Show the risk and ask `judgment_kind=residual_risk_acceptance` only when the active close or acceptance path requires it. |

<a id="error-taxonomy"></a>

## Error Taxonomy

| Code | Meaning |
|---|---|
| `VALIDATION_FAILED` | Payload shape, enum value, activation rule, or profile-specific validation failed before mutation. |
| `STATE_CONFLICT` | `expected_state_version` is stale, state lock ownership changed, or the same idempotency key was reused with a different canonical request. |
| `NO_ACTIVE_TASK` | A Task is required but none is active or addressed. |
| `NO_ACTIVE_CHANGE_UNIT` | A write-capable or close-relevant operation has no active scoped Change Unit. |
| `SCOPE_REQUIRED` | Scope confirmation is required before the requested write or action can proceed. |
| `SCOPE_VIOLATION` | Intended or observed paths, tools, commands, network targets, secret access, or sensitive categories exceed active scope or stored `AuthorizedAttemptScope`. |
| `WRITE_AUTHORIZATION_REQUIRED` | A write-capable Run is missing a required Write Authorization from `harness.prepare_write`. |
| `WRITE_AUTHORIZATION_INVALID` | The supplied Write Authorization is missing, expired, stale, revoked, consumed outside replay, or incompatible. |
| `DECISION_REQUIRED` | A blocking user-owned judgment must be requested before the action can proceed. |
| `DECISION_UNRESOLVED` | A relevant user judgment is pending, deferred without coverage, rejected, blocked, stale, superseded, or incompatible. |
| `AUTONOMY_BOUNDARY_EXCEEDED` | The intended operation exceeds the active Change Unit Autonomy Boundary. |
| `APPROVAL_REQUIRED` | Sensitive-action approval is required before proceeding. |
| `APPROVAL_DENIED` | The relevant sensitive-action approval was denied. |
| `APPROVAL_EXPIRED` | The relevant sensitive-action approval expired or drifted from scope/baseline. |
| `CAPABILITY_INSUFFICIENT` | The surface is recognized but cannot satisfy a required observation, capture, local access, blocking/isolation condition, guarantee claim, or active behavior. |
| `MCP_UNAVAILABLE` | Required MCP/Core access is unavailable, stale, or unreachable. |
| `LOCAL_ACCESS_MISMATCH` | The reachable local caller/access path is outside the registered local profile or lacks required local access. |
| `EVIDENCE_INSUFFICIENT` | Required evidence coverage is absent, partial, stale, or blocked. |
| `ACCEPTANCE_REQUIRED` | Required final acceptance is pending, rejected, or not compatible with the visible result basis. |
| `PROJECTION_STALE` | A requested readable status/view is stale or failed. It is not Core state and is not a close blocker by itself. |
| `RESIDUAL_RISK_NOT_VISIBLE` | Known close-relevant residual risk has not been made visible before final acceptance or close. |
| `ARTIFACT_MISSING` | A referenced artifact is missing or failed integrity/metadata checks. |
| `BASELINE_STALE` | Baseline no longer matches the repository state required by the operation. |
| `VALIDATOR_FAILED` | Fallback when a required active validator or blocker check failed and no more specific typed code applies. |

`ToolError.details.authorization_reason` uses exactly:

```text
missing | expired | stale | revoked | consumed | incompatible
```

Use `WRITE_AUTHORIZATION_REQUIRED` with `authorization_reason=missing` when no required authorization is supplied. Use `WRITE_AUTHORIZATION_INVALID` for an existing authorization that cannot be consumed.

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

A blocked response is not the same as a pre-commit failure. Core may commit a blocked response only where the method owner allows blocker recording. A committed blocked response may update `blockers`, events, state version, and idempotency replay, but it must not create the authority that the blocker says is missing.

`dry_run=true` is always non-authoritative. It may validate and return diagnostics, candidate blockers, or a would-change summary, but it must not create or update current records, events, artifacts, evidence summaries, consumable Write Authorizations, close state, or committed replay rows. A subsequent non-dry-run call must be validated against current state.

<a id="idempotency"></a>

## Idempotency

Every committed state-changing method requires `idempotency_key`. Keys are scoped to `(project_id, tool_name, idempotency_key)`.

`request_hash` is computed from canonical JSON over the tool name, schema-normalized request body, and every `ToolEnvelope` field except `request_id` and `idempotency_key`.

If a committed replay row exists with the same key and same hash, Core returns the original committed response without re-running freshness checks, appending events, registering artifacts, consuming authorization, updating blockers, or changing the replay row. If the same key is reused with a different hash, Core returns `STATE_CONFLICT` and preserves the original replay row.

Dry-run calls and pre-commit failures do not create or reserve replay rows.

<a id="state-conflict-behavior"></a>

## State Conflict Behavior

For a new state-changing attempt with no committed replay row, Core resolves the primary Task before freshness checking. Resolution order is tool-specific `task_id`, `ToolEnvelope.task_id`, then active Task.

Task-scoped mutations compare `expected_state_version` with `tasks.state_version`. Project-scoped mutations with no resolved primary Task compare it with `project_state.state_version`. Mismatch returns `STATE_CONFLICT` and creates no current records, events, artifacts, evidence summaries, Write Authorizations, close state, or replay rows.

`STATE_CONFLICT.details` should include:

```yaml
scope: task | project
current_state_version: integer
expected_state_version: integer
project_id: string
task_id: string | null
```

`WriteAuthorization.basis_state_version` is the compatibility basis for the allow decision. It is not necessarily the resulting `ToolResponseBase.state_version`.

<a id="harnessclose_task-close-blockers"></a>

## `harness.close_task` Close Blockers

`CloseTaskResponse.blockers` must use structured `CloseBlocker` objects from [API Schema Core](schema-core.md#current-position-display-schemas). Prose-only status text, report text, rendered views, or agent summaries are not close-blocker results.

Close blockers are ordered by the primary-error precedence when they map to public errors. Evidence blockers normally use `EVIDENCE_INSUFFICIENT`; artifact availability blockers use `ARTIFACT_MISSING`; unresolved user judgment blockers use `DECISION_REQUIRED` or `DECISION_UNRESOLVED`; sensitive-action permission blockers use the `APPROVAL_*` codes; scope blockers use the scope and baseline codes.

Known close-relevant risk that has not been shown uses `RESIDUAL_RISK_NOT_VISIBLE`. Visible but unaccepted close-relevant risk is not hidden under that code: if residual-risk acceptance is required, the close blocker uses category `residual_risk_acceptance` and `required_judgment_kind=residual_risk_acceptance`, with `DECISION_REQUIRED` or `DECISION_UNRESOLVED`.

`PROJECTION_STALE` is a readable-view freshness error, not an active close-blocker category by itself.

## User-Facing Label Guidance

These labels are display guidance, not new public error codes.

| API condition | User-facing label | Smallest unblocker |
|---|---|---|
| `VALIDATION_FAILED` | invalid request | Fix the payload, enum value, activation rule, or field set before retrying. |
| `STATE_CONFLICT` | state conflict | Refresh current status and retry with the current state version, or replay the original idempotent request. |
| `MCP_UNAVAILABLE` | Core unavailable | Reconnect or diagnose Core access before claiming state changes, gate updates, write compatibility, or close. |
| `LOCAL_ACCESS_MISMATCH` | local access denied or off capability | Use the registered local surface, repair local access, or move to a capable surface. |
| `CAPABILITY_INSUFFICIENT` | unsupported or insufficient surface | Use a capable surface, reduce the operation, or choose a path that does not require the missing capability. |
| `NO_ACTIVE_TASK` | no active Task | Select or create a Task before a Task-scoped action. |
| `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` | scope, boundary, or baseline issue | Confirm or narrow scope, update the Change Unit or baseline, or request the needed user judgment. |
| `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID` | missing or stale pre-write scope check | Call or retry `harness.prepare_write` for the exact operation, current scope, and current state. |
| `DECISION_REQUIRED`, `DECISION_UNRESOLVED` | judgment needed | Show or resolve the focused `UserJudgment` with kind, refs, options, and consequences. |
| `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED` | sensitive-action approval needed or not usable | Request, resolve, or renew a `judgment_kind=sensitive_approval` user judgment. |
| `EVIDENCE_INSUFFICIENT` | evidence needed | Record or rerun the missing check, or show the evidence gap and smallest unblocker. |
| `ACCEPTANCE_REQUIRED` | final acceptance needed | Request or resolve `judgment_kind=final_acceptance` for the visible result basis. |
| `RESIDUAL_RISK_NOT_VISIBLE` | residual risk not visible | Show the close-relevant risk before final acceptance or close. |
| `PROJECTION_STALE` | stale readable view | Refresh the readable view before relying on it; do not treat it as canonical close state. |
| `ARTIFACT_MISSING` | artifact issue | Reattach, regenerate, or replace the missing or failed artifact before relying on it. |
| `VALIDATOR_FAILED` | check or blocker failed | Show the specific validator or blocker when available; use this fallback only when no typed blocker applies. |
