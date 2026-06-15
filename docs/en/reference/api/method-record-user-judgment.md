<a id="harnessrecord_user_judgment"></a>

# `harness.record_user_judgment` reference

## What this document owns

This document owns baseline method behavior for `harness.record_user_judgment`:

- method-specific required inputs, access requirements, state version behavior, result branches, and `dry_run` behavior
- recording the user's answer to one existing pending `UserJudgment`
- method-specific boundaries for resolving, rejecting, deferring, blocking, or marking that pending user-owned judgment
- record-user-judgment examples

## What this document does not own

This document does not own:

- common request envelope, response branch, dry-run, or rejected-response schema bodies
- `UserJudgment`, `RecordUserJudgmentPayload`, `SensitiveActionScope`, `AcceptedRiskInput`, value-set, or status field definitions
- Core user-owned judgment meaning, final acceptance meaning, residual-risk meaning, sensitive-action approval meaning, or `Write Authorization` meaning
- storage record layouts, exact storage effects, public error code meaning, public error precedence, or shared response-branch routing

## Purpose

`harness.record_user_judgment` records the user's answer to one existing pending `UserJudgment`.

The method updates the addressed pending judgment according to the user's answer. It does not broaden the answer into unrelated approval, current scope expansion, final acceptance, residual-risk acceptance, sensitive-action approval, or `Write Authorization`.

## Required inputs

- A valid `ToolEnvelope`; committed non-dry-run requests require non-null `idempotency_key` and current `expected_state_version`.
- `user_judgment_id` for an existing pending judgment.
- Matching `judgment_kind`.
- `selected_option_id`, `answer`, `note`, and `accepted_risks`.
- An `answer` containing only the decision-specific payload branch for the pending `judgment_kind`.

`selected_option_id` and `note` stay at request level. `RecordUserJudgmentPayload` must not repeat them inside the decision-specific answer branch.

## Access requirements

The method requires:

- `VerifiedSurfaceContext.access_class=core_mutation`
- `verified=true`
- an addressed pending judgment that belongs to the same project and compatible Task selected by the request

Local access failures, unreadable judgment identity, and insufficient local capability reject before commit.

## State version behavior

A committed non-dry-run result:

- increments `project_state.state_version` exactly once
- updates the addressed `user_judgments` row
- may update dependent blocker or summary state only as allowed by the storage-effect owner

Non-claims:

- Dry run and rejection create no judgment resolution, blocker update, event, replay row, or state-version increment.
- A recorded `scope_decision` does not silently change current scope or currently applied Change Unit records. Those records still require the scope owner-defined transition, such as `harness.update_scope`.

## Success result

Returns `RecordUserJudgmentResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `user_judgment_ref`
- updated `user_judgment`
- `updated_refs`
- current `state`
- `next_actions`

The method may commit the addressed judgment as `resolved`, `rejected`, `deferred`, `blocked`, or another supported judgment status when that status is the user's answer or the compatible result of the focused judgment.

The result updates only covered blockers and judgment-dependent summaries. It does not create unrelated approvals, evidence, scope updates, `Write Authorization`, close state, or residual-risk acceptance beyond the recorded judgment itself.

## Blocked result

There is no separate committed blocked response branch for this method.

A committed `user_judgment.status=blocked` is a recorded judgment outcome, not `ToolRejectedResponse` and not a `PrepareWriteResult`-style blocked decision.

## Rejected result

Returns `ToolRejectedResponse` for pre-commit failures, including:

- stale `expected_state_version`
- unknown or non-pending judgment
- `judgment_kind` mismatch
- invalid selected option
- invalid answer payload
- expired pending judgment
- answer incompatible with the pending judgment
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

```yaml
method: harness.record_user_judgment
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: user
    surface_id: surface_local
    request_id: req_judgment_answer_001
    idempotency_key: idem_judgment_answer_001
    expected_state_version: 22
    dry_run: false
    locale: en-US
  user_judgment_id: uj_001
  judgment_kind: product_decision
  selected_option_id: accept
  answer:
    product_decision:
      judgment:
        decision: accepted
        rationale: "The invoice download confirmation copy is clear enough for this Task."
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

Result branch (`RecordUserJudgmentResult`, committed):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 23
  events:
    - event_id: evt_1006
      event_kind: user_judgment_recorded
user_judgment_ref:
  record_kind: user_judgment
  record_id: uj_001
  project_id: proj_123
  task_id: task_456
  state_version: 23
user_judgment:
  judgment_id: uj_001
  project_id: proj_123
  task_id: task_456
  change_unit_id: cu_001
  judgment_kind: product_decision
  status: resolved
  presentation: short
  question: "Is the invoice download confirmation copy sufficient for this Task?"
  options:
    - option_id: accept
      label: "Sufficient"
      description: "Record the user-owned product decision that the copy is sufficient."
      consequence: "Close readiness can evaluate this product decision as resolved."
      is_default: true
    - option_id: revise
      label: "Revise"
      description: "Keep the Task open for revised confirmation copy."
      consequence: "Close remains blocked on this product decision."
      is_default: false
  context:
    summary: "The confirmation copy appears before invoice PDF download and tells users they are about to download a billing document."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "Invoice PDF download confirmation is in scope; invoice generation is out of scope."
  affected_refs:
    - record_kind: task
      record_id: task_456
      project_id: proj_123
      task_id: task_456
      state_version: 21
  required_for: close
  resolution:
    selected_option_id: accept
    answer:
      product_decision:
        judgment:
          decision: accepted
          rationale: "The invoice download confirmation copy is clear enough for this Task."
    note: null
    accepted_risks: []
    resolved_by_actor_kind: user
  expires_at: null
  created_at: "<example-created-at>"
  resolved_at: "<example-resolved-at>"
updated_refs:
  - record_kind: user_judgment
    record_id: uj_001
    project_id: proj_123
    task_id: task_456
    state_version: 23
state:
  project_id: proj_123
  state_version: 23
next_actions:
  - action_kind: close_task
    owner_method: harness.close_task
    label: "Evaluate close readiness after recording the user's product decision."
    blocking_question: null
    required_refs:
      - record_kind: user_judgment
        record_id: uj_001
        project_id: proj_123
        task_id: task_456
        state_version: 23
```

## Owner links

- Request envelope, response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `UserJudgment`, `RecordUserJudgmentPayload`, `SensitiveActionScope`, and `AcceptedRiskInput`: [API Judgment Schemas](schema-judgment.md).
- State refs and summaries: [API State Schemas](schema-state.md).
- Judgment values and supported method-local values: [API Value Sets](schema-value-sets.md).
- User-owned judgment, final acceptance, residual-risk acceptance, and non-substitution rules: [Core Model](../core-model.md).
- Exact storage effects: [Storage Effects](../storage-effects.md#harnessrecord_user_judgment).
- Public errors, precedence, and rejected-response routing: [API error codes](error-codes.md), [API error precedence](error-precedence.md), and [API error routing](error-routing.md).
- Creating the pending judgment request: [`harness.request_user_judgment`](method-request-user-judgment.md).
