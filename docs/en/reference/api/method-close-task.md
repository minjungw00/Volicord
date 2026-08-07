<a id="volicordclose_task"></a>
<a id="volicordcheck_close"></a>

# `volicord.check_close` and `volicord.close_task` reference

Close readiness and workflow progression are separate. A ready advisor shaping
checkpoint selects advisor finalization; it is not itself the advice result
basis. Only advisor finalization through `volicord.finalize_advice` can establish
the checkpoint-backed advisor close basis. A work shaping checkpoint cannot supply
an implementation close basis. Blocker-local remediation actions do not
override an earlier workflow-required shaping action.

## What this document owns

This document owns baseline method behavior for the close method family:

- method-specific request conditions, `intent` handling, access requirements, state-version behavior, result branches, and `dry_run` behavior for `volicord.check_close` and `volicord.close_task`
- method-specific evaluation order for the `volicord.check_close` and `volicord.close_task` requests
- method-specific blocker-producing branches for `CheckCloseResult.blockers`
  and `CloseTaskResult.blockers`
- method-specific `CloseReadinessBlocker.code` production behavior
- close-task examples

## What this document does not own

This document does not own:

- common `ToolEnvelope`, `ToolRejectedResponse`, or `ToolDryRunResponse` schema bodies
- nested state, artifact, judgment, value-set, or error schema definitions
- Core close readiness authority concepts
- the `CloseReadinessBlocker` shape or `CloseReadinessBlocker.category` values
- public error code meaning, error precedence, or response-branch routing
- storage layouts, storage-effect detail, security guarantees, or rendered display wording

## Purpose

`volicord.check_close` evaluates close readiness for a selected `Task` as a read-only method. `volicord.close_task` performs supported terminal close mutations after the requested terminal path passes its checks.

The methods can:

- return a read-only close readiness observation through `volicord.check_close`
- commit `intent=complete`, `intent=cancel`, or `intent=supersede`
- return `CloseTaskResult(close_state=blocked)` with `CloseTaskResult.blockers`
- reject the request before close readiness evaluation
- return a common `dry_run` preview for valid mutating previews

Close is a Core state transition, not a report. `volicord.close_task` evaluates the current close basis for `intent=complete`; it does not infer close from chat, status text, a terminal close summary, final acceptance alone, residual-risk acceptance alone, evidence alone, a write ticket, or a rendered view.

Before method-specific request validation, Core builds the normalized current
`WorkflowSnapshot` and requires the exact `volicord.check_close/check_close` or
`volicord.close_task/close_task` transition from its canonical
`WorkflowMachine`. The admitted descriptor fixes the current authority
coordinates. Its absence returns a no-effect `TransitionRejection`; any
`recovery_action_key` is another transition in that same current catalog.
`check_close` remains read-only, while only an admitted `close_task` can reach a
terminal commit.

## Owner boundary

Method-owned block:

- request validation for `volicord.check_close`
- request validation and combinations of the `intent` field for `volicord.close_task`
- the order in which these methods reach read-only check, mutation, blocked, rejected, and `dry_run` branches
- whether a valid mutating branch commits a terminal result or returns a response-only blocked result
- which method-specific blocker codes may be produced in
  `CheckCloseResult.blockers` or `CloseTaskResult.blockers`

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
- For `volicord.close_task`, the requested `intent`, `close_reason`, and `superseding_task_id` combination must be valid.
- The verified invocation context, operation category, compatible actor source, and terminal-path preconditions must allow the requested path.

Read-only check condition:

- `volicord.check_close` has no `intent`, `close_reason`, `superseding_task_id`, or terminal mutation path. It returns the current close-readiness observation and must not commit close state.

Mutation conditions:

- `dry_run=false` mutating `intent` values require a non-null `idempotency_key` and current `expected_state_version`.
- Stale `expected_state_version` or an idempotency request-hash conflict is rejected before close readiness evaluation.
- A close-relevant write ticket is evaluated once against its explicit validity basis: current Task, Change Unit, scope revision, baseline, workspace context, current normalized project write-authority binding, the canonical typed approval assessment, task state, and an optional configured idle timeout. Close readiness consumes that evaluated ticket state and does not independently compare approval references or resolution IDs. Its audit-only `basis_state_version` is not compared with the global state version. Persisted approval-reference owner disagreement or duplicate identity is Store corruption and never a semantic changed result; expiry of an otherwise valid approval is a semantic currentness outcome.
- A write-ticket validity check does not record final acceptance, residual-risk acceptance, user-owned judgment, sensitive-action approval, or broad approval.

Close condition:

- `intent=complete` can close only after preflight succeeds, the close readiness evaluation over the current `CurrentCloseBasis` is valid, current close-basis refs satisfy their mode-compatible artifact, Run, or shaping-checkpoint lineage rules, and no close blocker remains.
- `intent=check` and `intent=complete` close readiness block when a write ticket for the Task remains active and unconsumed. Invalidated, revoked, and effective idle-timeout-invalidated ticket rows remain visible as stale authority state but do not block close by themselves. An unresolved Unrecorded Change, including an out-of-scope Product Repository path, remains a separate blocker. There is no fixed ticket expiry.
- A valid `effective_control_level=observe` Task has no write ticket or product-file write path. Its close-readiness result does not recommend `volicord.prepare_write`. For `advisor`, missing result, evidence, or close-basis work points to `volicord.finalize_advice` and never to `volicord.record_run`; for direct/work, compatible missing result or close-basis work remains Run-owned.
- Final acceptance follows Task mode, effective control, and authoritative project policy. `advisor` always requires a compatible final-acceptance UserAction even though its effective control is `observe`. For direct/work, `sensitive` and `tracked` require compatible final acceptance, `observe` uses `not_required`, and `light` is `policy_dependent` and may use `not_required` only when the current project policy explicitly permits it and all policy conditions remain satisfied, including no sensitive action, no unresolved user requirement, no residual-risk acceptance requirement, no required evidence gap, and no unresolved Unrecorded Change. Policy strengthening raises the effective control before this decision and never lowers it. No policy waives evidence, sensitive approval, risk acceptance, or another blocker.
- Effective `sensitive` control also requires a ticket-backed exact sensitive-action
  basis and a current matching user-owned approval. Final acceptance does not
  replace either one. If the exact basis was never recorded, close reports
  `missing_sensitive_action_basis`; if the basis exists but its approval is no
  longer current, close reports `missing_sensitive_approval`.
- Unresolved Unrecorded Changes block close until reconciliation resolves
  them. Repository-observation unavailability remains a separate diagnostic
  and does not produce `unresolved_unrecorded_changes`.
- Only current acceptance criteria with `evidence_requirement=required` create
  evidence close requirements. Each must have current target-matching evidence
  observation provenance. Optional, `not_required`, supplemental, and retired
  `optional`, `not_required`, supplemental, and retired targets never block close. Unverified, provenance-free, stale,
  contradicted, partial, unsupported, or cooperative-agent-only evidence does
  not satisfy a required criterion when stronger provenance is required.
  Strong evaluation independently requires current byte integrity, an
  authority-owned producer record, exact output binding, a current Task/scope/
  baseline and target, and supported relevance. Reused evidence recursively
  revalidates every original producer and relevance record.
- `intent=cancel` requires a current accepted cancellation judgment with `machine_action=accept`, `resolution_outcome=accepted`, `resolved_by_actor_source=local_user`, compatible User Channel provenance, and a basis bound to the Task, current scope revision, and current Change Unit. It does not require completion-only evidence, final acceptance, or residual-risk acceptance.
- `intent=supersede` evaluates the requested terminal path. It is not evidence sufficiency, final acceptance, or residual-risk acceptance.

The terminal close summary produced by a successful terminal close is not the current pre-close basis and is not used as a substitute for `CurrentCloseBasis`.

## Close intents

`volicord.check_close` has no `intent` field. Supported `volicord.close_task.intent` values are owned by [API Value Sets method-local values](schema-value-sets.md#method-local-values). Supported `close_reason` and `close_state` values are owned by [API Value Sets task lifecycle values](schema-value-sets.md#task-lifecycle-values).

| `intent` | `close_reason` | `superseding_task_id` | Method rule |
|---|---|---|---|
| `complete` | `completed_self_checked` or `completed_with_risk_accepted` | `null` | Completion path; runs close readiness evaluation. |
| `cancel` | `cancelled` | `null` | Cancellation path; requires compatible `accepted` cancellation authority and evaluates cancellation-specific terminal constraints. |
| `supersede` | `superseded` | Non-null same-project replacement `Task` reference | Supersession path; evaluates supersession-specific terminal constraints. |

## Required inputs

All `volicord.check_close` calls require:

- `ToolEnvelope` with method-required envelope fields, including `project_id`, `request_id`, and `dry_run`
- matching `task_id` in the envelope-selected request context and method params

All `volicord.close_task` calls require:

- `ToolEnvelope` with method-required envelope fields, including `project_id`, `request_id`, and `dry_run`
- matching `task_id` in the envelope-selected request context and method params
- `intent`
- `close_reason`
- `superseding_task_id`
- `user_note`

Additional requirements:

| Case | Required input rule |
|---|---|
| `volicord.check_close` | `idempotency_key` and `expected_state_version` may be `null`; no close intent fields are accepted. |
| `intent=complete`, `intent=cancel`, or `intent=supersede` with `dry_run=false` | `idempotency_key` and `expected_state_version` must be non-null and current. |
| `intent=supersede` | `superseding_task_id` must identify a compatible same-project replacement `Task`. |

## Request schema

This document owns the top-level `params` request fields in the generated
tables below. `envelope` is the shared
[`ToolEnvelope`](schema-core.md#tool-envelope); the tables do not redefine
`ToolEnvelope` fields. Requiredness and nullability come directly from the
semantic request descriptors.

<!-- BEGIN GENERATED: contract-structures api.method.check_close.request[params] api.method.close_task.request[params] -->
<!-- This region is generated from maintained sources; do not edit it directly. -->

### `CheckCloseRequest` fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `envelope` | yes | no | `ToolEnvelope` |
| `task_id` | yes | no | `TaskId` |

### `CloseTaskRequest` fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `close_reason` | yes | yes | `CloseReason` |
| `envelope` | yes | no | `ToolEnvelope` |
| `intent` | yes | no | `CloseMutationIntent` |
| `superseding_task_id` | yes | yes | `TaskId` |
| `task_id` | yes | no | `TaskId` |
| `user_note` | yes | yes | `string` |
<!-- END GENERATED: contract-structures api.method.check_close.request[params] api.method.close_task.request[params] -->



Nested owner links:
- `intent` values are owned by [API Value Sets method-local values](schema-value-sets.md#method-local-values).
- `close_reason` values are owned by [API Value Sets task lifecycle values](schema-value-sets.md#task-lifecycle-values).

## Access requirements

| Request kind | Method access rule |
|---|---|
| `volicord.check_close` | Requires verified invocation context with `operation_category=read` for protected close readiness detail. |
| Mutating `intent` values | Require verified invocation context with `operation_category=agent_workflow`, compatible `Task` state, and close-relevant owner records. |

Access to call this method is separate from user-owned judgment, final acceptance, residual-risk acceptance, sensitive-action approval, and write ticket.

## Method flow

Implementations evaluate `volicord.check_close` in this order:

1. Validate the envelope, method fields, and same-project `Task` identity. Shape failures, wrong-project identity, and unreadable `Task` identity return `ToolRejectedResponse`.
2. Verify the invocation context, operation category, and actor source.
3. Compute current close readiness and the canonical `EvidenceGateSummary` with the same calculation used by [`volicord.status`](method-status.md) when evidence or close is selected, and return `CheckCloseResult`. The top-level gate, `state.evidence_gate`, and `summary_card.evidence` reuse this one result.

Implementations evaluate `volicord.close_task` in this order:

1. Validate the envelope, method fields, `intent` field combination, and same-project `Task` identity. Shape failures, wrong-project identity, and unreadable `Task` identity return `ToolRejectedResponse`.
2. Verify the invocation context, operation category, actor source, and requested terminal-path preconditions.
3. For `dry_run=false` mutating `intent` values, check `idempotency_key`, current `expected_state_version`, idempotency request hash, and the explicit state-bound validity of each close-relevant write ticket, including its current project write-authority binding. A conflicting envelope version or request hash returns `ToolRejectedResponse`. An invalidated, revoked, policy-authority-stale, or effective idle-timeout-invalidated ticket remains disclosed but does not create a close blocker by itself. `basis_state_version` remains audit-only.
4. For mutating `intent` values with `dry_run=true`, return the common preview branch after valid preflight.
5. For `intent=complete`, run the close readiness evaluation over the current `CurrentCloseBasis`. If blockers remain, return the blocked branch; otherwise commit `close_state=closed`, the mode-compatible terminal close result, and any method-selected project continuity records for close-basis known limits that do not require residual-risk acceptance. The terminal result is `advice_only` for `Task.mode=advisor` and `completed` for `Task.mode=direct` or `work`.
6. For `intent=cancel`, require a current accepted `judgment_kind=cancellation` with `machine_action=accept`, `resolution_outcome=accepted`, `resolved_by_actor_source=local_user`, compatible User Channel provenance, and compatibility with the current Task, scope revision, and Change Unit. Missing or incompatible cancellation authority returns the blocked branch.
7. For `intent=cancel` or `intent=supersede`, evaluate only the requested terminal path. If terminal-path blockers remain, return the blocked branch; otherwise atomically invalidate the Task's active write tickets with `task_closed` and commit `close_state=cancelled` or `close_state=superseded`.

## State-version behavior

| Case | State-version effect |
|---|---|
| `volicord.check_close` | Never increments `project_state.state_version`, including when `dry_run=true`. |
| Successful terminal mutation | Increments `project_state.state_version` exactly once. |
| Blocked result for a mutating `intent` | Never increments `project_state.state_version`; it returns `base.effect_kind=no_effect` without a terminal mutation, event, or replay row. |
| Preflight rejection or valid `dry_run` preview | Increments nothing. |

Preflight rejection includes stale `expected_state_version` and idempotency request-hash conflict. These conflicts route to the error owners; they are not close blockers. Write-ticket invalidation is state-bound and remains visible in ticket authority state; it does not create a close blocker by itself.

The read-only check, its close-readiness computation, and any unrelated
authority-state change do not consume or invalidate an active write ticket.

## Success result

Success here means a result branch that is not blocked or rejected.

Returns `CheckCloseResult` for `volicord.check_close` or `CloseTaskResult` for
`volicord.close_task`, with `base.response_kind=result`.

| Case | Effect | `close_state` |
|---|---|---|
| `volicord.check_close` and no current blocker | `base.effect_kind=read_only` | `ready` |
| Successful `intent=complete` | `base.effect_kind=core_committed` | `closed` |
| Successful `intent=cancel` | `base.effect_kind=core_committed` | `cancelled` |
| Successful `intent=supersede` | `base.effect_kind=core_committed` | `superseded` |

For successful `intent=complete`, both the returned `state.lifecycle.result` and the stored `Task.result` are `advice_only` when `Task.mode=advisor`; they are `completed` when `Task.mode=direct` or `work`. This result mapping does not change or infer the existing evidence, final-acceptance, residual-risk, or other close-readiness policy.

## Method result fields

`CheckCloseResult` and `CloseTaskResult` share the close-assessment fields
below but remain distinct public result types. `CheckCloseResultBase` permits
only `read_only`; `CloseTaskResultBase` permits only `core_committed` and
`no_effect`.

<!-- BEGIN GENERATED: contract-structures api.method.check_close.response[response_variants] api.method.check_close.response[result_body] api.method.check_close.response[result_metadata] api.method.check_close.response[rejection] api.method.close_task.response[response_variants] api.method.close_task.response[result_body] api.method.close_task.response[result_metadata] api.method.close_task.response[rejection] api.method.close_task.response[dry_run] -->
<!-- This region is generated from maintained sources; do not edit it directly. -->

### `CheckCloseResult` success fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `artifact_refs` | yes | no | `ArtifactRef[]` |
| `authority_receipt` | yes | no | `AuthorityReceipt` |
| `base` | yes | no | `CheckCloseResultBase` |
| `blockers` | yes | no | `CloseReadinessBlocker[]` |
| `close_state` | yes | no | `CloseState` |
| `continuity_summary` | yes | no | `ProjectContinuitySummary[]` |
| `current_close_basis` | no | yes | `CurrentCloseBasis` |
| `evidence_gate` | yes | no | `EvidenceGateSummary` |
| `evidence_summary` | no | yes | `EvidenceSummary` |
| `pending_user_action_summaries` | yes | no | `AgentSafeUserActionRequestSummary[]` |
| `risk_acceptance_coverage` | yes | no | `RiskAcceptanceCoverage[]` |
| `state` | yes | no | `StateSummary` |
| `summary_card` | yes | no | `SummaryCard` |

### `CloseTaskResult` success fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `artifact_refs` | yes | no | `ArtifactRef[]` |
| `authority_receipt` | yes | no | `AuthorityReceipt` |
| `base` | yes | no | `CloseTaskResultBase` |
| `blockers` | yes | no | `CloseReadinessBlocker[]` |
| `close_state` | yes | no | `CloseState` |
| `continuity_summary` | yes | no | `ProjectContinuitySummary[]` |
| `current_close_basis` | no | yes | `CurrentCloseBasis` |
| `evidence_gate` | yes | no | `EvidenceGateSummary` |
| `evidence_summary` | no | yes | `EvidenceSummary` |
| `pending_user_action_summaries` | yes | no | `AgentSafeUserActionRequestSummary[]` |
| `risk_acceptance_coverage` | yes | no | `RiskAcceptanceCoverage[]` |
| `state` | yes | no | `StateSummary` |
| `summary_card` | yes | no | `SummaryCard` |

### `Result Metadata: read_only` fields

Contract: `dry_run` preserves the normalized request intent; `events` must be empty (`maxItems: 0`).

| Field | Required | Nullable | Type |
|---|---|---|---|
| `disclosure` | yes | no | `GuaranteeDisclosure` |
| `dry_run` | yes | no | `boolean` |
| `effect_kind` | yes | no | `string enum("read_only")` |
| `events` | yes | no | `EmptyEventRefs` |
| `response_kind` | yes | no | `string enum("result")` |
| `state_version` | yes | no | `integer` |

### `Result Metadata: core_committed` fields

Contract: `dry_run` is `false`; `events` contains at least one event (`minItems: 1`).

| Field | Required | Nullable | Type |
|---|---|---|---|
| `disclosure` | yes | no | `GuaranteeDisclosure` |
| `dry_run` | yes | no | `boolean enum(false)` |
| `effect_kind` | yes | no | `string enum("core_committed")` |
| `events` | yes | no | `NonEmptyEventRefs` |
| `response_kind` | yes | no | `string enum("result")` |
| `state_version` | yes | no | `integer` |
| `transition` | yes | yes | `TransitionDescriptor` |

### `Result Metadata: no_effect` fields

Contract: `dry_run` is `false`; `events` must be empty (`maxItems: 0`).

| Field | Required | Nullable | Type |
|---|---|---|---|
| `disclosure` | yes | no | `GuaranteeDisclosure` |
| `dry_run` | yes | no | `boolean enum(false)` |
| `effect_kind` | yes | no | `string enum("no_effect")` |
| `events` | yes | no | `EmptyEventRefs` |
| `response_kind` | yes | no | `string enum("result")` |
| `state_version` | yes | no | `integer` |

### `dry_run` request policy

- `volicord.check_close`: `dry_run=true` is accepted through the regular result branch with `base.dry_run=true`; it does not create a preview response. `dry_run=false` or an omitted `dry_run` does not select a preview branch.
- `volicord.close_task`: `dry_run=true` selects the `ToolDryRunResponse` preview branch, whose `base.dry_run` is `true`. `dry_run=false` or an omitted `dry_run` does not select a preview branch.


### Shared response structures

Each response descriptor defines its supported response branches as an exact `anyOf` branch union. Only a response descriptor that exposes preview includes that branch. The rejection branch uses the generated [`ToolRejectedResponse`](schema-core.md#common-response) structure. When method behavior selects a preview branch, it uses the generated [`ToolDryRunResponse`](schema-core.md#common-response) structure. Shared rejection and preview fields remain distinct from the success fields above.
<!-- END GENERATED: contract-structures api.method.check_close.response[response_variants] api.method.check_close.response[result_body] api.method.check_close.response[result_metadata] api.method.check_close.response[rejection] api.method.close_task.response[response_variants] api.method.close_task.response[result_body] api.method.close_task.response[result_metadata] api.method.close_task.response[rejection] api.method.close_task.response[dry_run] -->

Each close blocker's remediation actions stay inside its `CloseReadinessBlocker.next_actions` list and use the canonical `NextActionSummary` shape from [API State Schemas](schema-state.md#current-position-display-shapes). Blockers are independent facts; `summary_card` is display data, while tagged `workflow` remains progression authority.

Pending user actions for another operation and informational-only pending
actions may remain visible through the broader
`state.pending_user_action_summaries` projection. They do not enter the top-level
`pending_user_action_summaries` list unless a current
`pending_user_action` blocker internally selects that request for the requested
close path. The public output of that selection is only the corresponding safe
summary, never a request ref or request detail.

This method owns the method-scoped `CloseReadinessBlocker.code` values it produces. Those codes are not public `ErrorCode` values and are not global value-set entries.

Method-local `CloseReadinessBlocker.code` list:

The production meanings below apply only after the method reaches close-readiness observation or terminal-path evaluation. Preflight failures still return `ToolRejectedResponse` according to the error owners.

| Code | Category | Local production meaning |
|---|---|---|
| `task_not_closeable` | `task` | The selected Task lifecycle or terminal-path state cannot take the requested close intent. |
| `missing_active_change_unit` | `scope` | A close path requires a current Change Unit, but none is available. |
| `pending_user_action` | `pending_user_action` | A required user-owned action remains pending or unresolved. |
| `missing_sensitive_action_basis` | `sensitive_approval` | The effective `sensitive` Task has no ticket-backed exact sensitive-action basis. Close remains blocked until `prepare_write` and a matching `record_run` preserve the user-approved operation, scope, baseline, and Change Unit. |
| `missing_sensitive_approval` | `sensitive_approval` | A required separate sensitive-action approval is absent. |
| `missing_cancellation_authority` | `user_action` | `intent=cancel` lacks a current accepted cancellation `UserActionResolution` with `resolved_by_actor_source=local_user`, compatible User Channel provenance, and a basis bound to the current Task, scope revision, and Change Unit. |
| `rejected_cancellation_authority` | `user_action` | A current local-user cancellation `UserActionResolution` explicitly rejects `intent=cancel`. |
| `stale_cancellation_authority` | `user_action` | A cancellation `UserActionResolution` exists, but its Task, scope revision, Change Unit, or effective user-action basis is no longer current. |
| `open_write_ticket` | `write_compatibility` | A write ticket for the selected Task remains open and unresolved. |
| `baseline_stale` | `baseline` | The close-relevant baseline basis is stale on a blocker-producing path. |
| `unresolved_unrecorded_changes` | `connection_capability` | Unresolved Unrecorded Changes must be reconciled before close. Repository-observation unavailable diagnostics do not produce this blocker. The blocker includes `next_actions` with `owner_method=volicord.reconcile_changes`. |
| `evidence_claim_unsupported` | `evidence_claim` | A required close claim lacks supported evidence coverage. |
| `evidence_claim_missing` | `evidence_claim` | A required close claim has no current evidence coverage record. |
| `evidence_provenance_insufficient` | `evidence_provenance` | Required close evidence exists but lacks sufficient current source and assurance provenance. |
| `evidence_provenance_stale` | `evidence_provenance` | Evidence observation provenance exists but is stale for the current Task scope, Change Unit, source Run, or close-basis evidence summary. |
| `evidence_agent_report_only` | `evidence_provenance` | Required close evidence is supported only by cooperative agent reports when stronger provenance is required. |
| `artifact_unavailable` | `artifact_availability` | A close-relevant artifact is missing, unavailable, unusable, or integrity-failed. |
| `missing_final_acceptance` | `final_acceptance` | Required final acceptance is absent for the current close basis. The surfaced action identifies the Agent Connection `volicord.request_user_action` procedure and the final-acceptance question. |
| `stale_final_acceptance` | `final_acceptance` | A final acceptance exists but is stale or incompatible with the current Task, Change Unit, `scope_revision`, `close_basis_revision`, baseline, or result refs. The surfaced action requests a judgment bound to the current basis. |
| `residual_risk_not_visible` | `residual_risk_visibility` | Close-relevant residual risk has not been made visible. |
| `missing_residual_risk_acceptance` | `residual_risk_acceptance` | Required residual-risk acceptance is absent for the current residual-risk requirements. |
| `stale_residual_risk_acceptance` | `residual_risk_acceptance` | Residual-risk acceptance exists but does not match the current `close_basis_revision` and exact residual-risk `risk_id` values. |
| `recovery_required` | `recovery` | Recovery work remains required before the requested close path can proceed. |

These codes are method-local `CloseReadinessBlocker.code` values. They are not public `ErrorCode` values, not `WriteDecisionReason.code` values, and not global value-set entries.

For `pending_user_action`, blocker next actions may identify the User Channel as the next actor, while `pending_user_action_summaries` carries only the exact agent-safe pending summaries. The public close result never carries the resolution form, capture command, or User Channel credential. The CLI inbox renderer obtains the complete typed CLI inbox item through its separate internal Core boundary. The blocker does not authorize an Agent Connection to resolve the user-owned action.

`missing_final_acceptance` with no pending final-acceptance request is a supported two-step state, not an authority shortcut. A read-only check or a blocked close attempt does not create a request or resolution record. Its `request_user_action` action has `allowed_operation_categories=[agent_workflow]`; the Agent Connection creates the current request using the displayed question. After that commit, the public `pending_user_action` blocker exposes only a generic `resolve_user_action` action with `allowed_operation_categories=[user_only]`. A separately verified User Channel projection supplies any available input path to the user. The Agent Connection must not perform the second action.

## Blocked result

Conditions:

- preflight succeeds
- the method reaches close readiness observation or terminal-path evaluation
- the requested path has one or more close or terminal blockers

Result:

- `volicord.check_close` may return `CheckCloseResult(close_state=blocked)`;
  `volicord.close_task` may return `CloseTaskResult(close_state=blocked)`.
  Both expose `blockers: CloseReadinessBlocker[]`.
- `volicord.check_close` returns blockers as response observation data and never creates blocker rows.
- A `dry_run=false` mutating `intent` with blockers returns a response-only result with `base.effect_kind=no_effect`. It does not persist close blocker rows, append an authority event, create a replay row, mutate terminal state, or increment `project_state.state_version`.

Method-specific blocker branches:

| Branch | Production rule |
|---|---|
| `volicord.check_close` | Returns current close readiness blockers as response observation data. |
| `intent=complete` | Produces close readiness blockers when the completion path reaches close readiness evaluation and owner-defined close requirements remain unresolved. This includes active unconsumed write tickets and unresolved Unrecorded Changes. Invalidated, revoked, and effective idle-timeout-invalidated ticket rows do not block by themselves. |
| `intent=cancel` | Produces blockers only for cancellation-specific terminal constraints, including missing or incompatible cancellation authority. Completion-only evidence, final acceptance, or residual-risk gaps do not block cancellation by themselves. |
| `intent=supersede` | Produces blockers only for supersession-specific terminal constraints. Completion-only evidence, final acceptance, or residual-risk gaps do not block supersession by themselves. |

Close-readiness Unrecorded Change rules:

- Unresolved Unrecorded Changes remain visible and block close.
  Repository-observation unavailable diagnostics remain separate and do not
  produce this blocker.

Non-claims:

- `CloseReadinessBlocker` presence alone does not prove persistence.
- `STATE_VERSION_CONFLICT` is never a `CloseReadinessBlocker.code`.
- `STATE_VERSION_CONFLICT` is a rejected-response `ErrorCode`, not a method-local blocker or decision code.
- A blocker category does not create the underlying user judgment, approval, evidence, artifact availability, final acceptance, residual-risk acceptance, or recovery state.
- Close readiness is not correctness proof, test sufficiency proof, or human review replacement. Both close-family result bases include `NotCorrectnessProof`, `NotTestSufficiencyProof`, and `NotHumanReviewReplacement` in `disclosure.non_guarantees`.
- An Unrecorded Change does not establish actor attribution, intent, correctness,
  review completion, or test sufficiency.
- Unverified claims, provenance-missing evidence, stale observation provenance, and cooperative agent reports may be recorded as evidence history, but they do not satisfy required close evidence when the close path requires stronger provenance.
- Rejected, deferred, stale, superseded, expired, invalid-basis, agent-recorded, provenance-missing, or outcome-absent cancellation judgments do not permit cancellation.

## Rejected result

The method returns `ToolRejectedResponse` when the request fails before a valid close readiness result or terminal-path evaluation.

Common rejected cases include:

- validation failure
- actor-source or operation-category mismatch
- stale `expected_state_version`
- idempotency request-hash conflict
- wrong-project or unreadable `Task` identity
- unavailable Core
- unsupported invocation context

Rejected responses:

- return no close-family result or `blockers`
- create no close effect
- create no write ticket, final acceptance, residual-risk acceptance, evidence, or artifact state

Public error meaning, precedence, and response-branch routing are owned by the API error documents linked below.

## `dry_run` behavior

`volicord.check_close` with `dry_run=true` remains the read-only
`CheckCloseResult` branch with `base.effect_kind=read_only`.

Mutating `intent` values with `dry_run=true` use `ToolDryRunResponse` after valid preflight. Preview blockers are `PlannedBlocker` data, not stored `CloseReadinessBlocker` objects.

Pre-preview failures with `dry_run=true` return `ToolRejectedResponse`, not `DryRunSummary.would_errors[]` or `PlannedBlocker`.

Branch shapes are owned by [API Schema Core](schema-core.md). Response-branch routing is owned by [API error routing](error-routing.md). Close readiness blocker/API response routing is owned by [API blocker routing](blocker-routing.md).

## Storage effect

`volicord.check_close` has no Core authority-state storage effect, including
when it returns blockers or uses `dry_run=true`. It does not create replay
rows, append events, persist close blocker rows, mutate `close_state`, touch
artifacts or evidence, or increment `project_state.state_version`.

Committed `dry_run=false` mutating intents persist only successful terminal outcomes. A blocked mutating intent returns a response-only `base.effect_kind=no_effect` result and leaves terminal state unchanged. A successful terminal close may persist a terminal close summary, distinct from the current close basis used for pre-close readiness. Successful `intent=complete` may also persist project continuity records with `kind=known_limit` for current close-basis residual risks that are visible but do not require residual-risk acceptance. Exact storage effects, replay rows, events, state-version increments, and project continuity persistence are owned by [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).

Every returned authority receipt derives `completion_claim_allowed`; callers
must treat `false` as an authority boundary and must not emit a completion claim
from summary wording, a successful transport, or a partial result.

Rejected responses and valid `ToolDryRunResponse` previews for mutating `intent` values have no storage effect.

## Examples

The examples are intentionally compact. They illustrate the method branch and keep nested schema, storage, and display details with their owners.

### Minimal valid request

```yaml contract=api.method.check_close.request shape=complete_request
method: volicord.check_close
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
```

### Representative blocked check response

Read-only `CheckCloseResult` for `task_close_001` at `state_version: 72`, where the method-local response reports one final-acceptance blocker:

```schema
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
    produced_at_state_version: 72
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
    - acceptance_criterion_id: criterion_onboarding_review_001
      statement: "The onboarding checklist is ready for user review."
      evidence_requirement: not_required
  autonomy_boundary: "Stay within onboarding checklist completion."
  active_change_unit_ref: null
  baseline_ref: baseline_close_001
  # The current complete WorkflowProjection is omitted from this abbreviated example.
  pending_user_action_summaries: []
  blocker_refs: []
  write_ticket_summary: null
  evidence_summary: null
  evidence_gate:
    state: not_required
  close_state: blocked
  close_blockers:
    - category: final_acceptance
      code: missing_final_acceptance
      message: "Final acceptance is still required before this Task can close."
      related_refs: []
      next_actions:
          action_kind: request_user_action
          owner_method: volicord.request_user_action
          allowed_operation_categories: [agent_workflow]
          label: "The Agent Connection must create a current final-acceptance request for the user."
          blocking_question: "Does the user accept the current Task result and close basis as complete?"
          expected_state_version: 72
          required_refs:
            - record_kind: task
              record_id: task_close_001
              project_id: proj_close_001
              task_id: task_close_001
              produced_at_state_version: 72
  guarantee_display: null
blockers:
  - category: final_acceptance
    code: missing_final_acceptance
    message: "Final acceptance is still required before this Task can close."
    related_refs: []
    next_actions:
        action_kind: request_user_action
        owner_method: volicord.request_user_action
        allowed_operation_categories: [agent_workflow]
        label: "The Agent Connection must create a current final-acceptance request for the user."
        blocking_question: "Does the user accept the current Task result and close basis as complete?"
        expected_state_version: 72
        required_refs:
          - record_kind: task
            record_id: task_close_001
            project_id: proj_close_001
            task_id: task_close_001
            produced_at_state_version: 72
evidence_summary: null
evidence_gate:
  state: not_required
artifact_refs: []
```

## Owner links

- Request envelope, common response branches, and `dry_run` summaries: [API Schema Core](schema-core.md).
- `CheckCloseResult.blockers`, `CloseTaskResult.blockers`, `CurrentCloseBasis`, `RiskAcceptanceCoverage`, `CloseReadinessBlocker`, `ProjectContinuitySummary`, `EvidenceSummary`, `EvidenceGateSummary`, `StateSummary`, and `NextActionSummary` shapes: [API State Schemas](schema-state.md#close-readiness-and-validation-shapes).
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
