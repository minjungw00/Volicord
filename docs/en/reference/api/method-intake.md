<a id="harnessintake"></a>

# `harness.intake` reference

## What this document owns

This document owns baseline method behavior for `harness.intake`:

- method-specific required inputs, access requirements, state version behavior, result branches, and `dry_run` behavior
- intake handling for starting, resuming, superseding, or rejecting a user work loop
- intake examples

## What this document does not own

This document does not own:

- common request envelope, response branch, dry-run, or rejected-response schema bodies
- nested state, artifact, judgment, value-set, or error schema definitions
- storage DDL, storage record layouts, exact storage effects, artifact lifecycle, security guarantees, or Core authority semantics
- public error code meaning, public error precedence, or shared response-branch routing

## Purpose

`harness.intake` starts, resumes, supersedes, or rejects an ordinary user work loop.

The method resolves the requested mode to a concrete Task mode:

- `advisor`
- `direct`
- `work`

Scope boundary:

- `harness.intake` may create the first scope candidate for write-capable work.
- Later scope changes belong to `harness.update_scope`.

## Required inputs

- A valid `ToolEnvelope`; committed non-dry-run requests require non-null `idempotency_key` and current `expected_state_version`.
- `plain_language_request`, `requested_mode`, and `resume_policy`.
- Any known initial scope candidate in `initial_scope.boundary`, `initial_scope.non_goals`, and `initial_scope.acceptance_criteria`; use empty arrays when no list items are known.

## Access requirements

A committed non-dry-run request requires:

- `VerifiedSurfaceContext.access_class=core_mutation`
- `verified=true`

Surface identity boundary:

- `surface_id` selects a registered local surface; `surface_id` is not itself authority.

## State version behavior

A committed non-dry-run result:

- increments project-wide `project_state.state_version` exactly once
- creates the replay row for the idempotency key

The following create no Task, Change Unit, event, replay row, blocker update, or state-version increment:

- dry run
- read failure
- validation failure
- local access failure
- stale `expected_state_version`

## Success result

Returns `IntakeResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `task_ref`
- optional `change_unit_ref`
- current `state`
- `next_actions`

If `requested_mode=auto`, the persisted and displayed mode must be the resolved concrete mode, never `auto`.

## Blocked result

The method may return a committed `IntakeResult` that records shaping or blocker state instead of a write-ready path.

Blocking questions must be represented through:

- Task or Change Unit state
- user judgment, evidence, blocker, or next-action fields
- the schema owners linked below for nested field shapes

## Rejected result

Returns `ToolRejectedResponse` for pre-commit failures such as:

- validation failure
- stale `expected_state_version`
- unavailable Core or local surface
- local access mismatch
- missing current Task compatibility
- validator failure

Public error code meaning, precedence, and rejected-response routing are owned by the error documents linked below.

## Dry-run behavior

For `dry_run=true`, a valid state-effecting preview:

- returns `ToolDryRunResponse`
- does not return `IntakeResult`
- creates no durable intake state

## Storage effect

On commit, the method may persist intake-owned Task or Change Unit state. Exact storage effects and storage record shapes are owned by the storage documents linked below.

## Minimal valid request

```yaml
method: harness.intake
params:
  envelope:
    project_id: proj_onboard_001
    task_id: null
    actor_kind: agent
    surface_id: surface_onboard
    request_id: req_intake_onboard_001
    idempotency_key: idem_intake_onboard_001
    expected_state_version: 17
    dry_run: false
    locale: en-US
  plain_language_request: "Create a first-run checklist for new workspace setup."
  requested_mode: work
  resume_policy: create_new
  initial_scope:
    boundary: "First-run checklist for new workspace setup."
    non_goals:
      - "Changing account creation."
    acceptance_criteria:
      - "New users see the checklist after opening a workspace."
  initial_context_refs: []
```

## Representative response

Result branch (`IntakeResult`, committed):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 18
  events:
    - event_id: evt_onboard_001
      event_kind: task_intake
task_ref:
  record_kind: task
  record_id: task_onboard_001
  project_id: proj_onboard_001
  task_id: task_onboard_001
  state_version: 18
change_unit_ref: null
state:
  project_id: proj_onboard_001
  state_version: 18
  task_ref:
    record_kind: task
    record_id: task_onboard_001
    project_id: proj_onboard_001
    task_id: task_onboard_001
    state_version: 18
  mode: work
  lifecycle:
    lifecycle_phase: shaping
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Create a first-run checklist for new workspace setup."
  scope_summary: "First-run checklist for new workspace setup."
  non_goals:
    - "Changing account creation."
  acceptance_criteria:
    - "New users see the checklist after opening a workspace."
  active_change_unit_ref: null
  blocker_refs: []
next_actions:
  - action_kind: update_scope
    owner_method: harness.update_scope
    label: "Create the first currently applied Change Unit before write checking."
    blocking_question: null
    required_refs:
      - record_kind: task
        record_id: task_onboard_001
        project_id: proj_onboard_001
        task_id: task_onboard_001
        state_version: 18
```

## Owner links

- Request envelope and response branches: [`ToolEnvelope`](schema-core.md#tool-envelope) and [common response branches](schema-core.md#common-response).
- State refs, `StateSummary`, `ShapingReadiness`, and next actions: [API State Schemas](schema-state.md).
- Supported method names, mode values, `resume_policy`, `response_kind`, `effect_kind`, and access classes: [API Value Sets](schema-value-sets.md).
- Public errors, precedence, and rejected-response routing: [API error codes](error-codes.md), [API error precedence](error-precedence.md), and [API error routing](error-routing.md).
- Persistence effects and storage records: [Storage Effects](../storage-effects.md), [Storage Records](../storage-records.md), and [Storage Versioning](../storage-versioning.md).
