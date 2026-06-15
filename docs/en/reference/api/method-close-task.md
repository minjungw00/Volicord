<a id="harnessclose_task"></a>

# `harness.close_task` reference

## What this document owns

This document owns baseline method behavior for `harness.close_task`:

- method-specific request conditions, intent handling, access requirements, state-version behavior, result branches, and `dry_run` behavior
- method-specific evaluation order for the `harness.close_task` request
- method-specific blocker-producing branches for `CloseTaskResult.blockers`
- method-specific `CloseReadinessBlocker.code` production behavior
- close-task examples

## What this document does not own

This document does not own:

- common `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, or `ToolDryRunResponse` schema bodies
- nested state, artifact, judgment, value-set, or error schema definitions
- Core close readiness authority concepts
- the `CloseReadinessBlocker` shape or `CloseReadinessBlocker.category` values
- public error code meaning, error precedence, or response-branch routing
- storage layouts, storage-effect detail, security guarantees, or rendered display wording

## Purpose

`harness.close_task` evaluates close readiness for a selected `Task` and, when the selected close intent permits it, performs the requested terminal path.

The method can:

- return a read-only close readiness observation
- commit `intent=complete`, `intent=cancel`, or `intent=supersede`
- return `CloseTaskResult(close_state=blocked)` with `CloseTaskResult.blockers`
- reject the request before close readiness evaluation
- return a common `dry_run` preview for valid mutating previews

Close is a Core state transition, not a report. This method does not infer close from chat, status text, final acceptance alone, residual-risk acceptance alone, evidence alone, a `Write Authorization`, or a rendered view.

## Owner boundary

Method-owned block:

- request validation and intent-field combinations for `harness.close_task`
- the order in which this method reaches check, mutation, blocked, rejected, and dry-run branches
- whether a valid mutating branch may commit a terminal result or committed blocked result
- which method-specific blocker codes may be produced in `CloseTaskResult.blockers`

Core-owned block:

- close readiness authority, close honesty, final acceptance, residual-risk visibility, residual-risk acceptance, and non-substitution rules belong to [Core Model close readiness](../core-model.md#close_task).

API boundary block:

- blocker/API response routing belongs to [API blocker routing](blocker-routing.md).
- error precedence and `STATE_VERSION_CONFLICT` selection belong to [API error precedence](error-precedence.md).
- rejected, blocked, and `dry_run` response-branch routing belongs to [API error routing](error-routing.md).

Schema and display block:

- `CloseReadinessBlocker` and state-shaped data belong to [API State Schemas](schema-state.md#close-readiness-and-validation-shapes).
- exact `intent`, `close_reason`, `close_state`, and blocker-category value names belong to [API Value Sets](schema-value-sets.md#task-lifecycle-values) and [state and blocker values](schema-value-sets.md#state-and-blocker-values).
- persistence effects belong to [Storage Effects](../storage-effects.md).
- rendered wording belongs to [Template Bodies](../template-bodies.md).

## Conditions

Preflight conditions:

- The envelope and method fields must be valid.
- `params.task_id` must identify the same-project `Task` selected by the request.
- The requested `intent`, `close_reason`, and `superseding_task_id` combination must be valid.
- The surface context, access class, local capability, and terminal-path preconditions must allow the requested path.

Mutation conditions:

- `dry_run=false` mutating intents require a non-null `idempotency_key` and current `expected_state_version`.
- Stale `expected_state_version`, stale close-relevant `WriteAuthorization.basis_state_version`, or idempotency request-hash conflict is rejected before close readiness evaluation.
- A close-relevant `Write Authorization` freshness check does not record final acceptance, residual-risk acceptance, user-owned judgment, sensitive-action approval, or broad approval.

Close condition:

- `intent=complete` can close only after preflight succeeds, the close readiness evaluation is valid, and no close blocker remains.
- `intent=cancel` and `intent=supersede` evaluate the requested terminal path. They are not evidence sufficiency, final acceptance, or residual-risk acceptance.

## Close intents

Supported values for `intent`, `close_reason`, and `close_state` are owned by [API Value Sets](schema-value-sets.md#task-lifecycle-values).

| `intent` | `close_reason` | `superseding_task_id` | Method rule |
|---|---|---|---|
| `check` | `null` | `null` | Read-only close readiness observation. |
| `complete` | `completed_self_checked` or `completed_with_risk_accepted` | `null` | Completion path; runs close readiness evaluation. |
| `cancel` | `cancelled` | `null` | Cancellation path; evaluates cancellation-specific terminal constraints. |
| `supersede` | `superseded` | Non-null same-project replacement `Task` reference | Supersession path; evaluates supersession-specific terminal constraints. |

## Required inputs

All calls require:

- `ToolEnvelope` with method-required envelope fields, including `project_id`, `surface_id`, `request_id`, and `dry_run`
- matching `task_id` in the envelope-selected request context and method params
- `intent`
- `close_reason`
- `superseding_task_id`
- `user_note`

Additional requirements:

| Case | Required input rule |
|---|---|
| `intent=check` | `idempotency_key` and `expected_state_version` may be `null`; `close_reason` and `superseding_task_id` must be `null`. |
| `intent=complete`, `intent=cancel`, or `intent=supersede` with `dry_run=false` | `idempotency_key` and `expected_state_version` must be non-null and current. |
| `intent=supersede` | `superseding_task_id` must identify a compatible same-project replacement `Task`. |

## Access requirements

| Request kind | Method access rule |
|---|---|
| `intent=check` | Requires `VerifiedSurfaceContext.access_class=read_status` for protected close readiness detail. |
| Mutating intents | Require `core_mutation`, verified surface context, compatible `Task` state, and close-relevant owner records. |

Access to call this method is separate from user-owned judgment, final acceptance, residual-risk acceptance, sensitive-action approval, and `Write Authorization`.

## Method flow

Implementations evaluate `harness.close_task` in this order:

1. Validate the envelope, method fields, intent-field combination, and same-project `Task` identity. Shape failures, wrong-project identity, and unreadable `Task` identity return `ToolRejectedResponse`.
2. Verify the surface context, access class, local capability, and requested terminal-path preconditions.
3. For `dry_run=false` mutating intents, check `idempotency_key`, current `expected_state_version`, idempotency request hash, and close-relevant `WriteAuthorization.basis_state_version`. Stale or conflicting values return `ToolRejectedResponse`.
4. For `intent=check`, compute current close readiness and return read-only `CloseTaskResult`.
5. For mutating intents with `dry_run=true`, return the common preview branch after valid preflight.
6. For `intent=complete`, run the close readiness evaluation. If blockers remain, return the blocked branch; otherwise commit `close_state=closed`.
7. For `intent=cancel` or `intent=supersede`, evaluate only the requested terminal path. If terminal-path blockers remain, return the blocked branch; otherwise commit `close_state=cancelled` or `close_state=superseded`.

## State-version behavior

| Case | State-version effect |
|---|---|
| `intent=check` | Always read-only and never increments state, including when `dry_run=true`. |
| Successful terminal mutation | Increments `project_state.state_version` exactly once. |
| Committed blocked result for a mutating intent | Increments `project_state.state_version` exactly once when this method and the storage-effect owner allow the committed blocked result. |
| Preflight rejection or valid `dry_run` preview | Increments nothing. |

Preflight rejection includes stale `expected_state_version`, stale close-relevant `WriteAuthorization.basis_state_version`, and idempotency request-hash conflict. These conflicts route to the error owners; they are not close blockers.

## Success result

Success here means a result branch that is not blocked or rejected.

Returns `CloseTaskResult` with `base.response_kind=result`.

| Case | Effect | `close_state` |
|---|---|---|
| `intent=check` and no current blocker | `base.effect_kind=read_only` | `ready` |
| Successful `intent=complete` | `base.effect_kind=core_committed` | `closed` |
| Successful `intent=cancel` | `base.effect_kind=core_committed` | `cancelled` |
| Successful `intent=supersede` | `base.effect_kind=core_committed` | `superseded` |

## Blocked result

Conditions:

- preflight succeeds
- the method reaches read-only close readiness observation or terminal-path evaluation
- the requested path has one or more close or terminal blockers

Result:

- The method may return `CloseTaskResult(close_state=blocked)` with `blockers: CloseReadinessBlocker[]`.
- `intent=check` returns blockers as read-only observation data and never creates blocker rows.
- `dry_run=false` mutating intents may commit a blocked result only when this method and [Storage Effects](../storage-effects.md) allow that effect.

Method-specific blocker branches:

| Branch | Production rule |
|---|---|
| `intent=check` | Returns current close readiness blockers as read-only observation data. |
| `intent=complete` | Produces close readiness blockers when the completion path reaches close readiness evaluation and owner-defined close requirements remain unresolved. |
| `intent=cancel` | Produces blockers only for cancellation-specific terminal constraints. Completion-only evidence, final acceptance, or residual-risk gaps do not block cancellation by themselves. |
| `intent=supersede` | Produces blockers only for supersession-specific terminal constraints. Completion-only evidence, final acceptance, or residual-risk gaps do not block supersession by themselves. |

Non-claims:

- `CloseReadinessBlocker` presence alone does not prove persistence.
- `STATE_VERSION_CONFLICT` is never a `CloseReadinessBlocker.code`.
- A blocker category does not create the underlying user judgment, approval, evidence, artifact availability, final acceptance, residual-risk acceptance, or recovery state.

## Rejected result

The method returns `ToolRejectedResponse` when the request fails before a valid close readiness result or terminal-path evaluation.

Common rejected cases include:

- validation failure
- local access failure
- stale `expected_state_version`
- stale close-relevant `WriteAuthorization.basis_state_version`
- idempotency request-hash conflict
- wrong-project or unreadable `Task` identity
- unavailable Core
- insufficient capability

Rejected responses:

- return no `CloseTaskResult.blockers`
- create no close effect
- create no `Write Authorization`, final acceptance, residual-risk acceptance, evidence, or artifact state

Public error meaning, precedence, and response-branch routing are owned by the API error documents linked below.

## Dry-run behavior

`intent=check` with `dry_run=true` remains the read-only `CloseTaskResult` branch with `base.effect_kind=read_only`.

Mutating intents with `dry_run=true` use `ToolDryRunResponse` after valid preflight. Preview blockers are `PlannedBlocker` data, not stored `CloseReadinessBlocker` objects.

Pre-preview failures with `dry_run=true` return `ToolRejectedResponse`, not `DryRunSummary.would_errors[]` or `PlannedBlocker`.

Branch shapes are owned by [API Schema Core](schema-core.md). Response-branch routing is owned by [API error routing](error-routing.md). Close readiness blocker/API response routing is owned by [API blocker routing](blocker-routing.md).

## Storage effect

`intent=check` has no storage effect, including when it returns blockers or uses `dry_run=true`.

Committed `dry_run=false` mutating intents may persist terminal or blocked outcomes according to the method result. Exact storage effects, replay rows, events, state-version increments, and blocker persistence rules are owned by [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).

Rejected responses and valid `dry_run` previews have no storage effect.

## Examples

The examples are intentionally compact. They illustrate the method branch and keep nested schema, storage, and display details with their owners.

### Minimal valid request

```yaml
method: harness.close_task
params:
  envelope:
    project_id: proj_close_001
    task_id: task_close_001
    actor_kind: agent
    surface_id: surface_close
    request_id: req_close_check_local_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: en-US
  task_id: task_close_001
  intent: check
  close_reason: null
  superseding_task_id: null
  user_note: null
```

### Representative blocked check response

Read-only `CloseTaskResult` for a `Task` whose final acceptance is still missing:

```yaml
base:
  response_kind: result
  effect_kind: read_only
  dry_run: false
  state_version: 72
  events: []
close_state: blocked
state:
  project_id: proj_close_001
  state_version: 72
  task_ref:
    record_kind: task
    record_id: task_close_001
    project_id: proj_close_001
    task_id: task_close_001
    state_version: 72
blockers:
  - category: final_acceptance
    code: missing_final_acceptance
    message: "Final acceptance is still required before this Task can close."
    related_refs: []
    next_actions:
      - action_kind: request_user_judgment
        owner_method: harness.request_user_judgment
        label: "Request final acceptance from the user."
        blocking_question: "Has the user given final acceptance for the completed Task?"
        required_refs:
          - record_kind: task
            record_id: task_close_001
            project_id: proj_close_001
            task_id: task_close_001
            state_version: 72
evidence_summary: null
artifact_refs: []
```

## Owner links

- Request envelope, common response branches, and `dry_run` summaries: [API Schema Core](schema-core.md).
- `CloseTaskResult.blockers`, `CloseReadinessBlocker`, `EvidenceSummary`, `StateSummary`, and `NextActionSummary` shapes: [API State Schemas](schema-state.md#close-readiness-and-validation-shapes).
- Close state, lifecycle, close reason, and `CloseReadinessBlocker.category` values: [API Value Sets](schema-value-sets.md#state-and-blocker-values).
- Close readiness meaning and close honesty: [Core Model close readiness](../core-model.md#close_task).
- Public `ErrorCode` meanings: [API error codes](error-codes.md).
- Error precedence and stale-state conflict selection: [API error precedence](error-precedence.md).
- Rejected, blocked, and `dry_run` response-branch routing: [API error routing](error-routing.md).
- Close readiness blocker/API response routing: [API blocker routing](blocker-routing.md).
- Persistence effects and state-version behavior: [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).
- Display labels and rendered wording: [Template Bodies](../template-bodies.md).
