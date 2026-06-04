# API Errors

## What this document helps you do

Use this reference for public API error codes, primary error precedence, idempotency replay, and stale-state behavior.

This document describes future Harness Server behavior for planning and review. It does not mean the current documentation repository implements an MCP server.

## Error taxonomy

| Code | Meaning |
|---|---|
| `VALIDATION_FAILED` | Request payload, enum value, activation rule, or profile-specific schema validation failed before mutation. |
| `STATE_CONFLICT` | `expected_state_version` is stale, lock ownership changed, or the same idempotency key was reused with a different payload. |
| `NO_ACTIVE_TASK` | A Task is required but none is active or addressed. |
| `NO_ACTIVE_CHANGE_UNIT` | A write-capable operation has no active scoped Change Unit. |
| `SCOPE_REQUIRED` | Scope confirmation is required before the requested write can proceed. |
| `SCOPE_VIOLATION` | Intended paths, tools, commands, network, secrets, or categories exceed scope. |
| `WRITE_AUTHORIZATION_REQUIRED` | A write-capable run is missing a required Write Authorization from `harness.prepare_write`. |
| `WRITE_AUTHORIZATION_INVALID` | The supplied Write Authorization is absent, expired, stale, revoked, already consumed outside idempotent replay, or incompatible. |
| `DECISION_REQUIRED` | Blocking user-owned judgment requires a user judgment request before the requested action can proceed. |
| `DECISION_UNRESOLVED` | A relevant user judgment is pending, deferred without coverage, rejected, blocked, stale, or incompatible. |
| `AUTONOMY_BOUNDARY_EXCEEDED` | The intended operation exceeds the active Change Unit Autonomy Boundary. |
| `APPROVAL_REQUIRED` | Sensitive action requires sensitive-action permission before proceeding. |
| `APPROVAL_DENIED` | The relevant sensitive-action permission / Approval was denied. |
| `APPROVAL_EXPIRED` | Sensitive-action permission / Approval expired or drifted from baseline/scope. |
| `CAPABILITY_INSUFFICIENT` | The connected surface is valid but cannot satisfy a required validator, feature, or enforcement condition. |
| `MCP_UNAVAILABLE` | Required MCP access is unavailable, stale, or unreachable. |
| `LOCAL_ACCESS_MISMATCH` | Core or an operator can classify the caller's local access mode as outside the registered local profile. |
| `EVIDENCE_INSUFFICIENT` | Required evidence coverage is absent, partial, stale, or blocked. |
| `VERIFY_NOT_DETACHED` | Verification cannot count as detached verification. |
| `QA_REQUIRED` | Required Manual QA is pending, failed, or missing. |
| `ACCEPTANCE_REQUIRED` | Required work acceptance is pending or rejected. |
| `PROJECTION_STALE` | Projection freshness is stale or failed for the requested action. |
| `RECONCILE_REQUIRED` | Human-editable or managed-block drift requires reconcile. |
| `RESIDUAL_RISK_NOT_VISIBLE` | Known close-relevant residual risk has not been made visible before work acceptance or close. |
| `ARTIFACT_MISSING` | A referenced artifact file is missing or integrity check failed. |
| `BASELINE_STALE` | Baseline no longer matches the repository state required by the operation. |
| `VALIDATOR_FAILED` | Fallback when required validators or close/blocker checks failed and no more specific typed code applies. |

`WRITE_AUTHORIZATION_REQUIRED` and `WRITE_AUTHORIZATION_INVALID` are only for missing or invalid Write Authorization records. Scope problems still use `SCOPE_VIOLATION` when observed paths, tools, commands, network targets, secrets, or sensitive categories exceed the Write Authorization record or active scope.

MCP availability, local access/profile mismatch, and capability insufficiency are distinct:

- `MCP_UNAVAILABLE`: Core cannot be reached or required MCP access is stale/unusable.
- `LOCAL_ACCESS_MISMATCH`: a reachable local endpoint or caller path is off-profile, stale, weak, forwarded/tunneled, cross-user, unauthorized, or otherwise mismatched.
- `CAPABILITY_INSUFFICIENT`: the caller is on a recognized surface/profile, but the profile cannot satisfy a required capability, validator, or enforcement condition.

## User-facing display labels

These labels are display guidance, not new public error codes.

| API condition | User-facing label | Smallest unblocker language |
|---|---|---|
| `VALIDATION_FAILED` | invalid request | Fix the payload, enum value, activation rule, or profile-specific field set before retrying. |
| `STATE_CONFLICT` | state conflict | Refresh current status, then retry with the current state version or replay the original idempotent request. |
| `MCP_UNAVAILABLE` | MCP unavailable | Reconnect or diagnose Core access before claiming state changes, gate updates, projection repair, pre-write scope-check compatibility, or close. |
| `LOCAL_ACCESS_MISMATCH` | local access profile mismatch | Reconnect through the registered local surface/profile or repair the local binding/profile. |
| `CAPABILITY_INSUFFICIENT` | capability insufficient | Use a capable surface/profile, reduce the operation, or choose a path that does not need the missing capability. |
| `NO_ACTIVE_TASK` | no active Task | Select or create a Task before using a Task-scoped action. |
| `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID` | missing or stale pre-write scope check | Call or retry `harness.prepare_write` for the exact intended operation, current scope, and current state. |
| `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` | scope, boundary, or baseline issue | Confirm or narrow scope, update the Change Unit or baseline, or request the needed user judgment. |
| `DECISION_REQUIRED`, `DECISION_UNRESOLVED` | judgment needed | Show the relevant user judgment prompt or pending outcome with refs and consequences. |
| `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED` | sensitive-action permission needed or not usable | Request, resolve, or renew a sensitive-action approval user judgment in minimum MVP-1. Committed Approval records are later-profile. |
| `EVIDENCE_INSUFFICIENT`, `VERIFY_NOT_DETACHED`, `QA_REQUIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE` | evidence, verification, QA, work acceptance, or risk visibility needed | Record or rerun the missing check, show residual risk, request work acceptance, or use a valid owner waiver path. |
| `PROJECTION_STALE` | stale status view | Refresh or reconcile the projection before relying on that readable view. |
| `RECONCILE_REQUIRED` | reconcile needed | Reconcile human-editable or managed-block drift before using the affected projection or close path. |
| `ARTIFACT_MISSING` | artifact issue | Reattach, regenerate, or replace the missing or failed artifact before relying on it. |
| `VALIDATOR_FAILED` | check or blocker failed | Show the specific validator or blocker when available; use this fallback only when no typed blocker applies. |

## Primary Error Code Precedence

Public tool responses carry one primary `ToolError.code` even when Core observes multiple blockers. When `ToolResponseBase.errors` is non-empty, `errors[0]` is the primary error selected by this precedence unless a tool subsection defines a narrower order. Secondary blockers may still appear in tool-specific fields, validator results, `ToolError.details`, and state summaries.

| Precedence | Primary `ErrorCode` | Selection note |
|---:|---|---|
| 1 | `VALIDATION_FAILED` | Request payload, enum, activation, or profile-specific field validation failed before mutation. |
| 2 | `STATE_CONFLICT` | Stale `expected_state_version`, state lock conflict, or idempotency key reused with a different payload. |
| 3 | `MCP_UNAVAILABLE` | Required MCP access is unavailable, stale, or unreachable after Core/operator classification. |
| 4 | `LOCAL_ACCESS_MISMATCH` | Reachable local caller/access mode is off-profile or unauthorized for the registered local profile. |
| 5 | `NO_ACTIVE_TASK` | The operation requires a Task and none is active or addressed. |
| 6 | `NO_ACTIVE_CHANGE_UNIT` | The operation is write-capable or close-relevant and no active scoped Change Unit applies. |
| 7 | `BASELINE_STALE` | The requested operation depends on a stale baseline. |
| 8 | `SCOPE_REQUIRED` | Scope must be confirmed before the requested operation can proceed. |
| 9 | `SCOPE_VIOLATION` | Intended or observed paths, tools, commands, network, secrets, or categories exceed scope. |
| 10 | `WRITE_AUTHORIZATION_REQUIRED` | A write-capable Run is missing a required Write Authorization. |
| 11 | `WRITE_AUTHORIZATION_INVALID` | The supplied Write Authorization is stale, expired, revoked, consumed outside replay, or incompatible. |
| 12 | `APPROVAL_DENIED` | Relevant sensitive-action permission was denied. |
| 13 | `APPROVAL_EXPIRED` | Relevant sensitive-action permission expired or drifted. |
| 14 | `APPROVAL_REQUIRED` | A sensitive change needs sensitive-action permission and no compatible grant exists. |
| 15 | `DECISION_UNRESOLVED` | Existing relevant user judgment is pending, rejected, stale, or incompatible. |
| 16 | `AUTONOMY_BOUNDARY_EXCEEDED` | Intended operation exceeds the active Autonomy Boundary. |
| 17 | `DECISION_REQUIRED` | Blocking user-owned judgment needs a user judgment request. |
| 18 | `CAPABILITY_INSUFFICIENT` | The connected surface cannot satisfy a required capability or enforcement condition. |
| 19 | `EVIDENCE_INSUFFICIENT` | Required evidence coverage is absent, partial, stale, or blocked. |
| 20 | `VERIFY_NOT_DETACHED` | Verification cannot count as detached verification. |
| 21 | `QA_REQUIRED` | Required Manual QA is pending, failed, missing, or not validly waived. |
| 22 | `RESIDUAL_RISK_NOT_VISIBLE` | Known close-relevant residual risk has not been made visible. |
| 23 | `ACCEPTANCE_REQUIRED` | Required work acceptance is pending or rejected after residual-risk visibility is satisfied. |
| 24 | `PROJECTION_STALE` | Projection freshness is stale or failed for the requested action. |
| 25 | `RECONCILE_REQUIRED` | Human-editable or managed-block drift requires reconcile. |
| 26 | `ARTIFACT_MISSING` | A referenced artifact file is missing or failed integrity checks. |
| 27 | `VALIDATOR_FAILED` | Generic validator fallback selected only when no more specific typed blocker applies. |

## `harness.close_task` Close Blockers

`harness.close_task` may return multiple close blockers. The primary `ToolError` in `CloseTaskResponse.base.errors` uses the precedence above, and `CloseTaskResponse.blockers` must include observed close blockers as structured results in the same relative order. Prose-only status text, reports, Journey views, or agent summaries are not close-blocker results.

Visible-but-unaccepted close-relevant risk is not returned as `RESIDUAL_RISK_NOT_VISIBLE`. If the requested close path requires risk acceptance, public close/API responses use primary `DECISION_REQUIRED` when a residual-risk acceptance user judgment must be requested, or `DECISION_UNRESOLVED` when a relevant residual-risk acceptance user judgment exists but is pending, rejected, blocked, stale, deferred without coverage, or incompatible. The structured close blocker category must be `residual_risk_acceptance`, with refs to the relevant `blocker` and `user_judgment` records in MVP-1; rich `residual_risk` refs are later/profile-promoted.

## Idempotency

Idempotency keys are scoped to `(project_id, tool_name, idempotency_key)`. Repeating the same payload with the same key returns the original committed response. Reusing a key with a different payload returns `STATE_CONFLICT`.

`request_hash` is computed from canonical JSON encoded as UTF-8. The canonical input includes `tool_name`, the schema-normalized request body, and every `ToolEnvelope` field except `request_id` and `idempotency_key`.

For state-changing tools, Core checks an existing committed replay row before treating the call as a new mutation attempt. A matching hash returns the original committed response without re-running current freshness checks, appending events, registering artifacts, enqueueing projections, or updating the replay row. A different hash returns `STATE_CONFLICT` and preserves the original replay row.

When a key is reused with a different canonical request payload, `ToolError.details` may include the idempotency scope, stored/received request hashes or equivalent opaque comparison, and the fact that the caller must replay the original request or retry with a fresh key. Details must not expose sensitive request bodies.

## State conflict behavior

For state-changing tools with no committed replay row for the supplied idempotency scope, Core compares `expected_state_version` with current project/task state before mutation. A mismatch returns `STATE_CONFLICT`. No current records, events, artifacts, projection jobs, or replay rows are created for that conflicting new attempt.

Core first resolves the primary addressed Task from `ToolEnvelope.task_id`, any tool-specific `task_id`, or active Task resolution. Task-scoped tools compare against `tasks.state_version`; project-scoped tools with no resolved primary Task compare against `project_state.state_version`.

`STATE_CONFLICT.details` should include:

```yaml
scope: task | project
current_state_version: integer
expected_state_version: integer
project_id: string
task_id: string | null
```

A stale `expected_state_version` is concurrency drift, not proof of caller identity. The caller must refresh before retrying; Core must not accept an older Task or project view merely because the caller supplied it.
