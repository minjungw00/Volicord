<a id="volicordupdate_scope"></a>

# `volicord.update_scope` reference

## What this document owns

This document owns baseline method behavior for `volicord.update_scope`:

- method-specific required inputs, access requirements, state version behavior, result branches, and `dry_run` behavior
- scope and Change Unit update behavior after `volicord.intake`
- update-scope examples

## What this document does not own

This document does not own:

- common request envelope, response branch, `dry_run`, or rejected-response schema bodies
- nested state, artifact, judgment, value-set, or error schema definitions
- storage DDL, storage record layouts, exact storage effects, artifact lifecycle, security guarantees, or Core authority semantics
- public error code meaning, public error precedence, or shared response-branch routing

## Purpose

`volicord.update_scope` updates current `Task` and currently applied Change Unit fields after `volicord.intake`:

- goal summary
- scope boundary
- non-goals
- acceptance criteria
- autonomy boundary
- baseline reference
- currently applied Change Unit

This method records a Change Unit as the current work boundary. It never changes `work_phase`. A `work` Task in shaping therefore remains in shaping after `create_current` or `replace_current`; only `volicord.advance_task` enters implementation.

For `direct` and `work` Tasks, a committed `create_current` or
`replace_current` operation binds
the new Change Unit's baseline to the verified current workspace context. On a
Git-backed Product Repository that context contains the common Git directory,
exact worktree identity, branch or detached-HEAD state, HEAD SHA, and workspace
fingerprint. Replacing the Change Unit with a current baseline is the explicit
retarget or rebaseline path after those coordinates change. `keep_current`
never silently retargets an existing Change Unit. When a current Change Unit
exists, `keep_current` rejects a Task `baseline_ref` change; the caller must use
`replace_current` so Task and Change Unit baselines change atomically.

For an `advisor` Task, `keep_current`, `create_current`, and `replace_current`
all require the canonical non-write Change Unit predicate. The Change Unit has
no affected or allowed paths, its effect contract allows only
`artifact_registration`, `user_action_request`, and `evidence_update`, has no
sensitive expectation, and explicitly forbids `product_file_write`,
`run_recording`, `sensitive_action`, `external_network`, and `secret_access`.
Core rejects an update that would create or retain any write-capable or
otherwise incompatible advisor Change Unit before committing scope effects.

The current MCP action form fixes the Task, complete scope-owned
`related_scope_decision_refs`, and the exact `change_unit.operation` selected
from current Change Unit authority. The request's `baseline_ref` is the
Agent-authored next baseline, not a copy of the current nullable baseline.
Current baseline and scope revision remain Core-current authority covered by
the admitted form and expected state version. Scope content and the
operation-specific Change Unit fields are also Agent-authored slots. The
adapter injects project and expected state version; the generic binder rejects
any altered or omitted caller-visible fixed value before Core.

Core publishes the closed action variants `keep_current_change_unit`,
`create_current_change_unit`, and `replace_current_change_unit`; each maps
exactly to the correspondingly named `ChangeUnitOperation`. With no current
Change Unit, only create is current. With a current Change Unit, keep and a
Core-policy-compatible replace are current, and create is not. An
implementation replacement that would stale current shaping applications is
not published. Keep fixes `keep_current`; create and replace fix their
operation and require Agent-authored `scope_summary`, `affected_paths`, and
next baseline while retaining optional Change Unit fields and effect contract.

## Required inputs

- A valid `ToolEnvelope`; committed `dry_run=false` requests require non-null `idempotency_key` and current `expected_state_version`.
- `task_id`.
- Any scope fields to change. For include/exclude updates, `scope_update.include` lists product work to bring into scope and `scope_update.exclude` lists product behavior that remains out of scope. `null` means leave the existing value unchanged; an empty array replaces that list with an empty list.
- `acceptance_criteria=null` leaves the canonical criterion set unchanged. A
  non-null array is a complete replacement set: a current ID from the same `Task` preserves
  that criterion identity and may update its statement or
  `evidence_requirement`; this is an update to the same criterion, not a new
  identity. A null ID requests a new Core-generated ID, and an
  omitted current criterion is retired. Unknown, retired, cross-Task, and
  duplicate IDs reject before commit.
- `change_unit.operation` and the fields needed by that operation; supported operation values and their meanings are owned by [API Value Sets](schema-value-sets.md#method-local-values).
- Optional `change_unit.effect_contract` when creating or replacing the current Change Unit. When present, the object uses `ChangeUnitEffectContract`; when absent, the Change Unit has no extra effect contract.
- `related_scope_decision_refs` must exactly cover the current checkpoint's
  accepted scope-owned gaps. It is empty when there is no such gap, including
  product-only and technical-only progression.

When a scope update applies a `scope_decision`, each reference must identify the
exact resolution linked to an accepted scope gap in the current checkpoint and
must have `judgment_kind=scope_decision`, `status=resolved`,
`machine_action=accept`, `resolution_outcome=accepted`,
`resolved_by_actor_source=local_user`, compatible User Channel provenance,
`basis.coordinates.compatibility_status=current`, exact
`required_for=[scope_update]`, and a basis compatible with the current Task,
Change Unit, checkpoint, `scope_revision`, baseline, request, and affected refs.
Product, technical, and sensitive resolutions are not accepted in this field.

Before applying any scope or Change Unit effect, Core rejects with
`DECISION_UNRESOLVED` when a current pending user-action request includes
`scope_update` in `required_for` and its action kind, Task, current Change Unit,
`scope_revision`, basis, and affected refs match this operation. Informational,
resolved, stale, superseded, expired, non-matching, and action-kind-incompatible
requests do not block the update.

## Request schema

This method owns the top-level `params` request fields in the generated table
below. `envelope` is the shared [`ToolEnvelope`](schema-core.md#tool-envelope);
the table does not redefine `ToolEnvelope` fields. Requiredness and nullability
come directly from the semantic request descriptor.

<!-- BEGIN GENERATED: contract-structures api.method.update_scope.request[params] -->
<!-- Generated by `cargo run -p xtask -- docs-sync`; do not edit this region. -->

### `UpdateScopeRequest` fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `acceptance_criteria` | yes | yes | `AcceptanceCriterionReplacement[]` |
| `autonomy_boundary` | yes | yes | `string` |
| `baseline_ref` | yes | yes | `BaselineRef` |
| `change_unit` | yes | no | `ChangeUnitUpdate` |
| `envelope` | yes | no | `ToolEnvelope` |
| `goal_summary` | yes | yes | `string` |
| `non_goals` | yes | yes | `string[]` |
| `related_scope_decision_refs` | yes | no | `StateRecordRef[]` |
| `scope_boundary` | yes | yes | `string` |
| `scope_update` | yes | yes | `ScopeUpdate` |
| `task_id` | yes | no | `TaskId` |
<!-- END GENERATED: contract-structures api.method.update_scope.request[params] -->



Nested owner links:
- `acceptance_criteria` uses `AcceptanceCriterionReplacement[]`; the nested
  shape is owned by [API State Schemas](schema-state.md#evidence-and-run-snapshot-shapes).
- `related_scope_decision_refs` uses `StateRecordRef[]`; the nested shape is owned by [API State Schemas](schema-state.md#state-references).
- `change_unit.operation` values are owned by [API Value Sets method-local values](schema-value-sets.md#method-local-values).
- `change_unit.effect_contract`, when present, uses `ChangeUnitEffectContract`; the nested shape is owned by [API State Schemas](schema-state.md#changeuniteffectcontract).

## Access requirements

A committed `dry_run=false` request requires:

- verified invocation context with `operation_category=agent_workflow`
- a verified current workspace context when the Product Repository is Git-backed
- a compatible same-project `Task`
- enough scope to make the next safe action honest when creating or replacing the currently applied Change Unit
- for `Task.mode=advisor`, a current or proposed Change Unit satisfying the
  canonical non-write advisor predicate

## State version behavior

A committed `dry_run=false` result increments `project_state.state_version` exactly once.

Before committing, Core reevaluates the `Task`'s effective control level against
the authoritative project policy and the proposed scope/effect contract. It may
raise the level, including to `sensitive`; it never automatically lowers an
active Task. A policy relaxation therefore does not change an active Task, while
a strengthened policy or newly visible sensitive effect is reflected in the
committed `StateSummary`.

A material criterion statement or `evidence_requirement` update increments the
Task scope revision. Evidence coverage recorded against the earlier scope is
projected as `stale`, even when the retained `AcceptanceCriterionId` is
unchanged; current target identity does not make earlier-scope evidence current.

Core invalidates a `status=active` write ticket when the committed update changes
one of its state-bound validity coordinates:

- `scope_revision`
- baseline
- currently applied Change Unit
- recorded workspace binding for the currently applied Change Unit

The stored invalidation reason is respectively `scope_revision_changed`,
`baseline_changed`, `change_unit_changed`, or `workspace_changed`. A normalized
no-op, acceptance/non-goal/autonomy edit that does not change `scope_revision`,
or the unrelated `state_version` increment does not invalidate the ticket.
Invalidation does not consume or silently reuse it.

Applying a scope decision creates its deterministic
`ShapingDecisionApplication`, binds it to the resulting scope revision,
baseline, and Change Unit, links it to the current checkpoint, changes only its
selected gap from `accepted` to `applied`, and increments the scope revision in
one transaction. The result and event include the exact application refs. A compatible no-op or Change Unit
creation can preserve and rebase the current checkpoint without a scope
decision ref. A transition supersedes the checkpoint only when it genuinely
invalidates the checkpoint's scope or baseline authority basis. A scope,
baseline, or incompatible Change Unit change explicitly marks affected current
applications `stale`; row absence is not invalidation.
During `work/implementation`, a scope, baseline, or Change Unit update that
would make any current shaping application stale is rejected before mutation.
The typed no-effect recovery names the affected application refs and requires
the Task to leave implementation through its owned close/supersede transition;
the method never silently returns the Task to shaping.

A rejected, deferred, expired, or inconsistent shaping decision grants no
scope authority. The method returns a no-effect workflow rejection whose
current workflow is `decision_recovery_required` and whose recovery owner is
`volicord.record_shaping_checkpoint`; a semantic no-op scope request cannot bypass this
gate.

## Success result

The committed `UpdateScopeResult` uses `base.response_kind=result` and
`base.effect_kind=core_committed`.

## Method result fields

`UpdateScopeResult` is the method-specific result branch for a successful committed scope update. It carries `base: UpdateScopeResultBase`, whose only result effect is `core_committed`, and these method-owned top-level fields:

<!-- BEGIN GENERATED: contract-structures api.method.update_scope.response[response_variants] api.method.update_scope.response[result_body] api.method.update_scope.response[result_metadata] api.method.update_scope.response[rejection] api.method.update_scope.response[dry_run] -->
<!-- Generated by `cargo run -p xtask -- docs-sync`; do not edit this region. -->

### `UpdateScopeResult` success fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `applied_scope_decision_refs` | yes | no | `StateRecordRef[]` |
| `applied_shaping_decision_application_refs` | yes | no | `StateRecordRef[]` |
| `applied_shaping_gap_refs` | yes | no | `StateRecordRef[]` |
| `base` | yes | no | `UpdateScopeResultBase` |
| `blocker_refs` | yes | no | `StateRecordRef[]` |
| `change_unit_ref` | no | yes | `StateRecordRef` |
| `stale_write_ticket_refs` | yes | no | `StateRecordRef[]` |
| `state` | yes | no | `StateSummary` |
| `task_ref` | yes | no | `StateRecordRef` |

### `Result Metadata: core_committed` fields

Contract: `dry_run` is `false`; `events` contains at least one event (`minItems: 1`).

| Field | Required | Nullable | Type |
|---|---|---|---|
| `disclosure` | yes | no | `GuaranteeDisclosure` |
| `dry_run` | yes | no | `boolean enum(false)` |
| `effect_kind` | yes | no | `string enum("core_committed")` |
| `events` | yes | no | `NonEmptyEventRefs` |
| `response_kind` | yes | no | `string enum("result")` |
| `state_version` | yes | no | `integer` |

### `dry_run` request policy

- `volicord.update_scope`: `dry_run=true` selects the `ToolDryRunResponse` preview branch, whose `base.dry_run` is `true`. `dry_run=false` or an omitted `dry_run` does not select a preview branch.


### Shared response structures

The response descriptor defines success, rejection, and preview as an exact `anyOf` branch union. The rejection branch uses the generated [`ToolRejectedResponse`](schema-core.md#common-response) structure. When method behavior selects a preview branch, it uses the generated [`ToolDryRunResponse`](schema-core.md#common-response) structure. Shared rejection and preview fields remain distinct from the success fields above.
<!-- END GENERATED: contract-structures api.method.update_scope.response[response_variants] api.method.update_scope.response[result_body] api.method.update_scope.response[result_metadata] api.method.update_scope.response[rejection] api.method.update_scope.response[dry_run] -->

The supported `change_unit.operation` values are owned by [API Value Sets](schema-value-sets.md#method-local-values). This method owns how each operation is reflected in `change_unit_ref`, `state.active_change_unit_ref`, stale write-ticket refs, blocker refs, and the tagged `state.workflow` projection.

When `change_unit.operation=create_current` or `change_unit.operation=replace_current`, `change_unit.effect_contract` may be recorded on the new current Change Unit. The effect contract is optional Core state. It can express allowed effects, forbidden effects, allowed Product Repository paths, expected outputs, invariants, evidence expectations, and sensitive-action expectations without creating a workflow engine or replacing user-owned authority records. The same operation records the verified workspace coordinate used by later write preparation. A non-Git repository records no VCS binding and does not receive Git-specific comparison checks.

`applied_shaping_gap_refs` and `applied_scope_decision_refs` identify only the
exact scope gaps and resolutions applied by this call. Product, technical, and
sensitive gaps remain unchanged for their own application owner.

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
- invalid `Task` identity
- invalid Change Unit operation
- missing required scope
- scope violation
- unresolved required decision
- autonomy-boundary violation
- stale baseline
- keep-current baseline retargeting, returned as a typed no-effect
  `AuthorityBasisMismatch` whose retry action requires `replace_current`
- actor-source or operation-category mismatch
- validator failure

Public error code meaning, precedence, and rejected-response routing are owned by the error documents linked below.

## `dry_run` behavior

For `dry_run=true`, a valid state-effecting preview:

- returns `ToolDryRunResponse`
- creates no scope, Change Unit, blocker, or write-ticket state

## Storage effect

On commit, the method may persist scope-owned current state and stale write-ticket consequences. Exact storage effects are owned by the storage documents linked below.

The examples are intentionally compact and method-local. The representative response is abbreviated to the fields needed to show the update-scope branch, refs, state version, current scope, current Change Unit, lifecycle, and next action.

Method-local precondition: `task_filter_001` already exists in `proj_filter_001` at `state_version: 18`, with no suitable current Change Unit. This request creates `cu_filter_001` as the current Change Unit.

## Minimal valid request

```yaml contract=api.method.update_scope.request shape=complete_request
method: volicord.update_scope
params:
  envelope:
    project_id: proj_filter_001
    task_id: task_filter_001
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
    - acceptance_criterion_id: null
      statement: "Saved filters reject changes outside owner and label fields."
      evidence_requirement: required
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

```schema
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
  produced_at_state_version: 19
change_unit_ref:
  record_kind: change_unit
  record_id: cu_filter_001
  project_id: proj_filter_001
  task_id: task_filter_001
  produced_at_state_version: 19
applied_shaping_gap_refs: []
applied_scope_decision_refs: []
applied_shaping_decision_application_refs: []
stale_write_ticket_refs: []
blocker_refs: []
state:
  project_id: proj_filter_001
  state_version: 19
  task_ref:
    record_kind: task
    record_id: task_filter_001
    project_id: proj_filter_001
    task_id: task_filter_001
    produced_at_state_version: 19
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  work_phase: shaping
  goal_summary: "Limit saved search filters to owner and label fields."
  scope_summary: "Saved-filter owner and label edit validation."
  non_goals:
    - "Search indexing behavior."
  acceptance_criteria:
    - acceptance_criterion_id: criterion_filter_001
      statement: "Saved filters reject changes outside owner and label fields."
      evidence_requirement: required
  autonomy_boundary: "Stay within saved-filter edit validation and related tests."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_filter_001
    project_id: proj_filter_001
    task_id: task_filter_001
    produced_at_state_version: 19
  baseline_ref: baseline_filter_001
  workspace_context:
    vcs: git
    git_common_dir: "/work/search/.git"
    worktree_id: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
    branch_ref: "refs/heads/filter-scope"
    head_sha: "0123456789abcdef0123456789abcdef01234567"
    workspace_fingerprint: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
  # The current complete WorkflowProjection is omitted from this abbreviated example.
  pending_user_action_summaries: []
  blocker_refs: []
  write_ticket_summary: null
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display: null
```

## Owner links

- Request envelope and response branches: [API Schema Core](schema-core.md).
- State refs, `StateSummary`, tagged workflow progression, and blockers: [API State Schemas](schema-state.md).
- Scope-related user judgment shapes: [API Judgment Schemas](schema-judgment.md).
- Supported value sets, `change_unit.operation` meanings, and operation categories: [API Value Sets](schema-value-sets.md#operation-category-values).
- Public errors, precedence, and rejected-response routing: [API error codes](error-codes.md), [API error precedence](error-precedence.md), and [API error routing](error-routing.md).
- Persistence effects and stale write-ticket behavior: [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).
