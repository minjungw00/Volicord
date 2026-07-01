<a id="volicordclose_task"></a>

# `volicord.close_task` reference

## What this document owns

This document owns baseline method behavior for `volicord.close_task`:

- method-specific request conditions, intent handling, access requirements, state-version behavior, result branches, and `dry_run` behavior
- method-specific evaluation order for the `volicord.close_task` request
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

`volicord.close_task` evaluates close readiness for a selected `Task` and, when the selected close intent permits it, performs the requested terminal path.

The method can:

- return a read-only close readiness observation
- commit `intent=complete`, `intent=cancel`, or `intent=supersede`
- return `CloseTaskResult(close_state=blocked)` with `CloseTaskResult.blockers`
- reject the request before close readiness evaluation
- return a common `dry_run` preview for valid mutating previews

Close is a Core state transition, not a report. This method evaluates the current close basis for `intent=complete`; it does not infer close from chat, status text, a terminal close summary, final acceptance alone, residual-risk acceptance alone, evidence alone, a `Write Check`, or a rendered view.

## Owner boundary

Method-owned block:

- request validation and intent-field combinations for `volicord.close_task`
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
- exact `intent` value names belong to [API Value Sets method-local values](schema-value-sets.md#method-local-values).
- exact `close_reason` and `close_state` value names belong to [API Value Sets task lifecycle values](schema-value-sets.md#task-lifecycle-values).
- exact blocker-category value names belong to [API Value Sets state and blocker values](schema-value-sets.md#state-and-blocker-values).
- persistence effects belong to [Storage Effects](../storage-effects.md).
- rendered wording belongs to [Template Bodies](../template-bodies.md).

## Conditions

Preflight conditions:

- The envelope and method fields must be valid.
- `params.task_id` must identify the same-project `Task` selected by the request.
- The requested `intent`, `close_reason`, and `superseding_task_id` combination must be valid.
- The verified invocation context, operation category, compatible actor source, and terminal-path preconditions must allow the requested path.

Mutation conditions:

- `dry_run=false` mutating intents require a non-null `idempotency_key` and current `expected_state_version`.
- Stale `expected_state_version`, stale close-relevant `WriteCheck.basis_state_version`, or idempotency request-hash conflict is rejected before close readiness evaluation.
- A close-relevant `WriteCheck.basis_state_version` is stale when it does not equal the current `project_state.state_version` at preflight.
- A close-relevant `Write Check` freshness check does not record final acceptance, residual-risk acceptance, user-owned judgment, sensitive-action approval, or broad approval.

Close condition:

- `intent=complete` can close only after preflight succeeds, the close readiness evaluation over the current `CurrentCloseBasis` is valid, current close-basis refs satisfy their artifact and Run compatibility rules, and no close blocker remains.
- When the verified connection is in `guarded` or `managed` mode, close readiness also checks guard health, prompt-capture availability facts, unresolved unrecorded Product Repository changes, and guard-detected write-readiness issues. These checks are close blockers only for guarded or managed behavior; `mcp_only` remains cooperative unless an owner-defined configuration selects guarded or managed behavior.
- Required close evidence must be supported by current claim-matching evidence observation provenance. Unverified, provenance-free, stale, or cooperative-agent-only evidence does not satisfy a close requirement when stronger provenance is required.
- `intent=cancel` requires a current accepted cancellation judgment with `machine_action=accept`, `resolution_outcome=accepted`, `resolved_by_actor_source=local_user`, compatible User Channel provenance, and a basis bound to the Task, current scope revision, and current Change Unit. It does not require completion-only evidence, final acceptance, or residual-risk acceptance.
- `intent=supersede` evaluates the requested terminal path. It is not evidence sufficiency, final acceptance, or residual-risk acceptance.

The terminal close summary produced by a successful terminal close is not the current pre-close basis and is not used as a substitute for `CurrentCloseBasis`.

## Close intents

Supported `intent` values are owned by [API Value Sets method-local values](schema-value-sets.md#method-local-values). Supported `close_reason` and `close_state` values are owned by [API Value Sets task lifecycle values](schema-value-sets.md#task-lifecycle-values).

| `intent` | `close_reason` | `superseding_task_id` | Method rule |
|---|---|---|---|
| `check` | `null` | `null` | Read-only close readiness observation. |
| `complete` | `completed_self_checked` or `completed_with_risk_accepted` | `null` | Completion path; runs close readiness evaluation. |
| `cancel` | `cancelled` | `null` | Cancellation path; requires compatible accepted cancellation authority and evaluates cancellation-specific terminal constraints. |
| `supersede` | `superseded` | Non-null same-project replacement `Task` reference | Supersession path; evaluates supersession-specific terminal constraints. |

## Required inputs

All calls require:

- `ToolEnvelope` with method-required envelope fields, including `project_id`, `request_id`, and `dry_run`
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

## Request schema

This method owns the top-level `params` request shape below. `envelope` is the shared [`ToolEnvelope`](schema-core.md#tool-envelope); this block does not redefine `ToolEnvelope` fields.

All fields shown in this method-owned request block are required members of `params` unless a field note explicitly marks a member optional; `T | null` means the member must be present and may contain JSON `null`.

```yaml
CloseTaskRequest:
  envelope: ToolEnvelope
  task_id: string
  intent: string
  close_reason: string | null
  superseding_task_id: string | null
  user_note: string | null
```

Nested owner links:
- `intent` values are owned by [API Value Sets method-local values](schema-value-sets.md#method-local-values).
- `close_reason` values are owned by [API Value Sets task lifecycle values](schema-value-sets.md#task-lifecycle-values).

## Access requirements

| Request kind | Method access rule |
|---|---|
| `intent=check` | Requires verified invocation context with `operation_category=read` for protected close readiness detail. |
| Mutating intents | Require verified invocation context with `operation_category=agent_workflow`, compatible `Task` state, and close-relevant owner records. |

Access to call this method is separate from user-owned judgment, final acceptance, residual-risk acceptance, sensitive-action approval, and `Write Check`.

## Method flow

Implementations evaluate `volicord.close_task` in this order:

1. Validate the envelope, method fields, intent-field combination, and same-project `Task` identity. Shape failures, wrong-project identity, and unreadable `Task` identity return `ToolRejectedResponse`.
2. Verify the invocation context, operation category, actor source, and requested terminal-path preconditions.
3. For `dry_run=false` mutating intents, check `idempotency_key`, current `expected_state_version`, idempotency request hash, and close-relevant `WriteCheck.basis_state_version`. Stale or conflicting values return `ToolRejectedResponse`.
4. For `intent=check`, compute current close readiness, including selected guard-health facts, with the same calculation used by [`volicord.status`](method-status.md) when `include.close=true`, and return read-only `CloseTaskResult`.
5. For mutating intents with `dry_run=true`, return the common preview branch after valid preflight.
6. For `intent=complete`, run the close readiness evaluation over the current `CurrentCloseBasis`. If blockers remain, return the blocked branch; otherwise commit `close_state=closed`, the terminal close result, and any method-selected project continuity records for close-basis known limits that do not require residual-risk acceptance.
7. For `intent=cancel`, require a current accepted `judgment_kind=cancellation` with `machine_action=accept`, `resolution_outcome=accepted`, `resolved_by_actor_source=local_user`, compatible User Channel provenance, and compatibility with the current Task, scope revision, and Change Unit. Missing or incompatible cancellation authority returns the blocked branch.
8. For `intent=cancel` or `intent=supersede`, evaluate only the requested terminal path. If terminal-path blockers remain, return the blocked branch; otherwise commit `close_state=cancelled` or `close_state=superseded`.

## State-version behavior

| Case | State-version effect |
|---|---|
| `intent=check` | Always read-only and never increments state, including when `dry_run=true`. |
| Successful terminal mutation | Increments `project_state.state_version` exactly once. |
| Committed blocked result for a mutating intent | Increments `project_state.state_version` exactly once when this method and the storage-effect owner allow the committed blocked result. |
| Preflight rejection or valid `dry_run` preview | Increments nothing. |

Preflight rejection includes stale `expected_state_version`, stale close-relevant `WriteCheck.basis_state_version`, and idempotency request-hash conflict. These conflicts route to the error owners; they are not close blockers.

## Success result

Success here means a result branch that is not blocked or rejected.

Returns `CloseTaskResult` with `base.response_kind=result`.

| Case | Effect | `close_state` |
|---|---|---|
| `intent=check` and no current blocker | `base.effect_kind=read_only` | `ready` |
| Successful `intent=complete` | `base.effect_kind=core_committed` | `closed` |
| Successful `intent=cancel` | `base.effect_kind=core_committed` | `cancelled` |
| Successful `intent=supersede` | `base.effect_kind=core_committed` | `superseded` |

## Method result fields

`CloseTaskResult` is the method-specific result branch for a valid close check or terminal close attempt. It carries `base: ToolResultBase` and these method-owned top-level fields:

| Field | Result-field meaning |
|---|---|
| `base` | Common result metadata. The `ToolResultBase` shape, including `events`, is owned by [API Schema Core](schema-core.md#common-response). Valid `CloseTaskResult` branches use `base.response_kind=result`; this method selects `base.effect_kind=read_only` for `intent=check` and `base.effect_kind=core_committed` for committed terminal or owner-allowed committed blocked outcomes. |
| `close_state` | Method result close state for the requested path. Supported values are owned by [API Value Sets](schema-value-sets.md#task-lifecycle-values). `close_state=blocked` is a method result after valid close or terminal-path evaluation, not `ToolRejectedResponse`. |
| `state` | `StateSummary` for the selected Task after the check, terminal mutation, or owner-allowed blocked outcome. Nested state fields, including `close_blockers`, are owned by [API State Schemas](schema-state.md). |
| `current_close_basis` | `CurrentCloseBasis | null` used for close readiness when selected into the result. `null` means no current close basis is available for this result. Shape is owned by [API State Schemas](schema-state.md#close-readiness-and-validation-shapes). |
| `risk_acceptance_coverage` | `RiskAcceptanceCoverage[]` for current residual-risk acceptance coverage in the close-readiness result. Shape is owned by [API State Schemas](schema-state.md#close-readiness-and-validation-shapes). |
| `continuity_summary` | `ProjectContinuitySummary[]` for project continuity records made relevant by this close result. For successful `intent=complete`, this includes continuity records Core carries forward for close-basis known limits that do not require residual-risk acceptance. Empty means the computation ran and found no carry-forward records for this result. Shape is owned by [API State Schemas](schema-state.md#project-continuity-shapes). |
| `blockers` | `CloseReadinessBlocker[]` returned when the requested path has close or terminal blockers. Shape and nesting are owned by [API State Schemas](schema-state.md#close-readiness-and-validation-shapes); `category` values are owned by [API Value Sets](schema-value-sets.md#state-and-blocker-values). |
| `guard_health` | `GuardHealthSummary | null` for guard-health facts selected into the close-readiness result. Shape is owned by [API State Schemas](schema-state.md#guard-health-summary). |
| `evidence_summary` | `EvidenceSummary | null` for the close basis visible in the result, or `null` when no evidence summary is selected into the result. Shape is owned by [API State Schemas](schema-state.md#evidence-and-run-snapshot-shapes). |
| `artifact_refs` | `ArtifactRef[]` for close-relevant artifacts selected into the result. `ArtifactRef` shape is owned by [API Artifact Schemas](schema-artifacts.md#artifactref). |

`CloseTaskResult` does not have a top-level `next_actions` field. Next actions for close blockers appear inside `CloseReadinessBlocker.next_actions` and use the canonical `NextActionSummary` shape from [API State Schemas](schema-state.md#current-position-display-shapes).

This method owns the method-scoped `CloseReadinessBlocker.code` values it produces. Those codes are not public `ErrorCode` values and are not global value-set entries.

Method-local `CloseReadinessBlocker.code` list:

The production meanings below apply only after the method reaches close-readiness observation or terminal-path evaluation. Preflight failures still return `ToolRejectedResponse` according to the error owners.

| Code | Category | Local production meaning |
|---|---|---|
| `task_not_closeable` | `task` | The selected Task lifecycle or terminal-path state cannot take the requested close intent. |
| `missing_active_change_unit` | `scope` | A close path requires a current Change Unit, but none is available. |
| `pending_user_judgment` | `pending_user_judgment` | A required user-owned judgment remains pending or unresolved. |
| `missing_sensitive_approval` | `sensitive_approval` | A required separate sensitive-action approval is absent. |
| `missing_cancellation_authority` | `user_judgment` | `intent=cancel` lacks a current accepted user cancellation judgment with `resolved_by_actor_source=local_user`, compatible User Channel provenance, and a basis bound to the current Task, scope revision, and Change Unit. |
| `write_check_stale` | `write_compatibility` | A close-relevant `Write Check` is unusable for a freshness reason that is not routed as `STATE_VERSION_CONFLICT`. |
| `baseline_stale` | `baseline` | The close-relevant baseline basis is stale on a blocker-producing path. |
| `guard_not_installed` | `connection_capability` | A guarded or managed close path has no usable guard installation recorded for the verified connection. |
| `guard_reload_required` | `connection_capability` | Guard files are installed, but the host must restart or reload before Volicord has observed the configured hooks. |
| `guard_not_observed` | `connection_capability` | A guarded or managed close path has configured guard files, but no matching host guard hook observation is recorded. |
| `guard_stale` | `connection_capability` | A guarded or managed close path has a guard installation whose recorded status is `stale`. |
| `guard_broken` | `connection_capability` | A guarded or managed close path has a guard installation whose recorded status is `broken`. |
| `guard_degraded` | `connection_capability` | A guarded or managed close path has a guard installation whose recorded status is `degraded` and the current guard policy blocks close on degraded health. |
| `guard_connection_unhealthy` | `connection_capability` | A guarded or managed close path has an Agent Connection health fact that is not healthy. |
| `unresolved_unrecorded_changes` | `connection_capability` | Guard records show unresolved unrecorded Product Repository changes that must be recorded or reconciled before close. |
| `guard_write_readiness_missing_or_stale` | `write_compatibility` | Guard events detected missing or stale write readiness for the close path. |
| `evidence_claim_unsupported` | `evidence_claim` | A required close claim lacks supported evidence coverage. |
| `evidence_claim_missing` | `evidence_claim` | A required close claim has no current evidence coverage record. |
| `evidence_provenance_insufficient` | `evidence_provenance` | Required close evidence exists but lacks sufficient current source and assurance provenance. |
| `evidence_provenance_stale` | `evidence_provenance` | Evidence observation provenance exists but is stale for the current Task scope, Change Unit, source Run, or close-basis evidence summary. |
| `evidence_agent_report_only` | `evidence_provenance` | Required close evidence is supported only by cooperative agent reports when stronger provenance is required. |
| `artifact_unavailable` | `artifact_availability` | A close-relevant artifact is missing, unavailable, unusable, or integrity-failed. |
| `missing_final_acceptance` | `final_acceptance` | Required final acceptance is absent for the current close basis. |
| `stale_final_acceptance` | `final_acceptance` | A final acceptance exists but is stale or incompatible with the current Task, Change Unit, `scope_revision`, `close_basis_revision`, baseline, or result refs. |
| `residual_risk_not_visible` | `residual_risk_visibility` | Close-relevant residual risk has not been made visible. |
| `missing_residual_risk_acceptance` | `residual_risk_acceptance` | Required residual-risk acceptance is absent for the current residual-risk requirements. |
| `stale_residual_risk_acceptance` | `residual_risk_acceptance` | Residual-risk acceptance exists but does not match the current `close_basis_revision` and exact residual-risk `risk_id` values. |
| `recovery_required` | `recovery` | Recovery work remains required before the requested close path can proceed. |

These codes are method-local `CloseReadinessBlocker.code` values. They are not public `ErrorCode` values, not `WriteDecisionReason.code` values, and not global value-set entries.

For `pending_user_judgment`, blocker next actions may point to available User Channel answer paths, including MCP elicitation, prompt-capture chat commands, or local user commands when those paths are available. The blocker does not authorize an Agent Connection to answer the user-owned judgment.

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
| `intent=complete` | Produces close readiness blockers when the completion path reaches close readiness evaluation and owner-defined close requirements remain unresolved. In `guarded` or `managed` mode this includes guard-health, unresolved unrecorded-change, and guard-detected write-readiness blockers. |
| `intent=cancel` | Produces blockers only for cancellation-specific terminal constraints, including missing or incompatible cancellation authority. Completion-only evidence, final acceptance, or residual-risk gaps do not block cancellation by themselves. |
| `intent=supersede` | Produces blockers only for supersession-specific terminal constraints. Completion-only evidence, final acceptance, or residual-risk gaps do not block supersession by themselves. |

Non-claims:

- `CloseReadinessBlocker` presence alone does not prove persistence.
- `STATE_VERSION_CONFLICT` is never a `CloseReadinessBlocker.code`.
- `STATE_VERSION_CONFLICT` is a rejected-response `ErrorCode`, not a method-local blocker or decision code.
- A blocker category does not create the underlying user judgment, approval, evidence, artifact availability, final acceptance, residual-risk acceptance, or recovery state.
- Unverified claims, provenance-missing evidence, stale observation provenance, and cooperative agent reports may be recorded as evidence history, but they do not satisfy required close evidence when the close path requires stronger provenance.
- Rejected, deferred, stale, superseded, expired, invalid-basis, agent-recorded, provenance-missing, or outcome-absent cancellation judgments do not permit cancellation.

## Rejected result

The method returns `ToolRejectedResponse` when the request fails before a valid close readiness result or terminal-path evaluation.

Common rejected cases include:

- validation failure
- actor-source or operation-category mismatch
- stale `expected_state_version`
- stale close-relevant `WriteCheck.basis_state_version`
- idempotency request-hash conflict
- wrong-project or unreadable `Task` identity
- unavailable Core
- unsupported invocation context

Rejected responses:

- return no `CloseTaskResult.blockers`
- create no close effect
- create no `Write Check`, final acceptance, residual-risk acceptance, evidence, or artifact state

Public error meaning, precedence, and response-branch routing are owned by the API error documents linked below.

## Dry-run behavior

`intent=check` with `dry_run=true` remains the read-only `CloseTaskResult` branch with `base.effect_kind=read_only`.

Mutating intents with `dry_run=true` use `ToolDryRunResponse` after valid preflight. Preview blockers are `PlannedBlocker` data, not stored `CloseReadinessBlocker` objects.

Pre-preview failures with `dry_run=true` return `ToolRejectedResponse`, not `DryRunSummary.would_errors[]` or `PlannedBlocker`.

Branch shapes are owned by [API Schema Core](schema-core.md). Response-branch routing is owned by [API error routing](error-routing.md). Close readiness blocker/API response routing is owned by [API blocker routing](blocker-routing.md).

## Storage effect

`intent=check` has no storage effect, including when it returns blockers or uses `dry_run=true`.

Committed `dry_run=false` mutating intents may persist terminal or blocked outcomes according to the method result. A successful terminal close may persist a terminal close summary, distinct from the current close basis used for pre-close readiness. Successful `intent=complete` may also persist project continuity records with `kind=known_limit` for current close-basis residual risks that are visible but do not require residual-risk acceptance. Exact storage effects, replay rows, events, state-version increments, project continuity persistence, and blocker persistence rules are owned by [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).

Rejected responses and valid `dry_run` previews have no storage effect.

## Examples

The examples are intentionally compact. They illustrate the method branch and keep nested schema, storage, and display details with their owners.

### Minimal valid request

```yaml
method: volicord.close_task
params:
  envelope:
    project_id: proj_close_001
    task_id: task_close_001
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

Read-only `CloseTaskResult` for `task_close_001` at `state_version: 72`, where the method-local response reports one final-acceptance blocker:

```yaml
base:
  response_kind: result
  effect_kind: read_only
  dry_run: false
  state_version: 72
  events: []
close_state: blocked
current_close_basis: null
risk_acceptance_coverage: []
continuity_summary: []
state:
  project_id: proj_close_001
  state_version: 72
  task_ref:
    record_kind: task
    record_id: task_close_001
    project_id: proj_close_001
    task_id: task_close_001
    state_version: 72
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Complete onboarding checklist setup."
  scope_summary: "Onboarding checklist completion."
  non_goals:
    - "Changing account creation."
  acceptance_criteria:
    - "The onboarding checklist is ready for user review."
  autonomy_boundary: "Stay within onboarding checklist completion."
  active_change_unit_ref: null
  baseline_ref: baseline_close_001
  shaping_readiness: null
  pending_user_judgment_refs: []
  blocker_refs: []
  write_check_summary: null
  evidence_summary: null
  close_state: blocked
  close_blockers:
    - category: final_acceptance
      code: missing_final_acceptance
      message: "Final acceptance is still required before this Task can close."
      can_resolve_in_chat: false
      terminal_action_required: false
      related_refs: []
      next_actions:
        - action_kind: request_user_judgment
          owner_method: volicord.request_user_judgment
          label: "Request final acceptance from the user."
          blocking_question: "Has the user given final acceptance for the completed Task?"
          required_refs:
            - record_kind: task
              record_id: task_close_001
              project_id: proj_close_001
              task_id: task_close_001
              state_version: 72
  guarantee_display: null
blockers:
  - category: final_acceptance
    code: missing_final_acceptance
    message: "Final acceptance is still required before this Task can close."
    can_resolve_in_chat: false
    terminal_action_required: false
    related_refs: []
    next_actions:
      - action_kind: request_user_judgment
        owner_method: volicord.request_user_judgment
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
- `CloseTaskResult.blockers`, `CurrentCloseBasis`, `RiskAcceptanceCoverage`, `CloseReadinessBlocker`, `ProjectContinuitySummary`, `EvidenceSummary`, `StateSummary`, and `NextActionSummary` shapes: [API State Schemas](schema-state.md#close-readiness-and-validation-shapes).
- `ArtifactRef` shape: [API Artifact Schemas](schema-artifacts.md#artifactref).
- `intent` values: [API Value Sets method-local values](schema-value-sets.md#method-local-values).
- Close state, lifecycle, and close reason values: [API Value Sets task lifecycle values](schema-value-sets.md#task-lifecycle-values).
- `CloseReadinessBlocker.category` values: [API Value Sets state and blocker values](schema-value-sets.md#state-and-blocker-values).
- Close readiness meaning and close honesty: [Core Model close readiness](../core-model.md#close_task).
- Public `ErrorCode` meanings: [API error codes](error-codes.md).
- Error precedence and stale-state conflict selection: [API error precedence](error-precedence.md).
- Rejected, blocked, and `dry_run` response-branch routing: [API error routing](error-routing.md).
- Close readiness blocker/API response routing: [API blocker routing](blocker-routing.md).
- Persistence effects and state-version behavior: [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).
- Display labels and rendered wording: [Template Bodies](../template-bodies.md).
