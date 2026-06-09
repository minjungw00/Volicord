# API Errors

## What this document helps you do

Use this reference for active current MVP public error codes, primary-error precedence, blocked and dry-run behavior, `tool_invocations` replay, state-version conflict behavior, documentation smoke-target error coverage, close blocker behavior, and user-facing label guidance.

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
| `stale_state` | `STATE_VERSION_CONFLICT`, `BASELINE_STALE`, `PROJECTION_STALE` | Refresh current state, baseline, readable view, scope-update result, or pre-write check before relying on it. A Write Authorization whose project-wide basis is stale uses `STATE_VERSION_CONFLICT`. |
| `unsupported_surface` | `CAPABILITY_INSUFFICIENT` or `VALIDATION_FAILED` | Reduce the request, move to a capable surface, or return a blocker. Do not emulate unsupported authority with prose. |
| `out_of_scope` | `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `NO_ACTIVE_CHANGE_UNIT`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` | Hold the affected action, show the mismatch, narrow to current scope, request the specific user-owned scope judgment, or apply the resolved scope change through `harness.update_scope`. |
| `missing_judgment` | `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `ACCEPTANCE_REQUIRED` | Ask or resolve the focused active `UserJudgment`. Do not collapse product, technical, scope, sensitive approval, final acceptance, residual-risk acceptance, cancellation, or later/reserved QA waiver and verification-risk routes into one broad approval. |
| `missing_evidence` | `EVIDENCE_INSUFFICIENT`, `ARTIFACT_MISSING` | Show the affected claim, refs, evidence status, artifact availability, and smallest unblocker. Do not invent test results, artifact integrity, or evidence sufficiency. |
| `close_blocked` | `CloseTaskResponse.close_state=blocked` plus the primary `ErrorCode` | Return structured blockers and next actions. Do not mark the Task terminal. |
| `residual_risk_present` | `RESIDUAL_RISK_NOT_VISIBLE`, `DECISION_REQUIRED`, or `DECISION_UNRESOLVED` | Show the risk and ask `judgment_kind=residual_risk_acceptance` only when the active close or acceptance path requires it. |

<a id="error-taxonomy"></a>

## Error Taxonomy

| Code | Meaning |
|---|---|
| `VALIDATION_FAILED` | Payload shape, enum value, activation rule, profile-specific validation, or `record_run` `ArtifactInput` validation failed before mutation. |
| `STATE_VERSION_CONFLICT` | Pre-commit stale-state rejection: `ToolEnvelope.expected_state_version` does not match current `project_state.state_version`, a Write Authorization is stale because its project-wide `basis_state_version` no longer matches current `project_state.state_version`, or the same idempotency key was reused with a different canonical request. |
| `NO_ACTIVE_TASK` | A Task is required but none is active or addressed. |
| `NO_ACTIVE_CHANGE_UNIT` | A write-capable or close-relevant operation has no active scoped Change Unit. |
| `SCOPE_REQUIRED` | Scope confirmation is required before the requested write or action can proceed. |
| `SCOPE_VIOLATION` | Intended or observed product-file paths or sensitive categories exceed active scope or stored `AuthorizedAttemptScope`. |
| `WRITE_AUTHORIZATION_REQUIRED` | A write-capable Run is missing a required Write Authorization from `harness.prepare_write`. |
| `WRITE_AUTHORIZATION_INVALID` | The supplied Write Authorization exists but is expired, revoked, consumed outside replay, or incompatible for a non-version reason. |
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
| `ARTIFACT_MISSING` | A referenced persistent artifact is missing, unavailable, unusable for the close basis, or failed integrity/metadata checks. |
| `BASELINE_STALE` | Baseline no longer matches the repository state required by the operation. |
| `VALIDATOR_FAILED` | Fallback when a required active validator or blocker check failed and no more specific typed code applies. In the current MVP, this is not a design-policy error. Design-quality concerns must route through an active judgment, blocker, evidence, capability, or residual-risk path, or remain advisory. |

`ToolError.details.authorization_reason` uses exactly:

```text
missing | expired | stale | revoked | consumed | incompatible
```

Use `WRITE_AUTHORIZATION_REQUIRED` with `authorization_reason=missing` when no required authorization is supplied. Use `WRITE_AUTHORIZATION_INVALID` with `authorization_reason=expired`, `revoked`, `consumed`, or `incompatible` when an existing authorization cannot be consumed for a non-version reason.
Use `STATE_VERSION_CONFLICT` with `authorization_reason=stale` when the supplied Write Authorization is stale because its project-wide `basis_state_version` does not match current `project_state.state_version`.

Use `VALIDATION_FAILED` when `ArtifactInput.source_kind` and its source fields do not match the schema shape. During `harness.record_run`, invalid staged-handle validation is a pre-commit failure that returns `ToolRejectedResponse`. Staged-handle validation failures for `ArtifactInput.source_kind=staged_artifact` also use public `VALIDATION_FAILED`, with structured detail in `ToolError.details.artifact_input_error`. Do not introduce new top-level public error codes for each staged-handle validation failure.

`ToolError.details.artifact_input_error` should include the input id and a specific reason. The active detail reason set includes:

```yaml
artifact_input_error:
  artifact_input_id: string
  reason:
    - staged_handle_expired
    - staged_handle_consumed
    - staged_handle_project_mismatch
    - staged_handle_task_mismatch
    - staged_handle_surface_mismatch
    - staged_handle_checksum_mismatch
    - staged_handle_size_mismatch
    - staged_handle_not_found
```

Staged-handle validation covers stored `project_id`, `task_id`, `created_by_surface_id`, `created_by_surface_instance_id`, expiration, consumed status, `sha256`, `size_bytes`, and `redaction_state`. When `redaction_state` is the mismatched staged metadata, the message or an additional detail field should name it while keeping the public code `VALIDATION_FAILED`. A staged-handle provenance or scope mismatch is a validation error, not a request-level local access failure. Do not use `LOCAL_ACCESS_MISMATCH` for staged-handle provenance mismatch; `LOCAL_ACCESS_MISMATCH` is only for request surface verification failure. Do not use `CAPABILITY_INSUFFICIENT` for staged-handle scope or provenance mismatch; `CAPABILITY_INSUFFICIENT` is only for missing or insufficient verified surface capability. `ARTIFACT_MISSING` remains available for referenced persistent artifacts and close-relevant artifact availability, not for staged-handle validation.

Use the local-access codes narrowly and keep them distinguishable. `MCP_UNAVAILABLE` is for unavailable MCP/Core or surface reachability itself, including `VerifiedSurfaceContext.failure_reason=unavailable`. If Core state cannot be read before this rejection, `ToolRejectedResponse.state_version` may be `null`; otherwise it should carry the observed project-wide `project_state.state_version`. `LOCAL_ACCESS_MISMATCH` is for a reachable local transport/session/binding that does not match the registered project surface, or for revoked local access, including `failure_reason=mismatch` or `revoked`. `CAPABILITY_INSUFFICIENT` is for a recognized active surface that lacks the capability needed by the requested access class or guarantee claim, including `failure_reason=insufficient_capability`. `surface_id` alone never resolves any of these errors. Do not substitute a surface-specific `UNAUTHORIZED` code for these public paths.

<a id="primary-error-code-precedence"></a>

## Primary Error Code Precedence

When an error-bearing response branch has non-empty `errors`, `errors[0]` is the primary public code selected by this order unless a method section defines a stricter order. For `ToolRejectedResponse`, `ToolRejectedResponse.errors[0]` is the primary rejection code. For a committed blocked result or a result with diagnostics, `MethodResult.base.errors[0]` is the primary public code. Secondary blockers may still appear in method-specific fields and `ToolError.details`. Valid `ToolDryRunResponse` branches keep `errors=[]`; previewed would-be failures belong in `DryRunSummary.would_errors`.

| Precedence | Primary `ErrorCode` |
|---:|---|
| 1 | `VALIDATION_FAILED` |
| 2 | `STATE_VERSION_CONFLICT` |
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

Every public response is exactly one branch: `ToolRejectedResponse`, a method-specific `MethodResult` built on `ToolResultBase`, or `ToolDryRunResponse`. Branch identity is part of the contract, not a display choice.

Response branch selection follows this precedence:

1. A pre-commit failure returns `ToolRejectedResponse` regardless of `dry_run`. This includes stale `expected_state_version`, stale `WriteAuthorization.basis_state_version` before consumption, request validation failure, local access failure, capability failure, state lookup failure, and invalid staged-handle validation.
2. A valid read-only selected operation returns its method-specific `MethodResult`, even when `dry_run=true`; the result uses `base.dry_run=true` and `base.effect_kind=read_only`.
3. A valid selected operation that could create a Core commit or storage-owned staging side effect returns `ToolDryRunResponse` when `dry_run=true` and Core can produce a preview.
4. A successful non-dry-run commit or staging operation returns its method-specific `MethodResult`.

`dry_run=true` is not a synonym for `ToolDryRunResponse`. It does not mask a primary rejection code, and it does not change a valid read-only method result into a dry-run branch.

`ToolRejectedResponse` is the branch for a pre-commit failure, including `STATE_VERSION_CONFLICT`, request validation failure, and invalid staged-handle validation. It has `response_kind=rejected`, `effect_kind=no_effect`, and no method-specific result object. It must not include result-only fields such as `decision`, `task_ref`, `run_summary`, `staged_artifact_handle`, `write_authorization_ref`, `user_judgment_ref`, or `close_state`. A rejection creates no current records, `task_events`, replay rows, artifacts, staged-handle consumption, evidence summaries, Write Authorization creation or consumption, close-state mutation, or `state_version` increment.

A committed blocked response is not `ToolRejectedResponse`. It is returned inside the method-specific result schema, such as `PrepareWriteResult` or `CloseTaskResult`, only when the method state-effect contract in [MVP API](mvp-api.md#active-mvp-method-behavior) permits a committed blocked result. A committed blocked result has `base.response_kind=result`, may commit only the blocker or status effects allowed by that method, and may update `blockers`, events, project-wide state version, and a `tool_invocations` replay row. It must not create the authority that the blocker says is missing.

Read-only calls, including `harness.status` and `harness.close_task intent=check`, may compute and return blockers or close blockers. Those blockers are response fields only: Core must not store them, append events, create `tool_invocations` replay rows, or increment state version for the read. When the request is otherwise valid, these calls return the method-specific result branch even with `dry_run=true`.

`ToolDryRunResponse` is only the branch for a valid dry-run preview of a selected state-effecting or storage-owned staging operation. `dry_run=true` is non-authoritative. A valid dry-run call whose request shape, local access verification, capability verification, and reachable state/preconditions can be evaluated enough to produce a preview returns `ToolDryRunResponse` with `DryRunSummary`. It may return diagnostics, candidate blockers, `DryRunSummary.would_errors`, `DryRunSummary.next_actions`, and descriptive `PlannedEffect` preview data, but it must not create or update current records, events, artifacts, evidence summaries, Write Authorization records or consumption, close state, committed `tool_invocations` replay rows, staged handles, staged-handle consumption, or state-version increments. It also must not include method-specific result-only fields or real generated refs such as `task_ref`, `run_summary`, `staged_artifact_handle`, `write_authorization_ref`, or `user_judgment_ref`. `PlannedEffect` is descriptive only and must not contain fake generated refs for records that do not exist.

Examples:

| Request condition | Response branch |
|---|---|
| `harness.status` with `dry_run=true`, valid read | `StatusResult` with `base.dry_run=true` and `base.effect_kind=read_only` |
| `harness.close_task intent=check` with `dry_run=true`, valid read | `CloseTaskResult` with `base.dry_run=true` and `base.effect_kind=read_only` |
| `harness.close_task intent=complete` with `dry_run=true`, otherwise valid and previewable | `ToolDryRunResponse` with `effect_kind=no_effect` |
| Supplied stale `expected_state_version` with `dry_run=true` | `ToolRejectedResponse` with primary `STATE_VERSION_CONFLICT`, `dry_run=true`, and `effect_kind=no_effect` |

If a `dry_run=true` request itself fails validation, local access verification, capability verification, state lookup, or stale-state checking before a read-only result or preview can be produced, the response is `ToolRejectedResponse` with `dry_run=true` and `effect_kind=no_effect`. A subsequent non-dry-run call must be validated against current state.

<a id="idempotency"></a>

## Idempotency

Every committed state-changing method requires `idempotency_key`. Read-only calls do not create replay rows and do not reserve keys. Keys are scoped to `(project_id, tool_name, idempotency_key)`.

`request_hash` is computed from canonical JSON over the tool name, schema-normalized request body, and every `ToolEnvelope` field except `request_id` and `idempotency_key`.

Only committed non-dry-run `MethodResult` responses for replay-row-creating state effects are stored in replay rows. If a committed replay row exists with the same key and same request hash, Core returns the original committed response without re-running freshness checks, appending events, promoting or linking artifacts, consuming Write Authorization, updating blockers, or changing the replay row. If the same key is reused with a different request hash, Core preserves the existing replay row and returns `ToolRejectedResponse` with `STATE_VERSION_CONFLICT`.

`ToolRejectedResponse` and `ToolDryRunResponse` do not create or reserve replay rows.

<a id="state-conflict-behavior"></a>

## State Version Conflict Behavior

For a new state-changing attempt with no committed replay row, Core may resolve the primary Task before freshness checking so it can select owner records. Resolution order is tool-specific `task_id`, `ToolEnvelope.task_id`, then active Task. That resolution does not select a separate state clock.

Every fresh non-dry-run state mutation compares `ToolEnvelope.expected_state_version` with the current project-wide `project_state.state_version`. A stale `expected_state_version` returns `ToolRejectedResponse` with `STATE_VERSION_CONFLICT`; it is not a method-specific result and is never a `PrepareWriteResult.decision` value. If a `dry_run=true` request supplies a stale `expected_state_version`, the same pre-commit rejection applies before any read-only result or dry-run preview. The pre-commit failure creates no current records, `task_events`, replay rows, artifacts, staged-handle consumption, evidence summaries, Write Authorization creation or consumption, close-state mutation, or `state_version` increment. `tasks.state_version` is not an active conflict or concurrency basis.

`STATE_VERSION_CONFLICT` is the only active current MVP public `ErrorCode` for project-wide state-version mismatch. Do not expose another public code, alias, deprecated spelling, alternate storage-layer public error name, or internal exception name for that mismatch.

`STATE_VERSION_CONFLICT.details` should include:

```yaml
state_clock: project_state.state_version
current_state_version: integer
expected_state_version: integer
project_id: string
task_id: string | null
```

`WriteAuthorization.basis_state_version` is the project-wide compatibility basis for the allow decision. Stale Write Authorization detection compares it with current `project_state.state_version`; no Task-local clock participates. If `harness.record_run` finds the supplied Write Authorization stale before consumption, the response is `ToolRejectedResponse` with `STATE_VERSION_CONFLICT`, not `WRITE_AUTHORIZATION_INVALID`, and the authorization is not consumed.

<a id="documentation-smoke-error-coverage"></a>

## Documentation Smoke Error Coverage

The first internal documentation smoke target in [MVP Plan](../../build/mvp-plan.md#first-internal-smoke-target) must use only active public errors and active `CloseBlocker.category` values. It does not define smoke-only codes, a complete conformance suite, or an implementation plan.

- Registered surface verification succeeds without an error only when Core derives a compatible `VerifiedSurfaceContext` for the registered surface. Failure uses `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, or `CAPABILITY_INSUFFICIENT`; a copied `surface_id` is not proof of access or capability.
- Project-wide state-version conflict returns `ToolRejectedResponse` with `STATE_VERSION_CONFLICT` when `ToolEnvelope.expected_state_version` is stale against `project_state.state_version`. The failed attempt must not create records, events, artifacts, evidence, Write Authorization creation or consumption, close state, replay rows, staged-handle consumption, or a state-version increment.
- A shaping readiness gap may surface `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, or a structured blocker, depending on the owner path. Read-only status or readiness reads do not mutate state.
- `prepare_write decision=allowed` creates the owner-scoped single-use Write Authorization. `decision=blocked` and `decision=approval_required` are committed `PrepareWriteResult` values only when the method state-effect table permits that blocked commit, and they must not create a consumable Write Authorization. `STATE_VERSION_CONFLICT` and request validation failure are `ToolRejectedResponse` branches, never `PrepareWriteResult.decision` values.
- `SensitiveActionScope` belongs to `judgment_kind=sensitive_approval`. Sensitive approval errors use `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, or `APPROVAL_EXPIRED`; that approval does not replace Write Authorization, final acceptance, residual-risk acceptance, evidence, or artifact authority.
- `harness.stage_artifact` success creates only a temporary handle and no Core mutation. `harness.record_run` is the active path that can promote a valid staged handle to persistent `ArtifactRef`; invalid source-field shape and staged-handle validation failures return `ToolRejectedResponse` with `VALIDATION_FAILED` and `artifact_input_error` detail unless the actual failure is request-level local access or capability verification. They must not be hidden as evidence sufficiency, local access mismatch, or capability insufficiency.
- `harness.record_run` consumes a compatible Write Authorization exactly once. Missing authorization uses `WRITE_AUTHORIZATION_REQUIRED`. A project-wide stale authorization basis uses `STATE_VERSION_CONFLICT`. Expired, revoked, consumed, or non-version-incompatible authorization uses `WRITE_AUTHORIZATION_INVALID`; observed-outside-authorized-scope attempts use the applicable scope or authorization code.
- `close_task intent=check` is read-only even when it returns blockers. `close_task intent=complete` returns `CloseTaskResponse.close_state=blocked` with structured blockers or `close_state=closed` only when no owner-defined complete blocker remains.
- Close smoke coverage must include `EVIDENCE_INSUFFICIENT` for evidence blockers, `ARTIFACT_MISSING` for artifact unavailable or missing blockers, `ACCEPTANCE_REQUIRED` for final acceptance blockers, and `DECISION_REQUIRED` or `DECISION_UNRESOLVED` with `category=residual_risk_acceptance` for visible but unaccepted residual risk. `RESIDUAL_RISK_NOT_VISIBLE` is reserved for risk that has not been shown.
- `close_task intent=supersede` uses supersession, lifecycle, local-access, state-version conflict, or recovery blockers when invalid. It must not require evidence sufficiency, final acceptance, or residual-risk acceptance, and a valid supersede that updates lifecycle plus `project_state.active_task_id` is one project-wide state mutation.

<a id="harnessclose_task-close-blockers"></a>

## `harness.close_task` Close Blockers

`CloseTaskResponse.blockers` must use structured `CloseBlocker` objects from [API Schema Core](schema-core.md#current-position-display-schemas). Prose-only status text, report text, rendered views, or agent summaries are not close-blocker results.

For `harness.close_task intent=complete`, close blockers are ordered by the deterministic matrix in [Core Model](../core-model.md#close_task). Public error precedence still selects between public `ErrorCode` values when a method needs one primary error, but it must not reorder the complete blocker matrix or hide earlier blockers behind later acceptance or risk checks. Evidence blockers normally use `EVIDENCE_INSUFFICIENT`; artifact availability blockers, including unavailable or missing close-relevant artifacts, use `ARTIFACT_MISSING`; unresolved user judgment blockers use `DECISION_REQUIRED` or `DECISION_UNRESOLVED`; sensitive-action permission blockers use the `APPROVAL_*` codes; scope blockers use the scope and baseline codes.

`intent=cancel` and `intent=supersede` are not successful completion. Their blocked responses are limited to the conditions that make that terminal transition invalid, such as task identity or lifecycle, local access, recovery constraints, cancellation conflict, and supersession validity. They must not require evidence sufficiency, final acceptance, or residual-risk acceptance and must not use those missing conditions as blockers for cancellation or supersession.

Known close-relevant risk that has not been shown uses `RESIDUAL_RISK_NOT_VISIBLE`. Visible but unaccepted close-relevant risk is not hidden under that code: if residual-risk acceptance is required, the close blocker uses category `residual_risk_acceptance` and `required_judgment_kind=residual_risk_acceptance`, with `DECISION_REQUIRED` or `DECISION_UNRESOLVED`.

`PROJECTION_STALE` is a readable-view freshness error, not an active close-blocker category by itself.

Run failures, violations, projection failures, artifact integrity failures, validator failures, evidence gaps, and blockers must not be converted into terminal `Task.result=failed`. Keep them in the typed status, error, evidence, artifact, or blocker record that explains what is blocked or must be repaired.

## User-Facing Label Guidance

These labels are display guidance, not new public error codes.

| API condition | User-facing label | Smallest unblocker |
|---|---|---|
| `VALIDATION_FAILED` | invalid request | Fix the payload, enum value, activation rule, or field set before retrying. |
| `STATE_VERSION_CONFLICT` | state version conflict | Refresh current status and retry with the current state version, or replay the original idempotent request. |
| `MCP_UNAVAILABLE` | Core or surface unavailable | Reconnect or diagnose MCP/Core and surface reachability before claiming state changes, gate updates, write compatibility, artifact body access, or close. |
| `LOCAL_ACCESS_MISMATCH` | local access mismatch | Use the registered local transport/session/binding or repair local access registration through the owner path before relying on Harness state. |
| `CAPABILITY_INSUFFICIENT` | unsupported or insufficient surface | Use a capable surface, reduce the operation, or choose a path that does not require the missing capability. |
| `NO_ACTIVE_TASK` | no active Task | Select or create a Task before a Task-scoped action. |
| `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` | scope, boundary, or baseline issue | Confirm or narrow scope, use `harness.update_scope` to update the Change Unit or baseline when the scope change is valid, or request the needed user judgment. |
| `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID` | missing or unusable pre-write scope check | Call or retry `harness.prepare_write` for the exact operation, current scope, and current state. Project-wide state-version drift is shown as `STATE_VERSION_CONFLICT`. |
| `DECISION_REQUIRED`, `DECISION_UNRESOLVED` | judgment needed | Show or resolve the focused `UserJudgment` with kind, refs, options, and consequences. |
| `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED` | sensitive-action approval needed or not usable | Request, resolve, or renew a `judgment_kind=sensitive_approval` user judgment. |
| `EVIDENCE_INSUFFICIENT` | evidence needed | Record or rerun the missing check, or show the evidence gap and smallest unblocker. |
| `ACCEPTANCE_REQUIRED` | final acceptance needed | Request or resolve `judgment_kind=final_acceptance` for the visible result basis. |
| `RESIDUAL_RISK_NOT_VISIBLE` | residual risk not visible | Show the close-relevant risk before final acceptance or close. |
| `PROJECTION_STALE` | stale readable view | Refresh the readable view before relying on it; do not treat it as canonical close state. |
| `ARTIFACT_MISSING` | artifact issue | Reattach, regenerate, restore availability, or replace the missing, unavailable, unusable, or failed artifact before relying on it. |
| `VALIDATOR_FAILED` | check or blocker failed | Show the specific validator or blocker when available; use this fallback only when no typed blocker applies. Do not use it as a design-policy blocker. |
