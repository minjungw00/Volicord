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

## Request schema

This method owns the top-level `params` request shape below. `envelope` is the shared [`ToolEnvelope`](schema-core.md#tool-envelope); this block does not redefine `ToolEnvelope` fields.

```yaml
IntakeRequest:
  envelope: ToolEnvelope
  plain_language_request: string
  requested_mode: string
  resume_policy: string
  initial_scope: object
  initial_context_refs: StateRecordRef[]
```

Nested owner links:
- `initial_context_refs` uses `StateRecordRef[]`; the nested shape is owned by [API State Schemas](schema-state.md#state-references).
- `requested_mode` and `resume_policy` values are owned by [API Value Sets](schema-value-sets.md#task-lifecycle-values) and [method-local values](schema-value-sets.md#method-local-values).

## Access requirements

A committed non-dry-run request requires:

- server-derived `VerifiedSurfaceContext` with `access_class=core_mutation`

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

## Method result fields

`IntakeResult` is the method-specific result branch for a successful committed intake. It carries `base: ToolResultBase` and these method-owned top-level fields:

| Field | Result-field meaning |
|---|---|
| `base` | Common result metadata. The `ToolResultBase` shape, including `events`, is owned by [API Schema Core](schema-core.md#common-response). `base.events[].event_kind`, when present, is an opaque illustrative classification string. |
| `task_ref` | `StateRecordRef` for the Task selected by the intake result. |
| `change_unit_ref` | `StateRecordRef | null` for a Change Unit selected or created during intake, or `null` when no current Change Unit applies yet. |
| `state` | Current `StateSummary` after intake, including current scope and currently applied Change Unit display fields. |
| `next_actions` | `NextActionSummary[]` describing the next safe API steps. |

The supported `resume_policy` input values are owned by [API Value Sets](schema-value-sets.md#method-local-values). This method owns how those values select the Task and optional Change Unit shown in `task_ref`, `change_unit_ref`, and `state`.

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

The examples are intentionally compact and method-local. The representative response is abbreviated to the fields needed to show the intake branch, refs, state version, lifecycle, current scope, current Change Unit, and next action.

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

Abbreviated result branch (`IntakeResult`, committed):

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
  autonomy_boundary: null
  active_change_unit_ref: null
  baseline_ref: null
  shaping_readiness: null
  pending_user_judgment_refs: []
  blocker_refs: []
  write_authority_summary: null
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display: null
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
