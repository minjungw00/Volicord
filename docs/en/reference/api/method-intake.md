<a id="harnessintake"></a>

# `harness.intake` reference

## What this document owns

This document owns baseline method behavior for `harness.intake`:

- method-specific required inputs, access requirements, state-version behavior, result branches, and dry-run behavior
- the scenario request fields and representative response for the shared account data export confirmation scenario
- method-level storage-effect summary and links to storage owners

## What this document does not own

This document does not own:

- common `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, or `ToolDryRunResponse` schema bodies
- nested state, artifact, judgment, value-set, or error schema definitions
- storage DDL, storage record layouts, artifact lifecycle, security guarantees, or Core product meaning

## Purpose

`harness.intake` starts, resumes, supersedes, or rejects an ordinary user work loop.

The method resolves the requested mode to a concrete Task state:

- `advisor`
- `direct`
- `work`

Scope boundary:

- `harness.intake` may create the first scope candidate for write-capable work.
- Subsequent scope changes belong to `harness.update_scope`.

## Required inputs

- `ToolEnvelope` with `project_id`, `surface_id`, `request_id`, `dry_run`, and, for non-dry-run commits, non-null `idempotency_key` and current `expected_state_version`.
- `plain_language_request`, `requested_mode`, and `resume_policy`.
- Put any known initial scope candidate in `initial_scope.boundary`, `initial_scope.non_goals`, and `initial_scope.acceptance_criteria`; use empty arrays for list fields and `initial_context_refs` when none are known.

## Access requirements

Conditions:

- `dry_run=false` commit.
- `VerifiedSurfaceContext.access_class=core_mutation`.
- `verified=true`.

Non-claim: `surface_id` selects a registered local surface; it is not itself authority.

## State version behavior

Committed non-dry-run result:

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

- Task
- Change Unit
- user judgment
- evidence
- blocker
- next-action fields

Non-claim: blocking questions are not represented through separate planning artifacts.

## Rejected result

Returns `ToolRejectedResponse` for pre-commit failures such as:

- validation failure
- stale `expected_state_version`
- unavailable Core or local surface
- local access mismatch
- missing active-task compatibility
- validator failure

Public error code meaning and precedence are owned by [API Errors](errors.md).

## Dry-run behavior

For `dry_run=true`, a valid state-effecting preview:

- returns `ToolDryRunResponse`
- does not return `IntakeResult`

Branch shape is owned by [API Schema Core](schema-core.md); no-effect persistence semantics are owned by [Storage Effects](../storage-effects.md).

## Storage effect

On commit, the method may persist intake-owned Task or Change Unit state. Exact storage effects are owned by [Storage Effects](../storage-effects.md), and storage record shapes are owned by [Storage Records](../storage-records.md).

## Scenario request example

```yaml
method: harness.intake
params:
  plain_language_request: "Add explicit confirmation before account data export."
  initial_scope:
    boundary: "Only the account data export flow and account data export confirmation tests."
    non_goals:
      - "Changing account deletion behavior"
    acceptance_criteria:
      - "Account data export requires explicit confirmation before download."
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
    - event_id: evt_1001
      event_kind: task_intake
task_ref:
  record_kind: task
  record_id: task_456
  project_id: proj_123
  task_id: task_456
  state_version: 18
change_unit_ref: null
state:
  project_id: proj_123
  state_version: 18
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 18
  mode: work
  lifecycle:
    lifecycle_phase: shaping
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Add explicit confirmation before account data export."
  scope_summary: "Only the account data export flow and account data export confirmation tests."
  non_goals:
    - "Changing account deletion behavior"
  acceptance_criteria:
    - "Account data export requires explicit confirmation before download."
  active_change_unit_ref: null
  blocker_refs: []
next_actions:
  - action: harness.update_scope
    reason: "Create the first active Change Unit before write checking."
```

## Owner links

- Request envelope and response branches: [`ToolEnvelope`](schema-core.md#tool-envelope) and [common response branches](schema-core.md#common-response).
- State refs, `StateSummary`, `ShapingReadiness`, and next actions: [API State Schemas](schema-state.md).
- Active method names, mode values, `resume_policy`, `response_kind`, `effect_kind`, and access classes: [API Value Sets](schema-value-sets.md).
- Public errors and state-version conflicts: [API Errors](errors.md).
- Persistence effects: [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).
