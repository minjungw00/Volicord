<a id="volicordintake"></a>

# `volicord.intake` reference

## What this document owns

This document owns baseline method behavior for `volicord.intake`:

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

`volicord.intake` starts, resumes, supersedes, or rejects an ordinary user work loop.

The method resolves the requested mode to a concrete Task mode:

- `advisor`
- `direct`
- `work`

Scope boundary:

- `volicord.intake` may create the first scope candidate for write-capable work.
- Later scope changes belong to `volicord.update_scope`.

Task-granularity guidance:

- `plain_language_request` and the initial scope preserve the user's requested
  outcome, rather than reducing it to the next analysis technique or other
  intermediate step.
- When analysis or shaping is one phase of a requested implementation outcome,
  the caller selects a `work` Task and records that phase later as a
  `shaping_update`; it does not create an isolated `advisor` Task merely because
  analysis happens first.
- The caller selects `advisor` when the requested outcome itself is read-only
  advice. When a broader outcome is unclear, it keeps only the known boundary
  in shaping state or asks the user; it does not infer a larger goal.

Core makes that distinction durable. A newly created Task records a
`work_phase` separately from the Task lifecycle: `advisor` and `work` begin in
`shaping`, while `direct` begins in `implementation`. Creating or replacing the
current Change Unit advances a `work` Task to `implementation`. Resuming a Task
keeps its recorded phase. The phase constrains Run-kind and write-ticket
compatibility; it is not a second Task, a methodology engine, or a substitute
for current scope.

Task creation can also record one predecessor relation. The relation preserves
why the new Task exists and which predecessor material was selected for
carry-forward; it does not make predecessor authority current. Status can use
the stored predecessor edges to show the connected Task flow.

## Required inputs

- A valid `ToolEnvelope`; committed non-dry-run requests require non-null `idempotency_key` and current `expected_state_version`.
- `plain_language_request`, `requested_mode`, and `resume_policy`.
- `acceptance_policy`, present as `required`, `not_required`,
  `policy_dependent`, or JSON `null`. `null` asks Core to select the mode
  default and still records the selected policy and its reason on the Task.
- `lineage`, present as `TaskLineageInput` or JSON `null`. A resumed Task uses
  `null`; a newly created follow-up may name exactly one predecessor.
- Any known initial scope candidate in `initial_scope.boundary`,
  `initial_scope.non_goals`, and `initial_scope.acceptance_criteria`; use empty
  arrays when no list items are known. Each criterion input supplies a statement
  and evidence requirement and never supplies an ID. Core generates each
  `AcceptanceCriterionId` on commit.

## Request schema

This method owns the top-level `params` request shape below. `envelope` is the shared [`ToolEnvelope`](schema-core.md#tool-envelope); this block does not redefine `ToolEnvelope` fields.

All fields shown in this method-owned request block are required members of `params` unless a field note explicitly marks a member optional; `T | null` means the member must be present and may contain JSON `null`.

```yaml
IntakeRequest:
  envelope: ToolEnvelope
  plain_language_request: string
  requested_mode: string
  resume_policy: string
  acceptance_policy: string | null
  lineage: TaskLineageInput | null
  initial_scope: object
  initial_context_refs: StateRecordRef[]
  initial_source_refs: SourceRef[]

TaskLineageInput:
  predecessor_task_id: string
  relation: string
  creation_reason: string
  carry_forward: string[]
```

Nested owner links:
- `initial_scope.acceptance_criteria` uses `AcceptanceCriterionInput[]`; the
  nested shape is owned by [API State Schemas](schema-state.md#evidence-and-run-snapshot-shapes).
- `initial_context_refs` uses `StateRecordRef[]`; the nested shape is owned by [API State Schemas](schema-state.md#state-references).
- `initial_source_refs` uses the non-authoritative `SourceRef[]` shape owned by [API State Schemas](schema-state.md#non-authoritative-source-references). Core structurally validates and stores these refs as Task context; it does not inspect their content or use them to expand scope, select a baseline, establish evidence, or create authority.
- `requested_mode` and `resume_policy` values are owned by [API Value Sets](schema-value-sets.md#task-lifecycle-values) and [method-local values](schema-value-sets.md#method-local-values).
- `acceptance_policy`, `lineage.relation`, and `lineage.carry_forward` values
  are owned by [API Value Sets](schema-value-sets.md#task-lifecycle-values).

Acceptance-policy rules:

- `acceptance_policy=null` selects `not_required` for `advisor` and `required`
  for `direct` or `work`.
- `resume_policy=resume_active` requires `acceptance_policy=null` and preserves
  the selected Task's stored policy and reason; intake does not mutate them on
  resume.
- `not_required` is valid only for an `advisor` Task at intake. An Agent
  Connection cannot waive final acceptance for write-capable work by selecting
  that value.
- `policy_dependent` records that Core must evaluate the owner-defined close
  policy from the current result and risk basis. It is not agent discretion at
  close time.
- The committed `StateSummary` exposes both the selected policy and a
  Core-generated reason. A policy never substitutes for evidence,
  residual-risk acceptance, sensitive-action approval, or another blocker.

Lineage and carry-forward rules:

- `lineage` is accepted only when intake creates a new Task. Its predecessor
  must be an existing different Task in the same project, and
  `creation_reason` must be non-empty.
- Relations are `continues`, `derived_from`, `split_from`, `replaces`, and
  `implements_advice_from`. `implements_advice_from` requires a completed
  `advisor` predecessor with `result=advice_only`.
- `carry_forward` is an explicit duplicate-free selection from `scope`,
  `non_goals`, `user_decisions`, `source_refs`, `context_refs`,
  `known_limitations`, `unresolved_obligations`, `residual_risks`, and
  `baseline`.
- An explicitly selected category must identify predecessor material that
  exists and can be carried or referenced. Intake rejects empty scope,
  non-goal, source-ref, context-ref, baseline, or reference-only selections
  instead of recording a misleading `applied` or `reference_only` disposition.
- Selected scope and non-goal material becomes new-Task input only when it is
  compatible with the submitted initial scope. Criterion statements and
  evidence requirements may be copied as scope material, but the new Task gets
  new `AcceptanceCriterionId` values.
- Selected source and context refs remain non-authoritative context. A
  `source_refs` selection copies only refs actually stored on the predecessor;
  an artifact-bearing ref remains predecessor-Task-scoped and Core revalidates
  that exact artifact ownership and integrity instead of relabeling it as a
  new-Task artifact.
- Selected user decisions, known limitations, obligations, and residual risks
  produce reference-only dispositions to exact active predecessor continuity
  records or compatible current close-basis Run/risk refs. Intake rejects a
  selected category with no active compatible record; it does not fabricate a
  disposition that merely points at the predecessor Task. A new-Task owner
  check is still required before any referenced fact can satisfy scope,
  acceptance, risk, or write rules.
- A selected baseline is copied only when the predecessor baseline exists and
  the predecessor Task baseline exactly equals its current Change Unit write
  basis, while the recorded Git worktree, branch or detached-HEAD state, HEAD
  SHA, and workspace fingerprint remain compatible. Otherwise intake rejects
  that carry-forward and requires an explicit rebaseline; it does not copy
  stale baseline authority.
- Core records one carry-forward disposition per selected category so status
  can distinguish applied material from reference-only context.

## Access requirements

A committed non-dry-run request requires:

- verified invocation context with `operation_category=agent_workflow`

Invocation boundary:

- `actor_source` and `operation_category` are derived from the verified local invocation context; callers do not submit them as authority claims.

## State version behavior

A committed non-dry-run result:

- increments project-wide `project_state.state_version` exactly once
- creates the replay row for the idempotency key

The following create no Task, Change Unit, event, replay row, blocker update, or state-version increment:

- dry run
- read failure
- validation failure
- actor-source or operation-category mismatch
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
| `state` | Current `StateSummary` after intake, including current scope, currently applied Change Unit display fields, and any current Change Unit effect contract. |
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
- unavailable Core or invocation context
- actor-source or operation-category mismatch
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
method: volicord.intake
params:
  envelope:
    project_id: proj_onboard_001
    task_id: null
    request_id: req_intake_onboard_001
    idempotency_key: idem_intake_onboard_001
    expected_state_version: 17
    dry_run: false
    locale: en-US
  plain_language_request: "Create a first-run checklist for new workspace setup."
  requested_mode: work
  resume_policy: create_new
  acceptance_policy: null
  lineage: null
  initial_scope:
    boundary: "First-run checklist for new workspace setup."
    non_goals:
      - "Changing account creation."
    acceptance_criteria:
      - statement: "New users see the checklist after opening a workspace."
        evidence_requirement: required
  initial_context_refs: []
  initial_source_refs: []
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
  produced_at_state_version: 18
change_unit_ref: null
state:
  project_id: proj_onboard_001
  state_version: 18
  task_ref:
    record_kind: task
    record_id: task_onboard_001
    project_id: proj_onboard_001
    task_id: task_onboard_001
    produced_at_state_version: 18
  mode: work
  work_phase: shaping
  acceptance_policy: required
  acceptance_policy_reason: "Write-capable work requires final acceptance."
  lineage: null
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
    - acceptance_criterion_id: criterion_onboard_001
      statement: "New users see the checklist after opening a workspace."
      evidence_requirement: required
  autonomy_boundary: null
  active_change_unit_ref: null
  baseline_ref: null
  shaping_readiness: null
  pending_user_action_summaries: []
  blocker_refs: []
  write_ticket_summary: null
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display: null
next_actions:
  - presentation_role: primary
    action_kind: update_scope
    owner_method: volicord.update_scope
    allowed_operation_categories: [agent_workflow]
    label: "Create the first currently applied Change Unit before write-ticket preparation."
    blocking_question: null
    expected_state_version: 18
    required_refs:
      - record_kind: task
        record_id: task_onboard_001
        project_id: proj_onboard_001
        task_id: task_onboard_001
        produced_at_state_version: 18
```

## Owner links

- Request envelope and response branches: [`ToolEnvelope`](schema-core.md#tool-envelope) and [common response branches](schema-core.md#common-response).
- State refs, `StateSummary`, `ShapingReadiness`, and next actions: [API State Schemas](schema-state.md).
- Supported method names, mode values, `resume_policy`, `response_kind`, `effect_kind`, and operation categories: [API Value Sets](schema-value-sets.md#operation-category-values).
- Public errors, precedence, and rejected-response routing: [API error codes](error-codes.md), [API error precedence](error-precedence.md), and [API error routing](error-routing.md).
- Persistence effects and storage records: [Storage Effects](../storage-effects.md), [Storage Records](../storage-records.md), and [Storage Versioning](../storage-versioning.md).
