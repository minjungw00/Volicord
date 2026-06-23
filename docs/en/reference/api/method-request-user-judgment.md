<a id="volicordrequest_user_judgment"></a>

# `volicord.request_user_judgment` reference

## What this document owns

This document owns baseline method behavior for `volicord.request_user_judgment`:

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

`volicord.request_user_judgment` creates one pending `UserJudgment` for a focused user-owned judgment. It asks the user; the agent must not answer, infer, broaden, or decide the judgment for the user.

The pending judgment is a request for a decision. It is not the decision itself, does not create evidence, does not change current scope, does not create `Write Authorization`, and does not close a `Task`.

When this method creates a pending judgment, Core derives a `JudgmentBasis` from current state. Callers do not submit `basis`, `scope_revision`, `close_basis_revision`, session-binding fields, access-class fields, verified actor context, machine action, resolution outcome, or current close-basis authority fields.

## Required inputs

- A valid `ToolEnvelope`; committed non-dry-run requests require non-null `idempotency_key` and current `expected_state_version`.
- `task_id`, `change_unit_id`, `judgment_kind`, `presentation`, `question`, `context`, `affected_refs`, `required_for`, and `expires_at`.
- A focused `question` with mutually understandable `options`.
- For non-authority judgment kinds, `options` is required by Core validation even though the wire field is optional-nullable. The caller-authored `UserJudgmentOptionInput[]` must contain at least one option, and each option has only `option_id`, `label`, `description`, `consequence`, and `is_default`.
- For authority-bearing judgment kinds, `options` may be omitted, `null`, or `[]`. Non-empty caller-authored options reject. Core creates the canonical authority options, localized labels, consequences, `machine_action`, and `resolution_outcome`.
- For `judgment_kind=sensitive_approval`, `sensitive_action_scope` must be present with a non-null `SensitiveActionScope`. For `product_decision`, `technical_decision`, `scope_decision`, and `cancellation`, a non-null `sensitive_action_scope` rejects. `final_acceptance` and `residual_risk_acceptance` derive their basis from the current close basis and do not use `sensitive_action_scope` as caller-submitted authority.
- Enough `context` for the user to judge the exact issue without relying on hidden chat state.

## Request schema

This method owns the top-level `params` request shape below. `envelope` is the shared [`ToolEnvelope`](schema-core.md#tool-envelope); this block does not redefine `ToolEnvelope` fields.

All fields shown in this method-owned request block are required members of `params` unless marked with `?`. `T | null` means a present member may contain JSON `null`; `field?: T | null` means callers may omit the member or send `null`, and omission and explicit `null` decode to the same semantic value.

```yaml
RequestUserJudgmentRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  sensitive_action_scope?: SensitiveActionScope | null
  judgment_kind: string
  presentation: string
  question: string
  options?: UserJudgmentOptionInput[] | null
  context: UserJudgmentContext
  affected_refs: StateRecordRef[]
  required_for: string[]
  expires_at: string | null
```

Request field notes:
- `options` and `sensitive_action_scope` are optional-nullable public request fields. Omission and explicit `null` have the same meaning.
- `basis`, `scope_revision`, `close_basis_revision`, verified actor context, `machine_action`, and `resolution_outcome` are not public request fields.
- Authority-bearing judgment kinds are `scope_decision`, `sensitive_approval`, `final_acceptance`, `residual_risk_acceptance`, and `cancellation`. Core generates canonical authority options for those kinds.
- Caller-authored options are allowed only for `product_decision` and `technical_decision`.

Nested owner links:
- The judgment-candidate fields align with `UserJudgmentCandidate`; request option input, output option, context, `JudgmentBasis`, and `SensitiveActionScope` shapes are owned by [API Judgment Schemas](schema-judgment.md#userjudgmentoptioninput).
- `affected_refs` uses `StateRecordRef[]`; the nested shape is owned by [API State Schemas](schema-state.md#state-references).
- `judgment_kind`, `presentation`, and `required_for` values are owned by [API Value Sets judgment values](schema-value-sets.md#judgment-values).

## Access requirements

The method requires:

- server-derived `VerifiedSurfaceContext` with `access_class=core_mutation`
- a compatible same-project Task and optional Change Unit

Local access failures, unreadable project or Task identity, and insufficient local capability reject before commit.

## State version behavior

A committed non-dry-run result:

- increments `project_state.state_version` exactly once
- creates one pending `UserJudgment`
- stores a Core-derived `JudgmentBasis` with `basis.compatibility_status=current`
- may update affected blocker state only as allowed by the storage-effect owner

Non-claims:

- A `UserJudgmentCandidate` returned by another method is not durable until `volicord.request_user_judgment` commits.
- For `judgment_kind=final_acceptance` or `judgment_kind=residual_risk_acceptance`, Core captures the current close basis in the judgment basis. If the required current close basis or current residual-risk IDs are unavailable, the request rejects before commit.
- For authority-bearing judgment kinds, the Core-created option set must include `machine_action=accept` and `machine_action=reject`. `machine_action=defer` appears only where an owner permits deferral. Labels and explanatory text do not override `machine_action` or `resolution_outcome`.
- For residual-risk acceptance, visible risks in the request context must carry exact current `risk_id` values.
- Dry run and rejection create no pending judgment, blocker update, event, replay row, or state-version increment.

## Success result

Returns `RequestUserJudgmentResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `user_judgment_ref`
- pending `user_judgment`
- affected `blocker_refs`
- current `state`

## Method result fields

`RequestUserJudgmentResult` is the method-specific result branch for a committed user-judgment request. It carries `base: ToolResultBase` and these method-owned top-level fields:

| Field | Result-field meaning |
|---|---|
| `base` | Common result metadata. The `ToolResultBase` shape, including `events`, is owned by [API Schema Core](schema-core.md#common-response). Committed `RequestUserJudgmentResult` branches use `base.response_kind=result` and `base.effect_kind=core_committed`. `base.events[].event_kind`, when present, is an opaque illustrative classification string. |
| `user_judgment_ref` | `StateRecordRef` for the pending `UserJudgment` created by this request. |
| `user_judgment` | The created pending `UserJudgment`. The nested shape, including `options`, `context`, `affected_refs`, `required_for`, and `resolution`, is owned by [API Judgment Schemas](schema-judgment.md#userjudgment). |
| `blocker_refs` | `StateRecordRef[]` for blocker records affected by or still relevant to the pending judgment request. |
| `state` | Current `StateSummary` after the pending judgment is created. Nested state fields are owned by [API State Schemas](schema-state.md). |

The method owns that the committed `user_judgment` is pending and that `resolution` is `null`. The full judgment field body and judgment value sets stay with [API Judgment Schemas](schema-judgment.md) and [API Value Sets](schema-value-sets.md#judgment-values).

## Blocked result

There is no separate committed blocked response branch for this method.

When a pending judgment cannot be created, the method rejects before commit.

## Rejected result

Returns `ToolRejectedResponse` for pre-commit failures such as:

- invalid request shape
- unsupported or incompatible `judgment_kind`
- missing or incompatible Task identity
- unresolved prerequisite judgment
- missing current close basis for final acceptance or residual-risk acceptance
- missing or non-current residual-risk ID for residual-risk acceptance
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

## Non-authority request with caller-authored options

Method-local precondition: `task_banner_001` and `cu_banner_001` already exist in `proj_banner_001`; the current project `state_version` is `51`.

```yaml
method: volicord.request_user_judgment
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
      consequence: "If selected, Core records the concise-copy product decision."
      is_default: true
    - option_id: expanded
      label: "Use expanded copy"
      description: "Record that the banner copy should include a longer explanation."
      consequence: "If selected, Core records the expanded-copy product decision."
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
  required_for:
    - close_complete
  expires_at: null
```

## Authority-bearing request with Core-generated options

Method-local precondition: `task_scope_001` and `cu_scope_001` already exist in `proj_scope_001`; the current project `state_version` is `17`.

```yaml
method: volicord.request_user_judgment
params:
  envelope:
    project_id: proj_scope_001
    task_id: task_scope_001
    actor_kind: agent
    surface_id: surface_scope
    request_id: req_scope_decision_001
    idempotency_key: idem_scope_decision_001
    expected_state_version: 17
    dry_run: false
    locale: en-US
  task_id: task_scope_001
  change_unit_id: cu_scope_001
  judgment_kind: scope_decision
  presentation: short
  question: "Should the Task scope narrow to email-only sign-in for this Change Unit?"
  options: null
  context:
    summary: "The proposed scope update removes social sign-in from this Change Unit."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "Only the requested scope change is covered by this judgment."
  affected_refs:
    - record_kind: task
      record_id: task_scope_001
      project_id: proj_scope_001
      task_id: task_scope_001
      state_version: 17
  required_for:
    - scope_update
  expires_at: null
```

If committed, Core generates the canonical authority options for `scope_decision`; the caller does not submit `machine_action`, `resolution_outcome`, or `basis`.

## Sensitive-approval request

Method-local precondition: `task_export_001` and the current Change Unit `cu_export_001` already exist in `proj_export_001`; the current project `state_version` is `28`.

```yaml
method: volicord.request_user_judgment
params:
  envelope:
    project_id: proj_export_001
    task_id: task_export_001
    actor_kind: agent
    surface_id: surface_export
    request_id: req_sensitive_export_001
    idempotency_key: idem_sensitive_export_001
    expected_state_version: 28
    dry_run: false
    locale: en-US
  task_id: task_export_001
  change_unit_id: cu_export_001
  sensitive_action_scope:
    action_kind: export_customer_report
    description: "Export a customer report for the support handoff."
    intended_paths:
      - reports/customer-handoff.csv
    sensitive_categories:
      - customer_data
    command_or_tool_summary: "Run the report export tool once."
    network_or_host_summary: null
    secret_or_credential_summary: null
    capability_claim: "The export is limited to the named report path."
    expires_at: null
  judgment_kind: sensitive_approval
  presentation: short
  question: "Do you approve this specific customer-report export?"
  options: null
  context:
    summary: "The export contains customer data and needs explicit user approval before the action runs."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "Approval covers only the report path and export described in sensitive_action_scope."
  affected_refs:
    - record_kind: change_unit
      record_id: cu_export_001
      project_id: proj_export_001
      task_id: task_export_001
      state_version: 28
  required_for:
    - prepare_write
  expires_at: null
```

## Representative response

Abbreviated result branch (`RequestUserJudgmentResult`, committed):

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
      consequence: "If selected, Core records the concise-copy product decision."
      machine_action: accept
      resolution_outcome: accepted
      is_default: true
    - option_id: expanded
      label: "Use expanded copy"
      description: "Record that the banner copy should include a longer explanation."
      consequence: "If selected, Core records the expanded-copy product decision."
      machine_action: accept
      resolution_outcome: accepted
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
  required_for:
    - close_complete
  resolution: null
  expires_at: null
  created_at: "<example-created-at>"
  resolved_at: null
blocker_refs: []
state:
  project_id: proj_banner_001
  state_version: 52
  task_ref:
    record_kind: task
    record_id: task_banner_001
    project_id: proj_banner_001
    task_id: task_banner_001
    state_version: 51
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Decide dashboard banner copy length."
  scope_summary: "Dashboard banner copy length decision."
  non_goals:
    - "Changing dashboard layout."
  acceptance_criteria:
    - "The banner copy length matches the user's product decision."
  autonomy_boundary: "Stay within dashboard banner copy."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_banner_001
    project_id: proj_banner_001
    task_id: task_banner_001
    state_version: 51
  baseline_ref: baseline_banner_001
  shaping_readiness: null
  pending_user_judgment_refs:
    - record_kind: user_judgment
      record_id: uj_banner_001
      project_id: proj_banner_001
      task_id: task_banner_001
      state_version: 52
  blocker_refs: []
  write_authority_summary: null
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display: null
```

## Owner links

- Request envelope, response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `UserJudgment`, options, context, and answer payloads: [API Judgment Schemas](schema-judgment.md).
- State refs and summaries: [API State Schemas](schema-state.md).
- Judgment kinds and supported values: [API Value Sets](schema-value-sets.md).
- User-owned judgment and non-substitution rules: [Core Model](../core-model.md).
- Exact storage effects: [Storage Effects](../storage-effects.md#volicordrequest_user_judgment).
- Public errors, precedence, and rejected-response routing: [API error codes](error-codes.md), [API error precedence](error-precedence.md), and [API error routing](error-routing.md).
- Recording the user's answer to a pending judgment: [`volicord.record_user_judgment`](method-record-user-judgment.md).
