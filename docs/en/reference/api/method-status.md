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

## Minimal valid request

```yaml
method: harness.status
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_status_001
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

Result branch (`StatusResult`, read-only):

```yaml
base:
  response_kind: result
  effect_kind: read_only
  dry_run: false
  state_version: 22
  events: []
active_task:
  project_id: proj_123
  state_version: 22
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 22
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Add a confirmation step before invoice PDF download."
  scope_summary: "Invoice PDF download confirmation and related tests."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_001
    project_id: proj_123
    task_id: task_456
    state_version: 19
status_summary: "A user-owned product decision about the invoice download confirmation copy is pending."
next_actions:
  - action_kind: record_user_judgment
    owner_method: harness.record_user_judgment
    label: "Record the user's answer for the pending product decision."
    blocking_question: "What is the user's answer for the pending product decision?"
    required_refs:
      - record_kind: user_judgment
        record_id: uj_001
        project_id: proj_123
        task_id: task_456
        state_version: 22
pending_user_judgments:
  - record_kind: user_judgment
    record_id: uj_001
    project_id: proj_123
    task_id: task_456
    state_version: 22
blocker_refs: []
close_readiness:
  ready: false
  blockers:
    - code: missing_user_judgment
      message: "User-owned product decision is still pending."
guarantee_display:
  level: cooperative
  notes:
    - "No stronger local guarantee is currently applied."
```

## Owner links

- Request envelope and response branches: [API Schema Core](schema-core.md).
- Status state, close-readiness shapes, evidence summaries, and guarantee display: [API State Schemas](schema-state.md).
- Supported values and access classes: [API Value Sets](schema-value-sets.md).
- Public errors, precedence, and rejected-response routing: [API error codes](error-codes.md), [API error precedence](error-precedence.md), and [API error routing](error-routing.md).
- Close-readiness blocker routing: [API blocker routing](blocker-routing.md).
- Persistence effects: [Storage Effects](../storage-effects.md).
