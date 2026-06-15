<a id="harnessrequest_user_judgment"></a>

# `harness.request_user_judgment` reference

## What this document owns

This document owns baseline method behavior for `harness.request_user_judgment`:

- method-specific required inputs, access requirements, state version behavior, result branches, and `dry_run` behavior
- creation of one pending `UserJudgment` for a focused user-owned judgment
- request-user-judgment examples

## What this document does not own

This document does not own:

- common request envelope, response branch, dry-run, or rejected-response schema bodies
- `UserJudgment`, option, context, answer payload, value-set, or status field definitions
- Core user-owned judgment meaning, final acceptance meaning, residual-risk meaning, sensitive-action approval meaning, or `Write Authorization` meaning
- storage record layouts, exact storage effects, public error code meaning, public error precedence, or shared response-branch routing

## Purpose

`harness.request_user_judgment` creates one pending `UserJudgment` for a focused user-owned judgment. It asks the user; the agent must not answer, infer, broaden, or decide the judgment for the user.

The pending judgment is a request for a decision. It is not the decision itself, does not create evidence, does not change current scope, does not create `Write Authorization`, and does not close a `Task`.

## Required inputs

- A valid `ToolEnvelope`; committed non-dry-run requests require non-null `idempotency_key` and current `expected_state_version`.
- `task_id`, `change_unit_id`, `judgment_kind`, `presentation`, `question`, `options`, `context`, `affected_refs`, `required_for`, and `expires_at`.
- A focused `question` with mutually understandable `options`.
- Enough `context` for the user to judge the exact issue without relying on hidden chat state.

## Access requirements

The method requires:

- `VerifiedSurfaceContext.access_class=core_mutation`
- `verified=true`
- a compatible same-project Task and optional Change Unit

Local access failures, unreadable project or Task identity, and insufficient local capability reject before commit.

## State version behavior

A committed non-dry-run result:

- increments `project_state.state_version` exactly once
- creates one pending `UserJudgment`
- may update affected blocker state only as allowed by the storage-effect owner

Non-claims:

- A `UserJudgmentCandidate` returned by another method is not durable until `harness.request_user_judgment` commits.
- Dry run and rejection create no pending judgment, blocker update, event, replay row, or state-version increment.

## Success result

Returns `RequestUserJudgmentResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `user_judgment_ref`
- pending `user_judgment`
- affected `blocker_refs`
- current `state`

## Blocked result

There is no separate committed blocked response branch for this method.

When a pending judgment cannot be created, the method rejects before commit.

## Rejected result

Returns `ToolRejectedResponse` for pre-commit failures such as:

- invalid request shape
- unsupported or incompatible `judgment_kind`
- missing or incompatible Task identity
- unresolved prerequisite judgment
- local access failure
- insufficient capability
- stale `expected_state_version`
- validator failure

Rejected attempts do not create a pending judgment and do not persist request-like blocker data as a side effect.

Public error code meaning, precedence, and rejected-response routing are owned by the error documents linked below.

## Dry-run behavior

For `dry_run=true`, a valid preview:

- returns `ToolDryRunResponse`
- does not return a durable `user_judgment_ref`
- creates no pending `UserJudgment`

## Storage effect

On commit, the method may persist a pending `user_judgments` row and related blocker state. Exact storage effects are owned by the storage documents linked below.

## Minimal valid request

```yaml
method: harness.request_user_judgment
params:
  envelope:
    project_id: proj_banner_001
    task_id: task_banner_001
    actor_kind: agent
    surface_id: surface_banner
    request_id: req_banner_request_001
    idempotency_key: idem_banner_request_001
    expected_state_version: 51
    dry_run: false
    locale: en-US
  task_id: task_banner_001
  change_unit_id: cu_banner_001
  judgment_kind: product_decision
  presentation: short
  question: "Should the dashboard banner use concise copy?"
  options:
    - option_id: concise
      label: "Use concise copy"
      description: "Record the user-owned product decision to keep the shorter banner copy."
      consequence: "The pending banner-copy decision can be treated as resolved."
      is_default: true
    - option_id: expanded
      label: "Use expanded copy"
      description: "Record that the banner copy should include a longer explanation."
      consequence: "The Task remains open for the expanded banner-copy change."
      is_default: false
  context:
    summary: "The dashboard banner has two candidate copy lengths and needs a user-owned product decision."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "Only banner copy length is in scope for this judgment request."
  affected_refs:
    - record_kind: task
      record_id: task_banner_001
      project_id: proj_banner_001
      task_id: task_banner_001
      state_version: 51
  required_for: close
  expires_at: null
```

## Representative response

Result branch (`RequestUserJudgmentResult`, committed):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 52
  events:
    - event_id: evt_banner_001
      event_kind: user_judgment_requested
user_judgment_ref:
  record_kind: user_judgment
  record_id: uj_banner_001
  project_id: proj_banner_001
  task_id: task_banner_001
  state_version: 52
user_judgment:
  judgment_id: uj_banner_001
  project_id: proj_banner_001
  task_id: task_banner_001
  change_unit_id: cu_banner_001
  judgment_kind: product_decision
  status: pending
  presentation: short
  question: "Should the dashboard banner use concise copy?"
  options:
    - option_id: concise
      label: "Use concise copy"
      description: "Record the user-owned product decision to keep the shorter banner copy."
      consequence: "The pending banner-copy decision can be treated as resolved."
      is_default: true
    - option_id: expanded
      label: "Use expanded copy"
      description: "Record that the banner copy should include a longer explanation."
      consequence: "The Task remains open for the expanded banner-copy change."
      is_default: false
  context:
    summary: "The dashboard banner has two candidate copy lengths and needs a user-owned product decision."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "Only banner copy length is in scope for this judgment request."
  affected_refs:
    - record_kind: task
      record_id: task_banner_001
      project_id: proj_banner_001
      task_id: task_banner_001
      state_version: 51
  required_for: close
  resolution: null
  expires_at: null
  created_at: "<example-created-at>"
  resolved_at: null
blocker_refs: []
state:
  project_id: proj_banner_001
  state_version: 52
```

## Owner links

- Request envelope, response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `UserJudgment`, options, context, and answer payloads: [API Judgment Schemas](schema-judgment.md).
- State refs and summaries: [API State Schemas](schema-state.md).
- Judgment kinds and supported values: [API Value Sets](schema-value-sets.md).
- User-owned judgment and non-substitution rules: [Core Model](../core-model.md).
- Exact storage effects: [Storage Effects](../storage-effects.md#harnessrequest_user_judgment).
- Public errors, precedence, and rejected-response routing: [API error codes](error-codes.md), [API error precedence](error-precedence.md), and [API error routing](error-routing.md).
- Recording the user's answer to a pending judgment: [`harness.record_user_judgment`](method-record-user-judgment.md).
