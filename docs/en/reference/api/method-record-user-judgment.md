<a id="volicordrecord_user_judgment"></a>

# `volicord.record_user_judgment` reference

## What this document owns

This document owns baseline method behavior for `volicord.record_user_judgment`:

- method-specific required inputs, access requirements, state version behavior, result branches, and `dry_run` behavior
- recording the user's answer to one existing pending `UserJudgment`
- method-specific boundaries for resolving that pending user-owned judgment and rejecting invalid answer attempts
- record-user-judgment examples

## What this document does not own

This document does not own:

- common request envelope, response branch, dry-run, or rejected-response schema bodies
- `UserJudgment`, `RecordUserJudgmentPayload`, `SensitiveActionScope`, `AcceptedRiskInput`, value-set, or status field definitions
- Core user-owned judgment meaning, final acceptance meaning, residual-risk meaning, sensitive-action approval meaning, or `Write Authorization` meaning
- storage record layouts, exact storage effects, public error code meaning, public error precedence, or shared response-branch routing

## Purpose

`volicord.record_user_judgment` records the user's answer to one existing pending `UserJudgment`.

The method updates the addressed pending judgment according to the user's answer. It does not broaden the answer into unrelated approval, current scope expansion, final acceptance, residual-risk acceptance, sensitive-action approval, or `Write Authorization`.

Before recording the answer, Core checks the pending judgment's `JudgmentBasis` against current state. A stale, superseded, incompatible, or invalid stored basis cannot be answered successfully.

## Required inputs

- A valid `ToolEnvelope`; committed non-dry-run requests require non-null `idempotency_key` and current `expected_state_version`.
- `user_judgment_id` for an existing pending judgment.
- Matching `judgment_kind`.
- `selected_option_id`, `answer`, `note`, and `accepted_risks`.
- An `answer` containing only the decision-specific payload branch for the pending `judgment_kind`.

`selected_option_id` and `note` stay at request level. `RecordUserJudgmentPayload` must not repeat them inside the decision-specific answer branch.

The selected option's stored `machine_action` and `resolution_outcome` are authoritative. If the answer payload contains an outcome, decision, or acceptance field, it must agree with that selected option. Free-form answer text, labels, or notes cannot grant authority.

## Request schema

This method owns the top-level `params` request shape below. `envelope` is the shared [`ToolEnvelope`](schema-core.md#tool-envelope); this block does not redefine `ToolEnvelope` fields.

All fields shown in this method-owned request block are required members of `params` unless a field note explicitly marks a member optional; `T | null` means the member must be present and may contain JSON `null`.

```yaml
RecordUserJudgmentRequest:
  envelope: ToolEnvelope
  user_judgment_id: string
  judgment_kind: string
  selected_option_id: string
  answer: RecordUserJudgmentPayload
  note: string | null
  accepted_risks: AcceptedRiskInput[]
```

Nested owner links:
- `answer` uses `RecordUserJudgmentPayload`; `SensitiveActionScope` may appear only inside that payload branch and is owned by [API Judgment Schemas](schema-judgment.md#resolution-and-answer-payload).
- `accepted_risks` uses `AcceptedRiskInput[]`; the nested shape is owned by [API Judgment Schemas](schema-judgment.md#acceptedriskinput).
- `judgment_kind` values are owned by [API Value Sets judgment values](schema-value-sets.md#judgment-values).

## Access requirements

The method requires:

- server-derived `VerifiedSurfaceContext` with `access_class=core_mutation`
- an addressed pending judgment that belongs to the same project and compatible Task selected by the request

Local access failures, unreadable judgment identity, and insufficient local capability reject before commit.

Authority-bearing resolution additionally requires a derived `VerifiedActorContext.role=user_interaction` for the bound surface instance and `envelope.actor_kind=user`. A surface registered with `interaction_role=agent` cannot satisfy user authority by submitting `actor_kind=user`.

## State version behavior

A committed non-dry-run result:

- increments `project_state.state_version` exactly once
- updates the addressed `user_judgments` row
- does not increment `scope_revision` or `close_basis_revision`
- may update dependent blocker or summary state only as allowed by the storage-effect owner

Non-claims:

- Dry run and rejection create no judgment resolution, blocker update, event, replay row, or state-version increment.
- A recorded `scope_decision` does not silently change current scope or currently applied Change Unit records. Those records still require the scope owner-defined transition, such as `volicord.update_scope`.

Compatibility requirements:

- Final acceptance must match the current Task, Change Unit, `scope_revision`, `close_basis_revision`, baseline, and result refs captured in the judgment basis.
- Residual-risk acceptance must include exact current `risk_id` values in `AcceptedRiskInput` and must match the current `close_basis_revision`.
- Sensitive approval must match current `scope_revision`, Change Unit, operation, normalized paths, sensitive categories, and baseline.
- Scope decision authority for a later scope update requires `judgment_kind=scope_decision`, `status=resolved`, `machine_action=accept`, `resolution_outcome=accepted`, current basis, `required_for` that includes scope update, verified `user_interaction` actor provenance, and compatible Task, Change Unit, `scope_revision`, and affected refs.
- Authority-bearing judgments require `resolved_by_actor_kind=user`, compatible verified actor provenance, `machine_action=accept`, and `resolution_outcome=accepted` to satisfy the authority requirement.
- Rejected or deferred authority-bearing judgments remain decision records but cannot authorize a current transition. Stale, superseded, expired, invalid-basis, provenance-missing, resolution-incomplete, or agent-recorded authority-bearing judgments cannot authorize a current transition.
- Scope or Run changes do not delete historical judgments; they make incompatible judgments ineligible for current close, write, scope-decision, or sensitive-approval requirements.

## Success result

Returns `RecordUserJudgmentResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `user_judgment_ref`
- updated `user_judgment`
- `updated_refs`
- current `state`
- `next_actions`

The method commits the addressed judgment as `status=resolved` when an answer is recorded successfully. The recorded `machine_action` and `resolution_outcome` are copied from the selected option and must match the option's action/outcome mapping.

The result updates only covered blockers and judgment-dependent summaries. It does not create unrelated approvals, evidence, scope updates, `Write Authorization`, close state, final acceptance, residual-risk acceptance, sensitive approval, or cancellation authority beyond an accepted, compatible authority-bearing judgment itself.

## Method result fields

`RecordUserJudgmentResult` is the method-specific result branch for a committed user-judgment answer. It carries `base: ToolResultBase` and these method-owned top-level fields:

| Field | Result-field meaning |
|---|---|
| `base` | Common result metadata. The `ToolResultBase` shape, including `events`, is owned by [API Schema Core](schema-core.md#common-response). Committed `RecordUserJudgmentResult` branches use `base.response_kind=result` and `base.effect_kind=core_committed`. `base.events[].event_kind`, when present, is an opaque illustrative classification string. |
| `user_judgment_ref` | `StateRecordRef` for the addressed `UserJudgment` after the answer is recorded. |
| `user_judgment` | The updated `UserJudgment` with its `resolution` populated when the focused judgment is resolved by the recorded answer. The nested shape is owned by [API Judgment Schemas](schema-judgment.md#userjudgment). |
| `updated_refs` | `StateRecordRef[]` for records updated by recording this judgment answer. |
| `state` | Current `StateSummary` after the judgment answer is recorded. Nested state fields are owned by [API State Schemas](schema-state.md). |
| `next_actions` | `NextActionSummary[]` describing next safe API steps. The canonical shape is owned by [API State Schemas](schema-state.md#current-position-display-shapes). |

`RecordUserJudgmentPayload` stays inside `user_judgment.resolution.answer` and uses the shape owned by [API Judgment Schemas](schema-judgment.md#resolution-and-answer-payload). `next_actions` entries use `action_kind`, `owner_method`, `label`, `blocking_question`, and `required_refs`; stale `action` or `reason` fields are not part of `NextActionSummary`.

## Blocked result

There is no separate committed blocked response branch for this method.

`blocked` is not a committed `JudgmentResolutionOutcome`. An answer payload that explicitly claims a blocked judgment result is rejected before commit.

## Rejected result

Returns `ToolRejectedResponse` for pre-commit failures, including:

- stale `expected_state_version`
- unknown or non-pending judgment
- `judgment_kind` mismatch
- invalid selected option
- invalid answer payload
- expired pending judgment
- stale, superseded, incompatible, or invalid stored judgment basis
- answer incompatible with the pending judgment
- missing or non-current residual-risk `risk_id`
- local access failure
- validator failure

Public error code meaning, precedence, and rejected-response routing are owned by the error documents linked below.

## Dry-run behavior

For `dry_run=true`, a valid preview:

- returns `ToolDryRunResponse`
- does not resolve the judgment
- updates no blockers, events, replay rows, or state version

## Storage effect

On commit, the method may persist judgment resolution and dependent blocker or summary state. Exact storage effects are owned by the storage documents linked below.

## Minimal valid request

Method-local precondition: `uj_empty_001` is an existing pending `product_decision` for `task_empty_001` and `cu_empty_001` in `proj_empty_001`; the current project `state_version` is `62`, and `keep` is one of its option IDs.

```yaml
method: volicord.record_user_judgment
params:
  envelope:
    project_id: proj_empty_001
    task_id: task_empty_001
    actor_kind: user
    surface_id: surface_empty
    request_id: req_empty_answer_001
    idempotency_key: idem_empty_answer_001
    expected_state_version: 62
    dry_run: false
    locale: en-US
  user_judgment_id: uj_empty_001
  judgment_kind: product_decision
  selected_option_id: keep
  answer:
    product_decision:
      judgment:
        decision: accepted
        rationale: "The empty-state illustration is suitable for this Task."
    technical_decision: null
    scope_decision: null
    sensitive_action_scope: null
    final_acceptance: null
    residual_risk_acceptance: null
    cancellation: null
  note: null
  accepted_risks: []
```

## Representative response

Abbreviated result branch (`RecordUserJudgmentResult`, committed):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 63
  events:
    - event_id: evt_empty_001
      event_kind: user_judgment_recorded
user_judgment_ref:
  record_kind: user_judgment
  record_id: uj_empty_001
  project_id: proj_empty_001
  task_id: task_empty_001
  state_version: 63
user_judgment:
  judgment_id: uj_empty_001
  project_id: proj_empty_001
  task_id: task_empty_001
  change_unit_id: cu_empty_001
  judgment_kind: product_decision
  status: resolved
  presentation: short
  question: "Should the empty-state illustration be kept?"
  options:
    - option_id: keep
      label: "Keep illustration"
      description: "Record the user-owned product decision to keep the illustration."
      consequence: "If selected, Core records the keep-illustration product decision."
      machine_action: accept
      resolution_outcome: accepted
      is_default: true
    - option_id: replace
      label: "Replace illustration"
      description: "Record that the illustration should be replaced."
      consequence: "If selected, Core records the replace-illustration product decision."
      machine_action: accept
      resolution_outcome: accepted
      is_default: false
  context:
    summary: "The empty-state screen has a proposed illustration and needs a user-owned product decision."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "Only the empty-state illustration choice is covered by this judgment."
  affected_refs:
    - record_kind: task
      record_id: task_empty_001
      project_id: proj_empty_001
      task_id: task_empty_001
      state_version: 62
  basis:
    task_id: task_empty_001
    change_unit_id: cu_empty_001
    scope_revision: 1
    close_basis_revision: null
    baseline_ref: baseline_empty_001
    result_refs: []
    residual_risk_ids: []
    sensitive_action_scope: null
    created_at_state_version: 62
    compatibility_status: current
  required_for:
    - close_complete
  resolution:
    selected_option_id: keep
    machine_action: accept
    resolution_outcome: accepted
    answer:
      product_decision:
        judgment:
          decision: accepted
          rationale: "The empty-state illustration is suitable for this Task."
      technical_decision: null
      scope_decision: null
      sensitive_action_scope: null
      final_acceptance: null
      residual_risk_acceptance: null
      cancellation: null
    note: null
    accepted_risks: []
    resolved_by_actor_kind: user
  expires_at: null
  created_at: "<example-created-at>"
  resolved_at: "<example-resolved-at>"
updated_refs:
  - record_kind: user_judgment
    record_id: uj_empty_001
    project_id: proj_empty_001
    task_id: task_empty_001
    state_version: 63
state:
  project_id: proj_empty_001
  state_version: 63
  task_ref:
    record_kind: task
    record_id: task_empty_001
    project_id: proj_empty_001
    task_id: task_empty_001
    state_version: 62
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Decide empty-state illustration."
  scope_summary: "Empty-state illustration decision."
  non_goals:
    - "Changing empty-state copy."
  acceptance_criteria:
    - "The empty-state illustration follows the user's product decision."
  autonomy_boundary: "Stay within empty-state illustration choice."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_empty_001
    project_id: proj_empty_001
    task_id: task_empty_001
    state_version: 62
  baseline_ref: baseline_empty_001
  shaping_readiness: null
  pending_user_judgment_refs: []
  blocker_refs: []
  write_authority_summary: null
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display: null
next_actions:
  - action_kind: close_task
    owner_method: volicord.close_task
    label: "Evaluate close readiness after recording the user's product decision."
    blocking_question: null
    required_refs:
      - record_kind: user_judgment
        record_id: uj_empty_001
        project_id: proj_empty_001
        task_id: task_empty_001
        state_version: 63
```

## Owner links

- Request envelope, response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `UserJudgment`, `RecordUserJudgmentPayload`, `SensitiveActionScope`, and `AcceptedRiskInput`: [API Judgment Schemas](schema-judgment.md).
- State refs and summaries: [API State Schemas](schema-state.md).
- Judgment values and supported method-local values: [API Value Sets](schema-value-sets.md).
- User-owned judgment, final acceptance, residual-risk acceptance, and non-substitution rules: [Core Model](../core-model.md).
- Exact storage effects: [Storage Effects](../storage-effects.md#volicordrecord_user_judgment).
- Public errors, precedence, and rejected-response routing: [API error codes](error-codes.md), [API error precedence](error-precedence.md), and [API error routing](error-routing.md).
- Creating the pending judgment request: [`volicord.request_user_judgment`](method-request-user-judgment.md).
