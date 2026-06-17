<a id="harnessstatus"></a>

# `harness.status` reference

## What this document owns

This document owns baseline method behavior for `harness.status`:

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

`harness.status` returns a read-only current-position view over Core state. The view can include current Task summary, blockers, pending user judgments, `Write Authorization` summary, evidence summary, close state, close-readiness findings, guarantee display, and next safe actions.

## Required inputs

- A valid `ToolEnvelope`; `idempotency_key` and `expected_state_version` may be `null`.
- `include` flags selecting which summaries the caller needs.

## Request schema

This method owns the top-level `params` request shape below. `envelope` is the shared [`ToolEnvelope`](schema-core.md#tool-envelope); this block does not redefine `ToolEnvelope` fields.

```yaml
StatusRequest:
  envelope: ToolEnvelope
  include: object
```

Field notes:
- `include` is the method-local flag object selecting status summaries, as shown in the minimal valid request example.

## Access requirements

When protected Core detail is requested, the read requires:

- same-project current local surface
- `VerifiedSurfaceContext.access_class=read_status`

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
- `Write Authorization` change

## Success result

Returns `StatusResult` with:

- `base.response_kind=result`
- `base.effect_kind=read_only`

When `include.close=true`, `StatusResult.close_blockers` are read-only `CloseReadinessBlocker[]` observations.

Non-claim: `StatusResult.close_blockers` are not stored close results.

## Method result fields

`StatusResult` is the method-specific result branch for a successful status read. It carries `base: ToolResultBase` and these method-owned top-level fields:

| Field | Result-field meaning |
|---|---|
| `base` | Common result metadata. The `ToolResultBase` shape is owned by [API Schema Core](schema-core.md#common-response). Read-only status results use `events: []`; `EventRef.event_kind`, when present in a common response branch, remains an opaque illustrative classification string. |
| `active_task` | `StateSummary | null` for the currently selected Task summary. |
| `status_summary` | Free-form display string summarizing the current status view. |
| `next_actions` | `NextActionSummary[]` describing the next safe API steps. |
| `pending_user_judgments` | `StateRecordRef[]` for pending user-judgment records selected into the status view. |
| `blocker_refs` | `StateRecordRef[]` for blocker records visible in the current status view. |
| `close_state` | Status close-state value for the current view. Supported values, including `none` when no current close state is available, are owned by [API Value Sets](schema-value-sets.md#task-lifecycle-values). |
| `close_blockers` | Read-only `CloseReadinessBlocker[]` observations for the current view. They are not stored close results. |
| `guarantee_display` | `GuaranteeDisplay | null` for the current status view. |

Nested `StateSummary`, `StateRecordRef`, `CloseReadinessBlocker`, `GuaranteeDisplay`, and `NextActionSummary` shapes are owned by [API State Schemas](schema-state.md).

## Blocked result

There is no committed blocked branch.

Blockers and close blockers in a `StatusResult` are computed response fields only.

## Rejected result

Returns `ToolRejectedResponse` only when the read cannot be safely served, such as:

- unavailable Core
- local access mismatch
- insufficient capability for the requested protected detail
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
method: harness.status
params:
  envelope:
    project_id: proj_export_001
    task_id: task_export_001
    actor_kind: agent
    surface_id: surface_status
    request_id: req_status_export_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: en-US
  include:
    task: true
    pending_user_judgments: true
    write_authority: false
    evidence: false
    close: true
    guarantees: true
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
  write_authority_summary: null
  evidence_summary: null
  close_state: blocked
  close_blockers:
    - category: user_judgment
      code: missing_user_judgment
      message: "User-owned product decision about CSV column order is still pending."
      related_refs:
        - record_kind: user_judgment
          record_id: uj_export_columns_001
          project_id: proj_export_001
          task_id: task_export_001
          state_version: 42
      next_actions:
        - action_kind: record_user_judgment
          owner_method: harness.record_user_judgment
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
status_summary: "A user-owned product decision about CSV column order is pending."
next_actions:
  - action_kind: record_user_judgment
    owner_method: harness.record_user_judgment
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
close_blockers:
  - category: user_judgment
    code: missing_user_judgment
    message: "User-owned product decision about CSV column order is still pending."
    related_refs:
      - record_kind: user_judgment
        record_id: uj_export_columns_001
        project_id: proj_export_001
        task_id: task_export_001
        state_version: 42
    next_actions:
      - action_kind: record_user_judgment
        owner_method: harness.record_user_judgment
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
- Status state, close-readiness shapes, evidence summaries, and guarantee display: [API State Schemas](schema-state.md).
- Supported values and access classes: [API Value Sets](schema-value-sets.md).
- Public errors, precedence, and rejected-response routing: [API error codes](error-codes.md), [API error precedence](error-precedence.md), and [API error routing](error-routing.md).
- Close-readiness blocker routing: [API blocker routing](blocker-routing.md).
- Persistence effects: [Storage Effects](../storage-effects.md).
