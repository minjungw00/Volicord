<a id="volicordstatus"></a>

# `volicord.status` reference

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
- project continuity, guarantee display, and next safe actions
- requested and effective control, the project-policy basis, and whether the
  current authority permits a completion claim

Every successful result also includes the compact `summary_card`.
When a Task is selected, every successful result also includes a freshly
computed `authority_receipt`, independent of optional `include` fields.

## Required inputs

- A valid `ToolEnvelope`; `idempotency_key` and `expected_state_version` may be `null`.
- `include` flags selecting which summaries the caller needs.

## Request schema

This method owns the top-level `params` request shape below. `envelope` is the shared [`ToolEnvelope`](schema-core.md#tool-envelope); this block does not redefine `ToolEnvelope` fields.

All fields shown in this method-owned request block are required members of `params` unless a field note explicitly marks a member optional; `T | null` means the member must be present and may contain JSON `null`.

```yaml
StatusRequest:
  envelope: ToolEnvelope
  include: object
  continuity_page?: ContinuityPageRequest | null
```

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

Exact identifiers used in this section: `suspected`.

Returns `StatusResult` with:

- `base.response_kind=result`
- `base.effect_kind=read_only`
- `base.disclosure.guarantee_class=authority_record`

When `include.close=true`, `StatusResult.close_blockers` are read-only `CloseReadinessBlocker[]` observations.

Non-claim: `StatusResult.close_blockers` are not stored close results, correctness proof, test sufficiency proof, or human review replacement. `base.disclosure.non_guarantees` carries the stable machine-readable values.

Include projection contract:

- `include.task` returns the selected `Task` summary and current Change Unit through `active_task`.
- `include.pending_user_actions` returns `pending_user_action_summaries`. The
  public result never returns User Channel availability, the Core-derived
  request body, canonical form, or a complete `UserActionInboxItem`. A verified
  User Channel renderer fetches the availability and complete item through its
  separate internal Core boundary. Relevant stale or superseded records can
  still appear as opaque authority refs in owner-defined state and next-action
  fields; those refs are not request-detail projections.
- `include.write_ticket` returns active, invalidated, consumed, or otherwise relevant write-ticket state through `write_ticket_summary`. An invalidated summary exposes its stable invalidation reason and validity basis; an optional project idle timeout is represented by `idle_timeout`, not by a fixed lifetime.
- `write_ticket_summary` is a compatibility summary only; it is not filesystem access, shell approval, final acceptance, ordinary write approval, or proof that a write occurred.
- `include.evidence` returns current `EvidenceSummary` and coverage when available, plus the canonical `evidence_gate` projection.
- `include.close` returns `CurrentCloseBasis | null`, close state, computed blockers, risk acceptance coverage, relevant next actions, and the same canonical `evidence_gate`. The blockers use the same close-readiness calculation as `volicord.check_close`.
- When evidence or close details are selected, `summary_card.evidence` is exactly `evidence_gate.state`. It uses `not_required`, `optional_none`, `required_missing`, `partial`, `sufficient`, `stale`, or `blocked`; it does not derive a second gate from evidence attachment display state.
- `include.guarantees` returns only guarantees derived from the project enforcement profile, verified invocation context, enabled enforcement mechanisms, and supported baseline scope.
- `include.continuity` returns a `ProjectContinuityPage` containing active
  `ProjectContinuitySummary` entries and explicit page information for durable
  project-level context.
- `include.continuity` also returns `task_flow`, the connected predecessor
  component for the selected Task, including branches joined by canonical
  lineage edges.
- `summary_card` is always returned on successful `StatusResult` responses. It summarizes the owner-selected view with public display terminology and one selected `next` action when knowable. It does not add authority beyond the structured fields it summarizes.
- `include.evidence=false` omits `evidence_summary`; `evidence_gate` is still returned when `include.close=true`.
- `include.close=false` omits `CurrentCloseBasis`, optional close-state and
  blocker projections, residual-risk coverage, and close-only top-level actions.
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
- Confirmed unresolved Unrecorded Changes contribute the close blocker.
  Suspected changes remain visible as warnings or verification requests and do
  not block close unless later promoted to `confirmed`.
- A terminal selected Task projects its stored terminal state as `closed`,
  `cancelled`, or `superseded`, with an empty close-blocker set and no next
  action. A non-terminal `ready` close state selects `volicord.close_task` as
  the Agent's next action before generic workflow suggestions.
- Uncomputed or unselected optional projections are omitted where the schema permits. Fixed-shape top-level fields remain `null` or empty when their corresponding `include` flag is false; interpret those values together with the request's `include` object.
- When a projection is selected, `null` means it was computed but no value is available, and an empty array, including empty close blockers, means it was computed and no entries were found.
- Capability declarations alone do not create guarantees.
- `GuaranteeDisplay.capability_refs` should identify invocation binding, Agent Connection, or observation facts when those refs are available.

`include.evidence=true` or `include.close=true` and [`volicord.check_close`](method-close-task.md#volicordcheck_close) use the same close-readiness evidence-gate calculation. Therefore an evidence-only status result and check-close result at the same state version return the same `evidence_gate`; selecting close controls exposure of close fields, not a second gate calculation. `volicord.status` creates no replay row, event, Core state mutation, close mutation, or state-version increment.

## Method result fields

Exact identifiers used in this section: `Task`.

`StatusResult` is the method-specific result branch for a successful status read. It carries `base: ToolResultBase` and these method-owned top-level fields:

| Field | Result-field meaning |
|---|---|
| `base` | Common result metadata. The `ToolResultBase` shape is owned by [API Schema Core](schema-core.md#common-response). Read-only status results use `events: []` and an authority-record disclosure; `EventRef.event_kind`, when present in a common response branch, remains an opaque illustrative classification string. |
| `summary_card` | `SummaryCard` for the selected status view. Its evidence display copies `evidence_gate.state` when evidence or close details are selected. Shape is owned by [API State Schemas](schema-state.md#current-position-display-shapes). |
| `active_task` | `StateSummary | null` for the currently selected `Task` summary. |
| `status_summary` | Free-form display string summarizing the current status view. When close-readiness is selected, it may summarize the current close-readiness state or the first close blocker code; the structured authority facts remain in the other result fields. |
| `next_actions` | `NextActionSummary[]` describing the next safe API steps. A non-empty list has exactly one `presentation_role=primary`; `summary_card.next_action` selects that action rather than relying on array position. |
| `pending_user_action_summaries` | `AgentSafeUserActionRequestSummary[]` containing only request ID, `status=pending`, and `next_actor=user` for each selected pending request. This is the Agent Connection projection. |
| `blocker_refs` | `StateRecordRef[]` for blocker records visible in the current status view. |
| `write_ticket_summary` | `WriteTicketStateSummary | null` for the write-ticket projection. When `include.write_ticket=true`, `null` means no relevant write ticket is available; when the projection is not selected, this fixed-shape field remains `null`. Shape is owned by [API State Schemas](schema-state.md#current-position-display-shapes). |
| `evidence_summary` | `EvidenceSummary | null` when `include.evidence=true`; explicit `null` means the selected projection found no current evidence summary. The field is omitted when `include.evidence=false`. Shape is owned by [API State Schemas](schema-state.md#evidence-and-run-snapshot-shapes). |
| `evidence_gate` | `EvidenceGateSummary | null` when `include.evidence=true` or `include.close=true`; explicit `null` means no Task-scoped gate is available. The field is omitted when neither projection is selected. `active_task.evidence_gate` and `summary_card.evidence` copy this projection. |
| `close_state` | Status close-state value for the current view. Supported values, including `none` when no current close state is available, are owned by [API Value Sets](schema-value-sets.md#task-lifecycle-values). |
| `current_close_basis` | `CurrentCloseBasis | null` selected into the close status view. Shape is owned by [API State Schemas](schema-state.md#close-readiness-and-validation-shapes). |
| `risk_acceptance_coverage` | `RiskAcceptanceCoverage[]` for current residual-risk acceptance coverage in the close status view. Shape is owned by [API State Schemas](schema-state.md#close-readiness-and-validation-shapes). |
| `close_blockers` | Read-only `CloseReadinessBlocker[]` observations for the current view. They are not stored close results. |
| `guarantee_display` | `GuaranteeDisplay | null` for the current status view. |
| `continuity_summary` | `ProjectContinuityPage` when `include.continuity=true`; omitted when the projection is not selected. It always reports `items` and complete `page_info`, including an empty page. Shape is owned by [API State Schemas](schema-state.md#project-continuity-shapes). |
| `task_flow` | `TaskFlowItem[]` when `include.continuity=true` and a Task is selected; omitted otherwise. It is the connected lineage projection, not inherited current authority. |
| `authority_receipt` | Fresh `AuthorityReceipt` whenever a Task is selected, otherwise `null`. It uses the same observed `state_version` and carries the complete close-blocker set, latest recorded Run, product-write observation, evidence gate, next actor/action, and derived `completion_claim_allowed`. Shape is owned by [API State Schemas](schema-state.md#task-lineage-workspace-and-authority-receipt). |

The nested `AgentSafeUserActionRequestSummary` shape is owned by
[API User Action Schemas](schema-user-action.md#inbox-and-capture-form). Nested
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

## Dry-run behavior

`dry_run=true` does not create a `ToolDryRunResponse` branch for this read-style method.

A valid request returns the same `StatusResult` shape with:

- `base.dry_run=true`
- `base.effect_kind=read_only`

## Storage effect

This method does not persist Core state changes, events, replay rows, close mutations, or state-version increments. Exact persistence semantics are owned by the storage documents linked below.

The examples are intentionally compact and method-local. The representative
response is the public agent-safe status projection. A verified User Channel
renderer obtains the complete canonical inbox form through its separate
internal Core boundary, never through `StatusResult`. The response is
abbreviated to the fields needed to show the status branch, observed refs,
state version, current scope, current Change Unit, close state, and next actions.

Method-local precondition: `task_export_001`, `cu_export_001`, and `ua_export_columns_001` already exist in `proj_export_001` at the listed state versions. The read-only response observes those refs; it does not create them.

## Minimal valid request

```yaml
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

```yaml
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
  shaping_readiness: null
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
        - presentation_role: primary
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
next_actions:
  - presentation_role: primary
    action_kind: resolve_user_action
    owner_method: volicord.resolve_user_action
    allowed_operation_categories: [user_only]
    label: "The user must resolve the pending action through a User Channel."
    blocking_question: null
    expected_state_version: null
    required_refs: []
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
      - presentation_role: primary
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
