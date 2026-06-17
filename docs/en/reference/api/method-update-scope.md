<a id="harnessupdate_scope"></a>

# `harness.update_scope` reference

## What this document owns

This document owns baseline method behavior for `harness.update_scope`:

- method-specific required inputs, access requirements, state version behavior, result branches, and `dry_run` behavior
- scope and Change Unit update behavior after intake
- update-scope examples

## What this document does not own

This document does not own:

- common request envelope, response branch, dry-run, or rejected-response schema bodies
- nested state, artifact, judgment, value-set, or error schema definitions
- storage DDL, storage record layouts, exact storage effects, artifact lifecycle, security guarantees, or Core authority semantics
- public error code meaning, public error precedence, or shared response-branch routing

## Purpose

`harness.update_scope` updates current Task and currently applied Change Unit fields after intake:

- goal summary
- scope boundary
- non-goals
- acceptance criteria
- autonomy boundary
- baseline reference
- currently applied Change Unit

This method is the supported path that turns shaping into a first safe Change Unit when user-owned blockers have been handled.

## Required inputs

- A valid `ToolEnvelope`; committed non-dry-run requests require non-null `idempotency_key` and current `expected_state_version`.
- `task_id`.
- Any scope fields to change. For include/exclude updates, `scope_update.include` lists product work to bring into scope and `scope_update.exclude` lists product behavior that remains out of scope. `null` means leave the existing value unchanged; an empty array replaces that list with an empty list.
- `change_unit.operation` and the fields needed by that operation; supported operation values and their meanings are owned by [API Value Sets](schema-value-sets.md#method-local-values).
- `related_scope_decision_refs` when the update applies a resolved `judgment_kind=scope_decision`.

## Request schema

This method owns the top-level `params` request shape below. `envelope` is the shared [`ToolEnvelope`](schema-core.md#tool-envelope); this block does not redefine `ToolEnvelope` fields.

```yaml
UpdateScopeRequest:
  envelope: ToolEnvelope
  task_id: string
  goal_summary: string | null
  scope_update: object | null
  scope_boundary: string | null
  non_goals: string[] | null
  acceptance_criteria: string[] | null
  autonomy_boundary: string | null
  baseline_ref: string | null
  change_unit: object
  related_scope_decision_refs: StateRecordRef[]
```

Nested owner links:
- `related_scope_decision_refs` uses `StateRecordRef[]`; the nested shape is owned by [API State Schemas](schema-state.md#state-references).
- `change_unit.operation` values are owned by [API Value Sets method-local values](schema-value-sets.md#method-local-values).

## Access requirements

A committed non-dry-run request requires:

- `VerifiedSurfaceContext.access_class=core_mutation`
- `verified=true`
- a compatible same-project Task
- enough scope to make the next safe action honest when creating or replacing the currently applied Change Unit

## State version behavior

A committed non-dry-run result increments `project_state.state_version` exactly once.

Core marks a `status=active` `Write Authorization` `status=stale` when its basis no longer matches:

- current scope
- baseline
- acceptance criteria
- non-goals
- autonomy boundary
- currently applied Change Unit
- project state

Non-claim: `status=stale` does not consume, revoke, expire, or silently reuse the authorization.

## Success result

Returns `UpdateScopeResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `task_ref`
- optional `change_unit_ref`
- linked scope-decision refs
- stale `Write Authorization` refs
- blocker refs
- current `state`
- `next_actions`

## Method result fields

`UpdateScopeResult` is the method-specific result branch for a successful committed scope update. It carries `base: ToolResultBase` and these method-owned top-level fields:

| Field | Result-field meaning |
|---|---|
| `base` | Common result metadata. The `ToolResultBase` shape, including `events`, is owned by [API Schema Core](schema-core.md#common-response). `base.events[].event_kind`, when present, is an opaque illustrative classification string. |
| `task_ref` | `StateRecordRef` for the Task updated by the scope result. |
| `change_unit_ref` | `StateRecordRef | null` for the currently applied Change Unit after the operation, or `null` when no current Change Unit applies. |
| `linked_scope_decision_refs` | `StateRecordRef[]` for `scope_decision` user judgments applied by the update. |
| `stale_write_authorization_refs` | `StateRecordRef[]` for `Write Authorization` records made stale by the committed update. Storage effects and versioning own the persistence detail. |
| `blocker_refs` | `StateRecordRef[]` for method-owned blockers committed or still relevant to the update. |
| `state` | Current `StateSummary` after the scope update, including current scope and currently applied Change Unit display fields. |
| `next_actions` | `NextActionSummary[]` describing the next safe API steps. |

The supported `change_unit.operation` values are owned by [API Value Sets](schema-value-sets.md#method-local-values). This method owns how each operation is reflected in `change_unit_ref`, `state.active_change_unit_ref`, stale `Write Authorization` refs, blocker refs, and `next_actions`.

## Blocked result

The method may commit method-owned blocker or current-row updates when scope is still not ready.

A committed blocked scope result must identify the missing user-owned judgment category:

- `product_decision`
- `technical_decision`
- `scope_decision`
- `sensitive_approval`

Not allowed:

- A blocked scope result must not hide the missing judgment behind vague ambiguity.

## Rejected result

Returns `ToolRejectedResponse` for pre-commit failures such as:

- stale `expected_state_version`
- invalid Task identity
- invalid Change Unit operation
- missing required scope
- scope violation
- unresolved required decision
- autonomy-boundary violation
- stale baseline
- local access failure
- validator failure

Public error code meaning, precedence, and rejected-response routing are owned by the error documents linked below.

## Dry-run behavior

For `dry_run=true`, a valid state-effecting preview:

- returns `ToolDryRunResponse`
- creates no scope, Change Unit, blocker, or `Write Authorization` state

## Storage effect

On commit, the method may persist scope-owned current state and stale-authorization consequences. Exact storage effects are owned by the storage documents linked below.

The examples are intentionally compact and method-local. The representative response is abbreviated to the fields needed to show the update-scope branch, refs, state version, current scope, current Change Unit, lifecycle, and next action.

Method-local precondition: `task_filter_001` already exists in `proj_filter_001` at `state_version: 18`, with no suitable current Change Unit. This request creates `cu_filter_001` as the current Change Unit.

## Minimal valid request

```yaml
method: harness.update_scope
params:
  envelope:
    project_id: proj_filter_001
    task_id: task_filter_001
    actor_kind: agent
    surface_id: surface_scope
    request_id: req_scope_filter_001
    idempotency_key: idem_scope_filter_001
    expected_state_version: 18
    dry_run: false
    locale: en-US
  task_id: task_filter_001
  goal_summary: "Limit saved search filters to owner and label fields."
  scope_update:
    include:
      - "Constrain saved-filter edits to owner and label fields."
      - "Update saved-filter validation tests."
    exclude:
      - "Search indexing behavior."
  scope_boundary: "Saved-filter owner and label edits plus related tests."
  non_goals:
    - "Search indexing behavior."
  acceptance_criteria:
    - "Saved filters reject changes outside owner and label fields."
  autonomy_boundary: "Stay within saved-filter edit validation and related tests."
  baseline_ref: baseline_filter_001
  change_unit:
    operation: create_current
    scope_summary: "Saved-filter owner and label edit validation."
    affected_areas:
      - "Saved-filter edit form"
      - "Saved-filter validation tests"
    affected_paths:
      - src/search/saved-filter.ts
      - src/search/filter-form.ts
      - tests/saved-filter.test.ts
    constraints:
      - "Leave search indexing behavior out of scope."
  related_scope_decision_refs: []
```

## Representative response

Abbreviated result branch (`UpdateScopeResult`, committed):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 19
  events:
    - event_id: evt_filter_001
      event_kind: scope_updated
task_ref:
  record_kind: task
  record_id: task_filter_001
  project_id: proj_filter_001
  task_id: task_filter_001
  state_version: 19
change_unit_ref:
  record_kind: change_unit
  record_id: cu_filter_001
  project_id: proj_filter_001
  task_id: task_filter_001
  state_version: 19
linked_scope_decision_refs: []
stale_write_authorization_refs: []
blocker_refs: []
state:
  project_id: proj_filter_001
  state_version: 19
  task_ref:
    record_kind: task
    record_id: task_filter_001
    project_id: proj_filter_001
    task_id: task_filter_001
    state_version: 19
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Limit saved search filters to owner and label fields."
  scope_summary: "Saved-filter owner and label edit validation."
  non_goals:
    - "Search indexing behavior."
  acceptance_criteria:
    - "Saved filters reject changes outside owner and label fields."
  autonomy_boundary: "Stay within saved-filter edit validation and related tests."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_filter_001
    project_id: proj_filter_001
    task_id: task_filter_001
    state_version: 19
  baseline_ref: baseline_filter_001
  shaping_readiness: null
  pending_user_judgment_refs: []
  blocker_refs: []
  write_authority_summary: null
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display: null
next_actions:
  - action_kind: prepare_write
    owner_method: harness.prepare_write
    label: "Check the saved-filter change against current scope."
    blocking_question: null
    required_refs:
      - record_kind: task
        record_id: task_filter_001
        project_id: proj_filter_001
        task_id: task_filter_001
        state_version: 19
      - record_kind: change_unit
        record_id: cu_filter_001
        project_id: proj_filter_001
        task_id: task_filter_001
        state_version: 19
```

## Owner links

- Request envelope and response branches: [API Schema Core](schema-core.md).
- State refs, `StateSummary`, `ShapingReadiness`, blockers, and next actions: [API State Schemas](schema-state.md).
- Scope-related user judgment shapes: [API Judgment Schemas](schema-judgment.md).
- Supported value sets, `change_unit.operation` meanings, and access classes: [API Value Sets](schema-value-sets.md#method-local-values) and [access class values](schema-value-sets.md#access-class-values).
- Public errors, precedence, and rejected-response routing: [API error codes](error-codes.md), [API error precedence](error-precedence.md), and [API error routing](error-routing.md).
- Persistence effects and stale authorization behavior: [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).
