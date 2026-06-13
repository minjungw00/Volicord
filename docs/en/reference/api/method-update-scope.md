<a id="harnessupdate_scope"></a>

# `harness.update_scope` reference

## What this document owns

This document owns baseline method behavior for `harness.update_scope`:

- method-specific required inputs, access requirements, state-version behavior, result branches, and dry-run behavior
- the minimal request and representative response for the shared account data export confirmation scenario
- method-level storage-effect summary and links to storage owners

## What this document does not own

This document does not own:

- common `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, or `ToolDryRunResponse` schema bodies
- nested state, artifact, judgment, value-set, or error schema definitions
- storage DDL, storage record layouts, artifact lifecycle, security guarantees, or Core product meaning

## Purpose

Update active Task and Change Unit fields after intake:

- goal summary
- scope boundary
- non-goals
- acceptance criteria
- autonomy boundary
- baseline reference
- active Change Unit

This method is the supported path that turns shaping into a first safe Change Unit when user-owned blockers have been handled.

## Required inputs

- `ToolEnvelope` with non-null `idempotency_key` and current `expected_state_version` for non-dry-run commits.
- `task_id`.
- Any scope fields to change. For include/exclude updates, `scope_update.include` lists product work to bring into scope and `scope_update.exclude` lists product behavior that remains out of scope. `null` means leave the existing value unchanged; an empty array replaces that list with an empty list.
- `change_unit.operation` and the fields needed by that operation.
- `related_scope_decision_refs` when the update applies a resolved `judgment_kind=scope_decision`.

## Access requirements

Conditions:

- `dry_run=false` commit.
- `VerifiedSurfaceContext.access_class=core_mutation`.
- `verified=true`.
- The request identifies a compatible same-project Task.
- When creating or replacing an active Change Unit, the request provides enough scope to make the next safe action honest.

## State version behavior

Committed non-dry-run result:

- increments `project_state.state_version` exactly once

Core marks an active Write Authorization `status=stale` when its basis no longer matches:

- scope
- baseline
- acceptance criteria
- non-goals
- autonomy boundary
- Change Unit
- project state

Non-claim: `status=stale` does not consume, revoke, expire, or silently reuse the authorization.

## Success result

Returns `UpdateScopeResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `task_ref`
- optional `change_unit_ref`
- linked scope-decision refs
- stale Write Authorization refs
- blocker refs
- current `state`
- `next_actions`

## Blocked result

The method may commit method-owned blocker or current-row updates when scope is still not ready.

A committed blocked scope result must identify the missing user-owned judgment category:

- `product_decision`
- `technical_decision`
- `scope_decision`
- `sensitive_approval`

Non-claim: a blocked scope result must not hide the missing judgment behind vague ambiguity.

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

Public error code meaning is owned by [API error codes](error-codes.md). Public error precedence is owned by [API error precedence](error-precedence.md).

## Dry-run behavior

For `dry_run=true`, a valid state-effecting preview returns `ToolDryRunResponse`.

Branch shape is owned by [API Schema Core](schema-core.md); no-effect persistence semantics are owned by [Storage Effects](../storage-effects.md).

## Storage effect

On commit, the method may persist scope-owned current state and stale-authorization consequences. Exact storage effects are owned by [Storage Effects](../storage-effects.md).

## Minimal valid request

```yaml
method: harness.update_scope
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_scope_001
    idempotency_key: idem_scope_001
    expected_state_version: 18
    dry_run: false
    locale: en-US
  task_id: task_456
  goal_summary: "Add explicit confirmation before account data export."
  scope_update:
    include:
      - "Update the account data export flow to require explicit confirmation before download."
      - "Update account data export confirmation tests."
    exclude:
      - "Account deletion behavior"
  scope_boundary: "Account data export flow and account data export confirmation tests."
  non_goals:
    - "Account deletion behavior"
  acceptance_criteria:
    - "Account data export requires an explicit confirmation step before download."
  autonomy_boundary: "Stay within the account data export flow and account data export confirmation tests."
  baseline_ref: baseline_account_export_001
  change_unit:
    operation: create_active
    scope_summary: "Account data export flow and account data export confirmation tests."
    affected_areas:
      - "Account data export flow"
      - "Account data export confirmation tests"
    affected_paths:
      - src/account/export.ts
      - src/account/export-confirmation.ts
      - tests/account-export.test.ts
    constraints:
      - "Keep account deletion behavior out of scope."
  related_scope_decision_refs: []
```

## Representative response

Result branch (`UpdateScopeResult`, committed):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 19
  events:
    - event_id: evt_1002
      event_kind: scope_updated
task_ref:
  record_kind: task
  record_id: task_456
  project_id: proj_123
  task_id: task_456
  state_version: 19
change_unit_ref:
  record_kind: change_unit
  record_id: cu_001
  project_id: proj_123
  task_id: task_456
  state_version: 19
linked_scope_decision_refs: []
stale_write_authorization_refs: []
blocker_refs: []
state:
  project_id: proj_123
  state_version: 19
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 19
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Add explicit confirmation before account data export."
  scope_summary: "Account data export flow and account data export confirmation tests."
  non_goals:
    - "Account deletion behavior"
  acceptance_criteria:
    - "Account data export requires an explicit confirmation step before download."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_001
    project_id: proj_123
    task_id: task_456
    state_version: 19
next_actions:
  - action: harness.prepare_write
    reason: "Check the account data export change against active scope."
```

## Owner links

- Request envelope and response branches: [API Schema Core](schema-core.md).
- State refs, `StateSummary`, `ShapingReadiness`, blockers, and next actions: [API State Schemas](schema-state.md).
- Scope-related user judgment shapes: [API Judgment Schemas](schema-judgment.md).
- Supported value sets and access classes: [API Value Sets](schema-value-sets.md).
- Public errors: [API error codes](error-codes.md) and [API error precedence](error-precedence.md).
- Persistence effects and stale authorization behavior: [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).
