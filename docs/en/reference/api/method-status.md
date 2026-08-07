<a id="volicordstatus"></a>

# `volicord.status` reference

Current progression is returned as the tagged `workflow` projection. Its transition catalog contains exactly zero or one required descriptor and remains independent of `close_state` and `close_blockers`. Current checkpoint readiness, typed gaps, boundary, and pending decision refs appear inside `workflow.checkpoint`. Close readiness is review data, not a progression selector.

## What this document owns

This document owns baseline method behavior for `volicord.status`:

- method-specific required inputs, access requirements, state version behavior, result branches, and `dry_run` behavior
- current Core-state status behavior and its no-state-version effect boundary
- status examples

## What this document does not own

This document does not own:

- common request envelope, response branch, `dry_run`, or rejected-response schema bodies
- nested state, artifact, judgment, value-set, or error schema definitions
- storage DDL, storage record layouts, exact storage effects, artifact lifecycle, security guarantees, or Core authority semantics
- public error code meaning, public error precedence, or shared response-branch routing

## Purpose

`volicord.status` returns a current-position view over Core state. Callers can select:

- the current `Task` and Change Unit
- blockers, pending user actions, available User Channel resolution paths, and write-ticket state
- evidence and close-readiness observations
- project continuity, guarantee display, and the authoritative tagged workflow
- requested and effective control, the project-policy basis, and whether the
  current authority permits a completion claim

Every successful result also includes the compact `summary_card`.
When a Task is selected, every successful result also includes a freshly
computed `authority_receipt`, independent of optional `include` fields.

## Required inputs

- A valid `ToolEnvelope`; `idempotency_key` and `expected_state_version` may be `null`.
- `include` flags selecting which summaries the caller needs.

## Request schema

This method owns the top-level `params` request fields in the generated table
below. `envelope` is the shared [`ToolEnvelope`](schema-core.md#tool-envelope);
the table does not redefine `ToolEnvelope` fields. Requiredness and nullability
come directly from the semantic request descriptor.

<!-- BEGIN GENERATED: contract-structures api.method.status.request[params] -->
<!-- This region is generated from maintained sources; do not edit it directly. -->

### `StatusRequest` fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `continuity_page` | no | yes | `ContinuityPageRequest` |
| `envelope` | yes | no | `ToolEnvelope` |
| `include` | yes | no | `StatusInclude` |
<!-- END GENERATED: contract-structures api.method.status.request[params] -->



Field notes:
- `include` is the method-local flag object selecting status summaries, as shown in the minimal valid request example.
- `continuity_page` is optional and applies only when `include.continuity=true`.
  Omitted or `null` selects `page_size=8` and `cursor=null`. A non-null object
  supplies both a `page_size` from 1 through 64 and a `cursor` that is either
  `null` for the first page or the exact `next_cursor` returned by an earlier
  page. Supplying a non-null object while `include.continuity=false` is
  rejected as unused ambiguous input.

## Access requirements

When protected Core detail is requested, the read requires:

- same-project verified invocation context
- `operation_category=read`

For this response, state authority comes from the Core-owned state summarized in `StatusResult`.

## State version behavior

No state change occurs and `project_state.state_version` never increments.

The result may report the current observed state version.

The method creates no:

- event
- replay row
- close mutation
- artifact effect
- staged-handle consumption
- evidence update
- write-ticket change

In particular, a status read and its observed `state_version` do not invalidate
or consume a write ticket. `WriteTicket.basis_state_version` is audit ordering
metadata, not a validity coordinate.

## Success result

Returns `StatusResult` with:

- `base.response_kind=result`
- `base.effect_kind=read_only`
- `base.disclosure.guarantee_class=authority_record`

When `include.close=true`, `StatusResult.close_blockers` are read-only `CloseReadinessBlocker[]` observations.

Non-claim: `StatusResult.close_blockers` are not stored `close_task` results, correctness proof, test sufficiency proof, or human review replacement. `base.disclosure.non_guarantees` carries the stable machine-readable values.

`include` projection contract:

- `include.task` returns the selected `Task` summary and current Change Unit through `active_task`.
- `include.pending_user_actions` returns `pending_user_action_summaries`. The
  public result never returns User Channel availability, the Core-derived
  request body, resolution form, or a complete `CliUserActionInboxItem`. A
  verified User Channel renderer fetches the availability and complete item
  through its separate internal Core boundary. A stale shaping application may
  appear only as a current recovery obligation in the tagged workflow and does
  not authorize work. Superseded requests, resolutions, applications, and
  checkpoints are excluded from current pending projection, workflow refs,
  blockers, and progression. Their immutable records remain available only
  through explicitly historical reads, diagnostics, or authority export; a
  historical ref is not a current request-detail projection.
- `include.write_ticket` returns active, invalidated, consumed, or otherwise relevant write-ticket state through `write_ticket_summary`. An invalidated summary exposes its stable invalidation reason and validity basis; an optional project idle timeout is represented by `idle_timeout`, not by a fixed lifetime.
- `write_ticket_summary` is a compatibility summary only; it is not filesystem access, shell approval, final acceptance, ordinary write approval, or proof that a write occurred.
- `include.evidence` returns current `EvidenceSummary` and coverage when available, plus the canonical `evidence_gate` projection.
- `include.close` returns `CurrentCloseBasis | null`, close state, computed blockers, risk acceptance coverage, blocker-local remediation actions, and the same canonical `evidence_gate`. The blockers use the same close-readiness calculation as `volicord.check_close`; their actions do not select current progression.
- When evidence or close details are selected, `summary_card.evidence` is exactly `evidence_gate.state`. It uses `not_required`, `optional_none`, `required_missing`, `partial`, `sufficient`, `stale`, or `blocked`; it does not derive a second gate from evidence attachment display state.
- `include.guarantees` returns only guarantees derived from the project enforcement profile, verified invocation context, enabled enforcement mechanisms, and supported baseline scope.
- `include.continuity` returns a `ProjectContinuityPage` containing active
  `ProjectContinuitySummary` entries and explicit page information for durable
  project-level context.
- `include.continuity` also returns `task_flow`, the connected predecessor
  component for the selected Task, including branches joined by canonical
  lineage edges.
- `summary_card` is always returned on successful `StatusResult` responses. It summarizes the owner-selected view with public display terminology and may carry one display-only `next` hint. It does not add authority beyond the structured fields it summarizes and never replaces `active_task.workflow`.
- `include.evidence=false` omits `evidence_summary`; `evidence_gate` is still returned when `include.close=true`.
- `include.close=false` omits `CurrentCloseBasis`, optional close-state and
  blocker projections, residual-risk coverage, and blocker-local remediation.
  Core still evaluates the same read-only close basis for the mandatory
  `authority_receipt`; the receipt carries the full blocker set even when those
  optional top-level fields are omitted.
- `include.guarantees=false` means guarantee display is not derived and not returned.
- `include.continuity=false` means project continuity summaries are not read or returned.

Continuity pagination is ordered exactly by `updated_at DESC,
continuity_record_id DESC`. A cursor is exclusive: the next page starts after
the cursor's complete ordering pair. Equal timestamps are therefore ordered by
the ID tie-breaker rather than Store row order. For an unchanged project and
request, repeated reads return the same page.

`page_info.total_count` is the full number of active project continuity records
at this status read, before applying the cursor. `returned_count` is
`items.len`. `truncated=true` only when at least one later item exists after
the returned page, and only then is `next_cursor` the ordering pair of the last
returned item. Exactly `page_size` total matching records yields
`truncated=false` and `next_cursor=null`; more than `page_size` yields
`truncated=true`. The maximum of 64 bounds the transport payload and is not a
hidden product truncation rule.

Truthful projection rules:
- `authority_receipt.latest_run_ref` uses durable `run_recorded` authority-event
  commit order. Run IDs and equal-millisecond timestamps never determine which
  Run is latest.
- `active_task` exposes `requested_control_level`, `effective_control_level`,
  and `control_level_reason`, together with the authoritative
  `policy_schema`, `policy_version`, `policy_fingerprint`, and policy source.
  Status never derives a lower effective control than the stored value. When a
  durable `policy_control_reevaluation` mark remains unsatisfied, the read-only
  projection reports the strongest stored, current-policy, and marked control
  and acceptance requirements without clearing the mark.
- `authority_receipt.completion_claim_allowed` is `true` only when the current
  close basis is valid and the complete close-blocker set is empty. It is
  `false` when no active Task exists, authority refresh cannot be completed, or
  any blocker remains; display text and an agent assertion cannot override it.
- Every unresolved Unrecorded Change represents a complete observed non-empty
  unmatched Product Repository delta and contributes the close blocker.
  Observation unavailability remains a separate diagnostic and is never
  projected as an Unrecorded Change.
- A terminal selected Task projects its stored terminal state as `closed`,
  `cancelled`, or `superseded`, with an empty close-blocker set and tagged
  `workflow.kind=terminal`. A non-terminal close state never selects a global
  action ahead of the tagged workflow. A caller begins close review with
  `volicord.check_close` only when the current workflow allows it; the review
  does not change the workflow kind.
- Uncomputed or unselected optional projections are omitted where the schema permits. Fixed-shape top-level fields remain `null` or empty when their corresponding `include` flag is false; interpret those values together with the request's `include` object.
- When a projection is selected, `null` means it was computed but no value is available, and an empty array, including empty close blockers, means it was computed and no entries were found.
- Capability declarations alone do not create guarantees.
- `GuaranteeDisplay.capability_refs` should identify invocation binding, Agent Connection, or observation facts when those refs are available.

`include.evidence=true` or `include.close=true` and [`volicord.check_close`](method-close-task.md#volicordcheck_close) use the same close-readiness evidence-gate calculation. Therefore an evidence-only status result and check-close result at the same state version return the same `evidence_gate`; selecting close controls exposure of close fields, not a second gate calculation. `volicord.status` creates no replay row, event, Core state mutation, close mutation, or state-version increment.

The administrative CLI preserves this complete `StatusResult` and
`SummaryCard` in `volicord status --json`. Its default human display is an
adapter-owned applicability projection: it selects the no-active-Task or
active-Task facts actually returned and does not synthesize absent collection
counts. Human visibility is not a second method result and does not change the
read-only effect or guarantee boundary.

## Method result fields

`StatusResult` is the method-specific result branch for a successful status read. It carries `base: StatusResultBase`, whose only result effect is `read_only`, and these method-owned top-level fields:

<!-- BEGIN GENERATED: contract-structures api.method.status.response[response_variants] api.method.status.response[result_body] api.method.status.response[result_metadata] api.method.status.response[rejection] -->
<!-- This region is generated from maintained sources; do not edit it directly. -->

### `StatusResult` success fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `active_task` | no | yes | `StatusStateSummary` |
| `authority_receipt` | no | yes | `AuthorityReceipt` |
| `base` | yes | no | `StatusResultBase` |
| `blocker_refs` | yes | no | `StateRecordRef[]` |
| `close_blockers` | no | yes | `CloseReadinessBlocker[]` |
| `close_state` | no | yes | `StatusCloseState` |
| `continuity_summary` | no | yes | `ProjectContinuityPage` |
| `current_close_basis` | no | yes | `CurrentCloseBasis` |
| `evidence_gate` | no | yes | `EvidenceGateSummary` |
| `evidence_summary` | no | yes | `EvidenceSummary` |
| `guarantee_display` | no | yes | `GuaranteeDisplay` |
| `pending_user_action_summaries` | yes | no | `AgentSafeUserActionRequestSummary[]` |
| `risk_acceptance_coverage` | no | yes | `RiskAcceptanceCoverage[]` |
| `status_summary` | yes | no | `string` |
| `summary_card` | yes | no | `SummaryCard` |
| `task_flow` | no | yes | `TaskFlowItem[]` |
| `write_ticket_summary` | no | yes | `WriteTicketStateSummary` |

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

### `dry_run` request policy

- `volicord.status`: `dry_run=true` is accepted through the regular result branch with `base.dry_run=true`; it does not create a preview response. `dry_run=false` or an omitted `dry_run` does not select a preview branch.


### Shared response structures

The response descriptor defines success and rejection as an exact `anyOf` branch union. The rejection branch uses the generated [`ToolRejectedResponse`](schema-core.md#common-response) structure. Shared rejection fields remain distinct from the success fields above.
<!-- END GENERATED: contract-structures api.method.status.response[response_variants] api.method.status.response[result_body] api.method.status.response[result_metadata] api.method.status.response[rejection] -->

The nested `AgentSafeUserActionRequestSummary` shape is owned by
[API User Action Schemas](schema-user-action.md#resolution-form). Nested
`SummaryCard`, `StateSummary`, `StateRecordRef`, `WriteTicketStateSummary`,
`EvidenceSummary`, `EvidenceGateSummary`, `ProjectContinuityPage`,
`CurrentCloseBasis`, `RiskAcceptanceCoverage`, `CloseReadinessBlocker`,
`GuaranteeDisplay`, and
`NextActionSummary` shapes are owned by [API State Schemas](schema-state.md).

## Blocked result

There is no committed blocked branch.

Blockers and close blockers in a `StatusResult` are computed response fields only.

## Rejected result

Returns `ToolRejectedResponse` only when the read cannot be safely served, such as:

- unavailable Core
- actor-source or operation-category mismatch
- unsupported invocation context for the requested protected detail
- missing current `Task` for a Task-scoped read
- stale or unavailable projection when a projection-backed view was requested
- corrupt Task, close-basis, evidence, or other owner state required to build
  the canonical authority receipt, even when its optional top-level projection
  was not requested
- a continuity page size outside 1 through 64, a malformed cursor, or a
  non-null `continuity_page` when continuity was not selected

Public error code meaning, precedence, and rejected-response routing are owned by the error documents linked below.

## `dry_run` behavior

`dry_run=true` does not create a separate preview response branch for this
read-style method.

A valid request returns the same `StatusResult` shape with:

- `base.dry_run=true`
- `base.effect_kind=read_only`

## Storage effect

This method does not persist Core state changes, events, replay rows, close mutations, or state-version increments. Exact persistence semantics are owned by the storage documents linked below.

The examples are intentionally compact and method-local. The representative
response is the public agent-safe status projection. A verified User Channel
renderer obtains the complete typed CLI inbox presentation through its separate
internal Core boundary, never through `StatusResult`. The response is
abbreviated to the fields needed to show the status branch, observed refs,
state version, current scope, current Change Unit, close state, and next actions.

Method-local precondition: `task_export_001`, `cu_export_001`, and `ua_export_columns_001` already exist in `proj_export_001` at the listed state versions. The read-only response observes those refs; it does not create them.

## Minimal valid request

```yaml contract=api.method.status.request shape=complete_request
method: volicord.status
params:
  envelope:
    project_id: proj_export_001
    task_id: task_export_001
    request_id: req_status_export_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: en-US
  include:
    task: true
    pending_user_actions: true
    write_ticket: false
    evidence: true
    close: true
    guarantees: true
    continuity: false
```

## Representative response

Abbreviated result branch (`StatusResult`, read-only):

```schema
base:
  response_kind: result
  effect_kind: read_only
  dry_run: false
  state_version: 42
  events: []
active_task:
  project_id: proj_export_001
  state_version: 42
  task_ref:
    record_kind: task
    record_id: task_export_001
    project_id: proj_export_001
    task_id: task_export_001
    produced_at_state_version: 42
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Add CSV summary export for dashboard totals."
  scope_summary: "CSV export column order and summary totals."
  non_goals:
    - "Changing dashboard chart rendering."
  acceptance_criteria:
    - acceptance_criterion_id: criterion_csv_columns_001
      statement: "CSV exports include the selected columns in the specified order."
      evidence_requirement: not_required
  autonomy_boundary: "Stay within CSV summary export behavior."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_export_001
    project_id: proj_export_001
    task_id: task_export_001
    produced_at_state_version: 42
  baseline_ref: baseline_export_001
  # The current complete WorkflowProjection is omitted from this abbreviated example.
  pending_user_action_summaries:
    - user_action_request_id: ua_export_columns_001
      status: pending
      next_actor: user
  blocker_refs: []
  write_ticket_summary: null
  evidence_summary: null
  evidence_gate:
    state: not_required
  close_state: blocked
  close_blockers:
    - category: pending_user_action
      code: pending_user_action
      message: "A user-owned action is pending."
      related_refs: []
      next_actions:
          action_kind: resolve_user_action
          owner_method: volicord.resolve_user_action
          allowed_operation_categories: [user_only]
          label: "The user must resolve the pending action through a User Channel."
          blocking_question: null
          expected_state_version: null
          required_refs: []
  guarantee_display:
    level: cooperative
    basis: "No stronger local guarantee is currently applied."
    capability_refs: []
status_summary: "Close readiness is blocked by pending_user_action."
pending_user_action_summaries:
  - user_action_request_id: ua_export_columns_001
    status: pending
    next_actor: user
blocker_refs: []
evidence_gate:
  state: not_required
close_state: blocked
current_close_basis: null
risk_acceptance_coverage: []
close_blockers:
  - category: pending_user_action
    code: pending_user_action
    message: "A user-owned action is pending."
    related_refs: []
    next_actions:
        action_kind: resolve_user_action
        owner_method: volicord.resolve_user_action
        allowed_operation_categories: [user_only]
        label: "The user must resolve the pending action through a User Channel."
        blocking_question: null
        expected_state_version: null
        required_refs: []
guarantee_display:
  level: cooperative
  basis: "No stronger local guarantee is currently applied."
  capability_refs: []
```

## Owner links

- Request envelope and response branches: [API Schema Core](schema-core.md).
- Status state, current close basis, close-readiness shapes, evidence summaries, and guarantee display: [API State Schemas](schema-state.md).
- Supported values and operation categories: [API Value Sets](schema-value-sets.md#operation-category-values).
- Public errors, precedence, and rejected-response routing: [API error codes](error-codes.md), [API error precedence](error-precedence.md), and [API error routing](error-routing.md).
- Close-readiness blocker routing: [API blocker routing](blocker-routing.md).
- Persistence effects: [Storage Effects](../storage-effects.md).
