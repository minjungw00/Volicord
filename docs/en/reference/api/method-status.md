<a id="volicordstatus"></a>

# `volicord.status` reference

## What this document owns

This document owns baseline method behavior for `volicord.status`:

- method-specific required inputs, access requirements, state version behavior, result branches, and `dry_run` behavior
- read-only status behavior for current Core state
- status examples

## What this document does not own

This document does not own:

- common request envelope, response branch, dry-run, or rejected-response schema bodies
- nested state, artifact, judgment, value-set, or error schema definitions
- storage DDL, storage record layouts, exact storage effects, artifact lifecycle, security guarantees, or Core authority semantics
- public error code meaning, public error precedence, or shared response-branch routing

## Purpose

`volicord.status` returns a read-only current-position view over Core state. The view can include current Task summary, blockers, pending user judgments, `Write Check` summary, evidence summary, close state, close-readiness findings, guard health, project continuity summaries, guarantee display, and next safe actions.

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
```

Field notes:
- `include` is the method-local flag object selecting status summaries, as shown in the minimal valid request example.

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
- `Write Check` change

## Success result

Returns `StatusResult` with:

- `base.response_kind=result`
- `base.effect_kind=read_only`

When `include.close=true`, `StatusResult.close_blockers` are read-only `CloseReadinessBlocker[]` observations.

Non-claim: `StatusResult.close_blockers` are not stored close results.

Include projection contract:

- `include.task` returns the selected `Task` summary and current Change Unit through `active_task`.
- `include.pending_user_judgments` returns current pending judgment refs, and relevant stale or superseded judgment state appears through existing result fields such as `blocker_refs` and `next_actions.required_refs`.
- `include.write_check` returns active, expired, stale, consumed, or otherwise relevant `Write Check` Core-state compatibility record state through `write_check_summary`.
- `write_check_summary` is a compatibility summary only; it is not filesystem access, shell approval, final acceptance, or ordinary write approval.
- `include.evidence` returns current `EvidenceSummary` and coverage when available.
- `include.close` returns `CurrentCloseBasis | null`, close state, computed blockers, risk acceptance coverage, guard health when available, and relevant next actions. The blockers use the same close-readiness calculation as `volicord.close_task intent=check`.
- `include.guarantees` returns only guarantees derived from the project enforcement profile, verified invocation context, enabled enforcement mechanisms, and supported baseline scope.
- `include.continuity` returns active `ProjectContinuitySummary[]` entries for durable project-level context.
- `include.evidence=false` means evidence summaries, coverage, artifact evidence refs, and evidence-only next actions are not computed and not returned.
- `include.close=false` means close readiness is not computed and `CurrentCloseBasis`, close state, close blockers, guard health, residual-risk coverage, and close-only next actions are not returned.
- `include.guarantees=false` means guarantee display is not derived and not returned.
- `include.continuity=false` means project continuity summaries are not read or returned.

Truthful projection rules:
- Uncomputed, unselected, or unavailable data is omitted where the schema permits, or `null` only when the selected projection was computed and unavailable. It is not an empty value that implies "computed and none."
- Empty arrays, including empty close blockers, mean the method computed that field and found no entries.
- Capability declarations alone do not create guarantees. A cooperative-only deployment must not claim `detective`.
- `GuaranteeDisplay.capability_refs` should identify invocation binding, Agent Connection, or observation facts when those refs are available.

`include.close=true` and [`volicord.close_task`](method-close-task.md) with `intent=check` use the same close-readiness calculation. `volicord.status` remains read-only and creates no replay row, event, state mutation, close mutation, or state-version increment.

## Method result fields

`StatusResult` is the method-specific result branch for a successful status read. It carries `base: ToolResultBase` and these method-owned top-level fields:

| Field | Result-field meaning |
|---|---|
| `base` | Common result metadata. The `ToolResultBase` shape is owned by [API Schema Core](schema-core.md#common-response). Read-only status results use `events: []`; `EventRef.event_kind`, when present in a common response branch, remains an opaque illustrative classification string. |
| `active_task` | `StateSummary | null` for the currently selected Task summary. |
| `status_summary` | Free-form display string summarizing the current status view. When close-readiness is selected, it may summarize the current close-readiness state or the first close blocker code; the structured authority facts remain in the other result fields. |
| `next_actions` | `NextActionSummary[]` describing the next safe API steps. |
| `pending_user_judgments` | `StateRecordRef[]` for pending user-judgment records selected into the status view. |
| `blocker_refs` | `StateRecordRef[]` for blocker records visible in the current status view. |
| `close_state` | Status close-state value for the current view. Supported values, including `none` when no current close state is available, are owned by [API Value Sets](schema-value-sets.md#task-lifecycle-values). |
| `current_close_basis` | `CurrentCloseBasis | null` selected into the close status view. Shape is owned by [API State Schemas](schema-state.md#close-readiness-and-validation-shapes). |
| `risk_acceptance_coverage` | `RiskAcceptanceCoverage[]` for current residual-risk acceptance coverage in the close status view. Shape is owned by [API State Schemas](schema-state.md#close-readiness-and-validation-shapes). |
| `close_blockers` | Read-only `CloseReadinessBlocker[]` observations for the current view. They are not stored close results. |
| `guard_health` | `GuardHealthSummary | null` selected into the close status view. Shape is owned by [API State Schemas](schema-state.md#guard-health-summary). |
| `guarantee_display` | `GuaranteeDisplay | null` for the current status view. |
| `continuity_summary` | `ProjectContinuitySummary[]` when `include.continuity=true`; omitted when the projection is not selected. Shape is owned by [API State Schemas](schema-state.md#project-continuity-shapes). |

Nested `StateSummary`, `StateRecordRef`, `ProjectContinuitySummary`, `CurrentCloseBasis`, `RiskAcceptanceCoverage`, `CloseReadinessBlocker`, `GuardHealthSummary`, `GuaranteeDisplay`, and `NextActionSummary` shapes are owned by [API State Schemas](schema-state.md).

## Blocked result

There is no committed blocked branch.

Blockers and close blockers in a `StatusResult` are computed response fields only.

## Rejected result

Returns `ToolRejectedResponse` only when the read cannot be safely served, such as:

- unavailable Core
- actor-source or operation-category mismatch
- unsupported invocation context for the requested protected detail
- missing current Task for a Task-scoped read
- stale or unavailable projection when a projection-backed view was requested

Public error code meaning, precedence, and rejected-response routing are owned by the error documents linked below.

## Dry-run behavior

`dry_run=true` does not create a `ToolDryRunResponse` branch for this read-only method.

A valid request returns the same `StatusResult` shape with:

- `base.dry_run=true`
- `base.effect_kind=read_only`

## Storage effect

This is a read-only method. Exact no-effect persistence semantics are owned by the storage documents linked below.

The examples are intentionally compact and method-local. The representative response is abbreviated to the fields needed to show the status branch, observed refs, state version, current scope, current Change Unit, close state, and next actions.

Method-local precondition: `task_export_001`, `cu_export_001`, and `uj_export_columns_001` already exist in `proj_export_001` at the listed state versions. The read-only response observes those refs; it does not create them.

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
    pending_user_judgments: true
    write_check: false
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
    state_version: 42
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
    - "CSV exports include the selected columns in the approved order."
  autonomy_boundary: "Stay within CSV summary export behavior."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_export_001
    project_id: proj_export_001
    task_id: task_export_001
    state_version: 41
  baseline_ref: baseline_export_001
  shaping_readiness: null
  pending_user_judgment_refs:
    - record_kind: user_judgment
      record_id: uj_export_columns_001
      project_id: proj_export_001
      task_id: task_export_001
      state_version: 42
  blocker_refs: []
  write_check_summary: null
  evidence_summary: null
  close_state: blocked
  close_blockers:
    - category: pending_user_judgment
      code: pending_user_judgment
      message: "User-owned product decision about CSV column order is still pending."
      can_resolve_in_chat: false
      terminal_action_required: false
      related_refs:
        - record_kind: user_judgment
          record_id: uj_export_columns_001
          project_id: proj_export_001
          task_id: task_export_001
          state_version: 42
      next_actions:
        - action_kind: record_user_judgment
          owner_method: volicord.record_user_judgment
          label: "Record the user's answer for the pending CSV column decision."
          blocking_question: "What is the user's answer for the pending CSV column decision?"
          required_refs:
            - record_kind: user_judgment
              record_id: uj_export_columns_001
              project_id: proj_export_001
              task_id: task_export_001
              state_version: 42
  guarantee_display:
    level: cooperative
    basis: "No stronger local guarantee is currently applied."
    capability_refs: []
status_summary: "Close readiness is blocked by pending_user_judgment."
next_actions:
  - action_kind: record_user_judgment
    owner_method: volicord.record_user_judgment
    label: "Record the user's answer for the pending CSV column decision."
    blocking_question: "What is the user's answer for the pending CSV column decision?"
    required_refs:
      - record_kind: user_judgment
        record_id: uj_export_columns_001
        project_id: proj_export_001
        task_id: task_export_001
        state_version: 42
pending_user_judgments:
  - record_kind: user_judgment
    record_id: uj_export_columns_001
    project_id: proj_export_001
    task_id: task_export_001
    state_version: 42
blocker_refs: []
close_state: blocked
current_close_basis: null
risk_acceptance_coverage: []
close_blockers:
  - category: pending_user_judgment
    code: pending_user_judgment
    message: "User-owned product decision about CSV column order is still pending."
    can_resolve_in_chat: false
    terminal_action_required: false
    related_refs:
      - record_kind: user_judgment
        record_id: uj_export_columns_001
        project_id: proj_export_001
        task_id: task_export_001
        state_version: 42
    next_actions:
      - action_kind: record_user_judgment
        owner_method: volicord.record_user_judgment
        label: "Record the user's answer for the pending CSV column decision."
        blocking_question: "What is the user's answer for the pending CSV column decision?"
        required_refs:
          - record_kind: user_judgment
            record_id: uj_export_columns_001
            project_id: proj_export_001
            task_id: task_export_001
            state_version: 42
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
