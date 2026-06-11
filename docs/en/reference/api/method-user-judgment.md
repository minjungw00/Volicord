# `harness.request_user_judgment` and `harness.record_user_judgment` reference

## What this document owns

This document owns active MVP method behavior for `harness.request_user_judgment` and `harness.record_user_judgment`:

- method-specific required inputs, access requirements, state-version behavior, result branches, and dry-run behavior
- the minimal requests and representative responses for the shared account data export confirmation scenario
- the method boundary between creating a pending user-owned judgment and recording the user's answer

## What this document does not own

This document does not own:

- common `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, or `ToolDryRunResponse` schema bodies
- `UserJudgment` schema field definitions, judgment value sets, public error precedence, or storage record layouts
- Core user-owned judgment meaning, final acceptance meaning, residual-risk meaning, or close-readiness meaning

## Purpose


<a id="harnessrequest_user_judgment"></a>

### `harness.request_user_judgment`

Create one pending `UserJudgment` for a focused user-owned decision. The method asks the user; the agent must not answer, infer, broaden, or decide the judgment for the user.


<a id="harnessrecord_user_judgment"></a>

### `harness.record_user_judgment`

Record the user's answer to one existing pending `UserJudgment`.

Result:

- The method resolves, rejects, defers, blocks, or marks the specific pending judgment according to the user's answer.

Non-claims:

- It does not broaden the answer into unrelated approval.
- It does not broaden the answer into scope expansion.
- It does not broaden the answer into acceptance or residual-risk acceptance.
- It does not broaden the answer into Write Authorization.

## Required inputs


### `harness.request_user_judgment`

- `ToolEnvelope` with non-null `idempotency_key` and current `expected_state_version` for non-dry-run commits.
- `task_id`, `change_unit_id`, `judgment_kind`, `presentation`, `question`, `options`, `context`, `affected_refs`, `required_for`, and `expires_at`.
- A focused question with mutually understandable options and enough context for the user to judge the exact issue.


### `harness.record_user_judgment`

- `ToolEnvelope` with non-null `idempotency_key` and current `expected_state_version` for non-dry-run commits.
- `user_judgment_id`, matching `judgment_kind`, `selected_option_id`, `answer`, `note`, and `accepted_risks`.
- `answer` must contain only the decision-specific payload branch for the pending `judgment_kind`; `selected_option_id` and `note` stay at request level.

## Access requirements


### `harness.request_user_judgment`

Requires `VerifiedSurfaceContext.access_class=core_mutation` and `verified=true`. The request must target a compatible same-project Task and optional Change Unit.


### `harness.record_user_judgment`

Requires `VerifiedSurfaceContext.access_class=core_mutation` and `verified=true`. The pending judgment must belong to the same project and compatible Task selected by the request.

## State version behavior


### `harness.request_user_judgment`

Committed non-dry-run result:

- increments `project_state.state_version` exactly once
- creates the pending judgment

Non-claims:

- A candidate returned by another method is not durable until this method commits.
- Dry run and rejection create no pending judgment, blocker update, event, replay row, or state-version increment.


### `harness.record_user_judgment`

Committed non-dry-run result:

- increments `project_state.state_version` exactly once
- updates the addressed `user_judgments` row

Non-claim: dry run and rejection create no judgment resolution, blocker update, event, replay row, or state-version increment.

## Success result


### `harness.request_user_judgment`

Returns `RequestUserJudgmentResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `user_judgment_ref`
- pending `user_judgment`
- affected `blocker_refs`
- current `state`


### `harness.record_user_judgment`

Returns `RecordUserJudgmentResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `user_judgment_ref`
- updated `user_judgment`
- `updated_refs`
- current `state`
- `next_actions`

## Blocked result


### `harness.request_user_judgment`

There is no separate committed blocked response branch.

The method rejects before commit when a judgment cannot be created because:

- the request is invalid
- prerequisites cannot be verified


### `harness.record_user_judgment`

The addressed judgment may be committed as `rejected`, `deferred`, `blocked`, or otherwise blocker-producing when that is the user's answer or the compatible result of the focused judgment.

Result:

- updates only covered blockers and judgment-dependent summaries

Non-claims:

- A resolved `scope_decision` alone does not change active scope or active Change Unit fields.
- Those fields still require `harness.update_scope`.

## Rejected result


### `harness.request_user_judgment`

Returns `ToolRejectedResponse` for:

- invalid question shape
- invalid `judgment_kind`
- missing Task
- unresolved prerequisite decision
- local access failure
- insufficient capability
- stale `expected_state_version`
- validator failure

Public error code meaning and precedence are owned by [API Errors](errors.md).


### `harness.record_user_judgment`

Returns `ToolRejectedResponse` for:

- stale `expected_state_version`
- unknown or non-pending judgment
- `judgment_kind` mismatch
- invalid selected option
- invalid answer payload
- expired or incompatible approval
- local access failure
- validator failure

Public error code meaning and precedence are owned by [API Errors](errors.md).

## Dry-run behavior


### `harness.request_user_judgment`

For `dry_run=true`, a valid preview returns `ToolDryRunResponse`. Branch shape is owned by [API Schema Core](schema-core.md); no-effect persistence semantics are owned by [Storage Effects](../storage-effects.md).


### `harness.record_user_judgment`

For `dry_run=true`, a valid preview returns `ToolDryRunResponse`. Branch shape is owned by [API Schema Core](schema-core.md); no-effect persistence semantics are owned by [Storage Effects](../storage-effects.md).

## Storage effect


### `harness.request_user_judgment`

On commit, the method may persist pending judgment and related blocker state. Exact storage effects are owned by [Storage Effects](../storage-effects.md).


### `harness.record_user_judgment`

On commit, the method may persist judgment resolution and dependent blocker or summary state. Exact storage effects are owned by [Storage Effects](../storage-effects.md).

## Minimal valid request


### `harness.request_user_judgment`

```yaml
method: harness.request_user_judgment
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_judgment_001
    idempotency_key: idem_judgment_001
    expected_state_version: 21
    dry_run: false
    locale: en-US
  task_id: task_456
  change_unit_id: cu_001
  judgment_kind: product_decision
  presentation: short
  question: "Is the account export confirmation copy sufficient for account data export that may include personal data?"
  options:
    - option_id: accept
      label: "Sufficient"
      description: "Record the user's judgment that the account export confirmation copy is sufficient."
      consequence: "Close readiness can evaluate the product decision as resolved."
      is_default: true
    - option_id: revise
      label: "Revise"
      description: "Keep the Task open for revised account export confirmation copy."
      consequence: "Close remains blocked on the product decision."
      is_default: false
  context:
    summary: "The account export confirmation copy tells the user that the export may include personal data before download."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "Current Task constraints apply"
  affected_refs:
    - record_kind: task
      record_id: task_456
      project_id: proj_123
      task_id: task_456
      state_version: 21
  required_for: close
  expires_at: null
```


### `harness.record_user_judgment`

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
        rationale: "The account export confirmation copy clearly states that the export may include personal data."
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


### `harness.request_user_judgment`

Result branch (`RequestUserJudgmentResult`, committed):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 22
  events:
    - event_id: evt_1005
      event_kind: user_judgment_requested
user_judgment_ref:
  record_kind: user_judgment
  record_id: uj_001
  project_id: proj_123
  task_id: task_456
  state_version: 22
user_judgment:
  judgment_id: uj_001
  project_id: proj_123
  task_id: task_456
  change_unit_id: cu_001
  judgment_kind: product_decision
  status: pending
  presentation: short
  question: "Is the account export confirmation copy sufficient for account data export that may include personal data?"
  options:
    - option_id: accept
      label: "Sufficient"
      description: "Record the user's judgment that the account export confirmation copy is sufficient."
      consequence: "Close readiness can evaluate the product decision as resolved."
      is_default: true
    - option_id: revise
      label: "Revise"
      description: "Keep the Task open for revised account export confirmation copy."
      consequence: "Close remains blocked on the product decision."
      is_default: false
  context:
    summary: "The account export confirmation copy tells the user that the export may include personal data before download."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "Current Task constraints apply"
  affected_refs:
    - record_kind: task
      record_id: task_456
      project_id: proj_123
      task_id: task_456
      state_version: 21
  required_for: close
  resolution: null
  expires_at: null
  created_at: "<example-created-at>"
  resolved_at: null
blocker_refs: []
state:
  project_id: proj_123
  state_version: 22
```


### `harness.record_user_judgment`

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
  question: "Is the account export confirmation copy sufficient for account data export that may include personal data?"
  options:
    - option_id: accept
      label: "Sufficient"
      description: "Record the user's judgment that the account export confirmation copy is sufficient."
      consequence: "Close readiness can evaluate the product decision as resolved."
      is_default: true
    - option_id: revise
      label: "Revise"
      description: "Keep the Task open for revised account export confirmation copy."
      consequence: "Close remains blocked on the product decision."
      is_default: false
  context:
    summary: "The account export confirmation copy tells the user that the export may include personal data before download."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "Current Task constraints apply"
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
          rationale: "The account export confirmation copy clearly states that the export may include personal data."
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
  - action: harness.close_task
    reason: "Evaluate close readiness after recording the user's product decision."
```

## Owner links


### `harness.request_user_judgment`

- Request envelope, response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `UserJudgment`, options, context, and answer payloads: [API Judgment Schemas](schema-judgment.md).
- State refs and summaries: [API State Schemas](schema-state.md).
- Judgment kinds and active values: [API Value Sets](schema-value-sets.md).
- User-owned judgment and non-substitution rules: [Core Model](../core-model.md).
- Public errors and persistence effects: [API Errors](errors.md) and [Storage Effects](../storage-effects.md).


### `harness.record_user_judgment`

- Request envelope, response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `UserJudgment`, `RecordUserJudgmentPayload`, `SensitiveActionScope`, and `AcceptedRiskInput`: [API Judgment Schemas](schema-judgment.md).
- State refs and summaries: [API State Schemas](schema-state.md).
- Judgment values and active method-local values: [API Value Sets](schema-value-sets.md).
- User-owned judgment, final acceptance, residual-risk acceptance, and non-substitution rules: [Core Model](../core-model.md).
- Public errors and persistence effects: [API Errors](errors.md) and [Storage Effects](../storage-effects.md).
