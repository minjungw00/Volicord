# API Errors

## What this document helps you do

Use this reference for public API error codes, primary error precedence, idempotency replay, and stale-state behavior.

This document describes future Harness Server behavior for planning and review. It does not mean the current documentation repository implements an MCP server.

<a id="mvp-1-guarantee-and-status-taxonomy"></a>

## MVP-1 guarantee and status taxonomy

This section is the single owner for MVP-1 public status/error condition names, user-facing message patterns, and agent behavior. The condition names below are display and routing names, not new `ErrorCode` enum values unless the `Public API path` column names a code. Security meaning for the guarantee level values is owned by [Security Reference: Honest guarantee display](../security.md#honest-guarantee-display).

`guarantee_display.level` uses the exact values `cooperative`, `detective`, `preventive`, and `isolated`.

| Level | MVP-1 display meaning | Agent rule |
|---|---|---|
| `cooperative` | Harness can check and record through the documented path when the agent or tool follows it. It is not OS permission, sandboxing, tamper-proof storage, or pre-execution blocking. | Use the Harness check, hold incompatible writes by instruction, and show the limit honestly. |
| `detective` | Harness or the connected surface can detect, record, or report a mismatch when it becomes observable. It is not prevention. | Report what was detected and what remains unproven; do not claim the action was blocked before it happened. |
| `preventive` | A promoted profile has a proven control that blocks the covered operation before it happens. | Use this label only when the exact covered operation and proof are named. |
| `isolated` | A promoted profile has a documented and proven separation boundary for the claim being made. | Name the boundary; do not infer sensitive-action approval, evidence, work acceptance, residual-risk acceptance, close, or stronger authority from isolation alone. |

MVP-1 defaults to cooperative behavior with limited detective reporting where the active surface can observe the mismatch. Stronger labels require owner-promoted profile documentation and proof for the exact operation or boundary.

| Condition | Public API path | Short meaning | User-facing message pattern | Agent behavior | Blocks next / write / close | Agent must not invent |
|---|---|---|---|---|---|---|
| `core_unavailable` | `MCP_UNAVAILABLE`; diagnostic `MCP_SERVER_UNAVAILABLE` or `SURFACE_MCP_UNAVAILABLE` when known | Harness/Core authority cannot be reached. | "Harness/Core authority is unavailable, so I cannot confirm current Harness state. I can reconnect or diagnose; I can proceed outside Harness only if you explicitly choose that mode." | Fail closed for authority. Hold Harness-dependent writes and close. Reconnect, diagnose, or move to a capable surface. Proceed outside Harness only after the user explicitly chooses that mode. | Yes / yes / yes for Harness-authority-dependent action. | Task state, sensitive-action approval, user judgment, evidence, work acceptance, residual-risk acceptance, gate updates, projection freshness, or close readiness. |
| `local_access_denied` | `LOCAL_ACCESS_MISMATCH` for off-profile local access; `CAPABILITY_INSUFFICIENT` when the current surface lacks required local access | Local file or system access is unavailable, denied, or outside the registered local profile. | "Local access is unavailable or denied, so I cannot inspect or change the requested local path from this surface." | Do not guess local state. Ask for a capable surface, repair the local profile, narrow to accessible paths, or continue only with clearly labeled unverified input. | If access is needed / yes / yes when close depends on that access or evidence. | File contents, command results, artifact bytes, evidence sufficiency, or successful local changes. |
| `stale_state` | `STATE_CONFLICT`, `BASELINE_STALE`, `PROJECTION_STALE`, or stale `WRITE_AUTHORIZATION_INVALID` | The current state, baseline, authorization, or readable view may be out of date. | "Current Harness state or the status view may be out of date. I need to refresh before relying on it for this action." | Refresh current status/state, baseline, projection, or pre-write scope check. Treat stale context as pull-only until refreshed or reconciled. | State-dependent next actions / yes / yes when close depends on stale facts. | Current state, freshness, valid Write Authorization, evidence sufficiency, acceptance, residual-risk status, or close readiness. |
| `unsupported_surface` | `CAPABILITY_INSUFFICIENT`; `VALIDATION_FAILED` when the request activates a stage/profile branch that is not active | The requested behavior is outside the current stage, profile, or connected surface capability. | "This behavior is outside the current Harness stage or surface, so I cannot treat it as available here." | Offer a supported fallback, reduce the request, or move to a capable/profile-promoted surface. Do not emulate later-profile authority with prose. | If that behavior is required / if the write needs it / if close needs it. | Active stage support, surface capability, stronger guarantee level, projection/job existence, evidence, QA, acceptance, risk acceptance, or close support. |
| `out_of_scope` | `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `NO_ACTIVE_CHANGE_UNIT`, `AUTONOMY_BOUNDARY_EXCEEDED`, or `BASELINE_STALE` | The proposed action or write is outside current scope or lacks a compatible scoped work boundary. | "This is outside the current scope. I can narrow the action or ask you to update the scope." | Hold the affected action. Show the mismatch, narrow to the current scope, or request the specific user-owned scope judgment. | Affected next action / yes / yes when unresolved scope affects close. | Scope expansion, non-goal removal, Write Authorization, sensitive-action permission, or user judgment. |
| `missing_judgment` | `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, or `ACCEPTANCE_REQUIRED` | A user-owned judgment is needed or an existing judgment cannot be used. | "User judgment is needed before this can continue." | Ask the focused judgment with options, consequences, uncertainty, and affected refs. Keep product/UX judgment, technical judgment, sensitive-action approval, work acceptance, and residual-risk acceptance separate. | Dependent next action / when the write depends on it / yes when close depends on it. | The user's decision, sensitive-action approval, work acceptance, residual-risk acceptance, waiver, or broad consent from vague text. |
| `missing_evidence` | `EVIDENCE_INSUFFICIENT`, `VERIFY_NOT_DETACHED`, `QA_REQUIRED`, or `ARTIFACT_MISSING` | Required Core-owned evidence summary, verification independence, Manual QA, or artifact support is absent, partial, stale, blocked, or insufficient. | "Evidence is insufficient for that claim." | Show `evidence_summary.status`, the affected claim, refs, and smallest unblocker; run or record the missing check when the agent can. | Evidence-dependent next action / only if evidence is a write precondition / yes when close depends on evidence. | Evidence, test results, QA, artifact integrity, verification independence, sufficiency, or close readiness. |
| `close_blocked` | `CloseTaskResponse.close_state=blocked` with the primary `ErrorCode` selected by precedence | Work cannot be closed under the current contract. | "Close is blocked under the current contract." | Show blockers, related refs, and the smallest unblocker. Do not collapse evidence, QA, work acceptance, residual-risk visibility, and residual-risk acceptance into one claim. | Next action becomes an unblocker / only if blocker requires a write / yes. | Closed terminal state, close readiness, work acceptance, residual-risk acceptance, verification, QA, or final report authority. |
| `residual_risk_present` | Status condition; `RESIDUAL_RISK_NOT_VISIBLE`, `DECISION_REQUIRED`, or `DECISION_UNRESOLVED` when it blocks acceptance or close | Known remaining risk exists and must be displayed; some contexts require explicit residual-risk acceptance. | "Residual risk remains. I will show it explicitly; this may need separate acceptance before close." | Display the risk, impact, refs, and whether acceptance is required. Ask residual-risk acceptance only when the close or acceptance path requires it. | Risk-sensitive next action when relevant / only if risk changes scope or safety / yes when not visible or required acceptance is missing. | No-risk status, hidden risk, accepted risk, work acceptance, or close readiness. |

Core unavailable rule: if Harness/Core authority is unavailable, the agent must not invent task state, sensitive-action approval, user judgment, evidence, work acceptance, residual-risk acceptance, or close readiness. It may only report that authority is unavailable and proceed outside Harness if the user explicitly chooses that mode.

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
| `WRITE_AUTHORIZATION_INVALID` | The supplied Write Authorization is missing, expired, stale, revoked, consumed (outside idempotent replay), or incompatible. |
| `DECISION_REQUIRED` | Blocking user-owned judgment requires a user judgment request before the requested action can proceed. |
| `DECISION_UNRESOLVED` | A relevant user judgment is pending, deferred without coverage, rejected, blocked, stale, or incompatible. |
| `AUTONOMY_BOUNDARY_EXCEEDED` | The intended operation exceeds the active Change Unit Autonomy Boundary. |
| `APPROVAL_REQUIRED` | Sensitive action requires sensitive-action permission before proceeding. |
| `APPROVAL_DENIED` | The relevant sensitive-action permission / Approval was denied. |
| `APPROVAL_EXPIRED` | Sensitive-action permission / Approval expired or drifted from baseline/scope. |
| `CAPABILITY_INSUFFICIENT` | The connected surface is valid but cannot satisfy a required validator, feature, enforcement condition, or MVP-1 behavior. |
| `MCP_UNAVAILABLE` | Required MCP/Core access is unavailable, stale, or unreachable. |
| `LOCAL_ACCESS_MISMATCH` | Core or an operator can classify the caller's local access mode as outside the registered local profile, or required local access is denied by that profile. |
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

When either error carries an invalid-authorization reason in `ToolError.details.authorization_reason`, the reason vocabulary is exactly:

```text
missing | expired | stale | revoked | consumed | incompatible
```

Use `missing` when no authorization id/ref is supplied or the supplied ref cannot be resolved. Use `expired`, `stale`, `revoked`, `consumed`, or `incompatible` for a row that exists but cannot be consumed. No other reason values are valid.

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
| `MCP_UNAVAILABLE` | Core unavailable | Reconnect or diagnose Core access before claiming state changes, gate updates, projection repair, pre-write scope-check compatibility, or close. |
| `LOCAL_ACCESS_MISMATCH` | local access denied or off-profile | Reconnect through the registered local surface/profile, repair the local binding/profile, or use a surface with the needed local access. |
| `CAPABILITY_INSUFFICIENT` | unsupported or insufficient surface | Use a capable surface/profile, reduce the operation, or choose a path that does not need the missing capability. |
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
| 11 | `WRITE_AUTHORIZATION_INVALID` | The supplied Write Authorization is missing, stale, expired, revoked, consumed (outside replay), or incompatible. |
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

<a id="harnessclose_task-close-blockers"></a>

## `harness.close_task` Close Blockers

`harness.close_task` may return multiple close blockers. The primary `ToolError` in `CloseTaskResponse.base.errors` uses the precedence above, and `CloseTaskResponse.blockers` must include observed close blockers as structured results in the same relative order. Prose-only status text, reports, Journey views, or agent summaries are not close-blocker results.

Evidence blockers use category `evidence` and normally primary `EVIDENCE_INSUFFICIENT` when required `evidence_summary.status` is `none`, `partial`, `stale`, or `blocked`. Artifact availability blockers use `artifact_availability` with `ARTIFACT_MISSING` when the evidence summary depends on missing, stale, blocked, or integrity-failed artifact refs.

Unresolved user judgment blockers use `user_judgment` or the more specific category `sensitive_action_approval`, `work_acceptance`, or `residual_risk_acceptance` when the missing judgment type is known. Work acceptance never resolves residual-risk acceptance, and residual-risk acceptance never resolves work acceptance.

Visible-but-unaccepted close-relevant risk is not returned as `RESIDUAL_RISK_NOT_VISIBLE`. If the requested close path requires risk acceptance, public close/API responses use primary `DECISION_REQUIRED` when a residual-risk acceptance user judgment must be requested, or `DECISION_UNRESOLVED` when a relevant residual-risk acceptance user judgment exists but is pending, rejected, blocked, stale, deferred without coverage, or incompatible. The structured close blocker category must be `residual_risk_acceptance`, with refs to the relevant `blocker` and `user_judgment` records in MVP-1; rich `residual_risk` refs are later/profile-promoted.

Projection freshness blockers use `projection_freshness` only for readable context that is too stale or failed for the requested display/action. They do not change Core state, evidence summary status, work acceptance, residual-risk status, or close result by themselves.

## Idempotency

Every committed state-changing tool call requires an `idempotency_key`. Idempotency keys are scoped to `(project_id, tool_name, idempotency_key)`. Repeating the same canonical request hash with the same key returns the original committed response. Reusing the same key with a different canonical request hash returns `STATE_CONFLICT`.

`request_hash` is computed from canonical JSON encoded as UTF-8. The canonical input includes `tool_name`, the schema-normalized request body, and every `ToolEnvelope` field except `request_id` and `idempotency_key`. Core stores this hash in `tool_invocations` or an equivalent committed replay record with the original response.

For state-changing tools, Core checks an existing committed replay row before treating the call as a new mutation attempt. A matching hash returns the original committed response without re-running current freshness checks, appending events, registering artifacts, enqueueing projections, or updating the replay row. A different hash returns `STATE_CONFLICT` and preserves the original replay row.

`dry_run=true` never creates or updates the committed replay row. Repeating a dry-run request therefore revalidates against current state instead of returning an earlier dry-run response as authority. If a later non-dry-run call uses the same `idempotency_key`, only an existing committed replay row participates in replay; a previous dry-run response is not a committed response and cannot reserve the key.

When a key is reused with a different canonical request payload, `ToolError.details` may include the idempotency scope, stored/received request hashes or equivalent opaque comparison, and the fact that the caller must replay the original request or retry with a fresh key. Details must not expose sensitive request bodies.

## State conflict behavior

For state-changing tools with no committed replay row for the supplied idempotency scope, Core resolves the primary Task before the freshness check. Resolution order is tool-specific `task_id`, then `ToolEnvelope.task_id`, then active Task resolution. Task-scoped mutations compare `expected_state_version` with `tasks.state_version`; project-scoped mutations with no resolved primary Task compare it with `project_state.state_version`. A mismatch returns `STATE_CONFLICT`. No current records, events, artifacts, projection jobs, or replay rows are created for that conflicting new attempt.

`WriteAuthorization.basis_state_version` is the affected-scope version used as the compatibility basis for the allow decision. It is not necessarily the resulting `ToolResponseBase.state_version`.

`STATE_CONFLICT.details` should include:

```yaml
scope: task | project
current_state_version: integer
expected_state_version: integer
project_id: string
task_id: string | null
```

A stale `expected_state_version` is concurrency drift, not proof of caller identity. The caller must refresh before retrying; Core must not accept an older Task or project view merely because the caller supplied it.
