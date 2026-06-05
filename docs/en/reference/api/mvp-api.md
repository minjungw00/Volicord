# MVP-1 API

## What this document helps you do

Use this short reference when you need the active MVP-1 public API surface without the later schema appendix.

This document describes future Harness Server behavior for planning and review. No Harness runtime or server implementation exists in this repository today. Current repository phase and implementation handoff status are tracked in [Implementation Overview](../../build/implementation-overview.md#documentation-acceptance-status).

## Main idea

MVP-1 exposes a small local MCP surface for the user work loop: intake ordinary work, show current status and next safe actions, check proposed writes cooperatively against current scope, record runs and evidence refs, route user-owned judgment, record the user's answer, and close only when the minimal contract allows it.

`harness.next` is not a separate MVP-1 method. MVP-1 callers read next safe actions from `harness.status.next_actions`. A separate `harness.next` method is later/compatibility material in [Schema Later](schema-later.md#harnessnext).

This API does not claim OS-level blocking, arbitrary-tool sandboxing, tamper-proof files, or pre-tool prevention. `harness.prepare_write` is a cooperative pre-write scope check against Core state. Any Write Authorization it returns is a Harness-level record/check, not OS permission, sandboxing, tamper-proof enforcement, or preventive blocking. Stronger preventive or isolated claims require an owner-promoted profile and proof in the relevant security and connector docs.

Status output follows the three-part model: `harness.status.status_card` is the user status card, agent surfaces may derive an agent context packet from current status and refs, and Core state is the only operational source of truth. Status cards, next-action text, rendered templates, and projections are read-only views; stale views are not authority. The active compact view set is exactly `status-card`, `agent-context-packet`, `judgment-request`, `run-evidence-summary`, and `close-result`; detailed report surfaces stay later/profile.

## MVP-1 method set

| Method | MVP-1 role |
|---|---|
| [`harness.status`](#harnessstatus) | Return current scope, blockers, pending judgments, evidence summary, next actions, and close readiness. |
| [`harness.intake`](#harnessintake) | Start or resume plain-language work and classify it as advice/read-only, small direct work, or tracked work. |
| [`harness.request_user_judgment`](#harnessrequest_user_judgment) | Create a focused user judgment request. |
| [`harness.record_user_judgment`](#harnessrecord_user_judgment) | Record the user's answer to a pending user judgment. |
| [`harness.prepare_write`](#harnessprepare_write) | Run a pre-write scope check for proposed product writes against current Task, scope, baseline, sensitive-action permission, and user judgment coverage. |
| [`harness.record_run`](#harnessrecord_run) | Record a shaping, implementation, or direct run and minimal artifact/evidence refs. |
| [`harness.close_task`](#harnessclose_task) | Check close readiness and close, cancel, or supersede only when blockers allow it. |

## Not MVP-1

These surfaces remain later/profile-gated unless an owner document promotes them:

- separate `harness.next`
- `harness.launch_verify`
- `harness.record_eval`
- `harness.record_manual_qa`
- committed Approval record lifecycle beyond sensitive-action approval as a `user_judgment`
- full Evidence Manifest, detached verification or detached Eval system, full Manual QA matrix, reconcile, export/recover suite, broad operations, and detailed diagnostic projections

## Shared request rules

All methods use [`ToolEnvelope`](schema-core.md#tool-envelope) and [`ToolResponseBase`](schema-core.md#common-response). State-changing tools require a non-null `idempotency_key` and a current `expected_state_version`. Read-only tools may set `expected_state_version` to `null`.

When a method has both a tool-specific `task_id` and `ToolEnvelope.task_id`, the tool-specific `task_id` is the first primary Task candidate. Core resolves the primary Task in this order: tool-specific `task_id`, envelope `task_id`, then active Task resolution. If no primary Task exists, the mutation is project-scoped for `expected_state_version` and `ToolResponseBase.state_version`.

MVP-1 request validators use the active value sets in [Schema Core](schema-core.md#stage-specific-active-value-sets). Later enum values and extension branches are not valid merely because they exist in [Schema Later](schema-later.md).

Error codes, MVP-1 status/error condition names, user-facing message patterns, primary error precedence, idempotency replay, and stale-state behavior are owned by [Errors](errors.md). Security meanings for guarantee levels are owned by [Security Reference: Honest guarantee display](../security.md#honest-guarantee-display). `dry_run=true` is non-authoritative for every state-changing tool: it may return validation diagnostics or a would-change summary, but it creates no current record, `task_events` row, artifact, consumable Write Authorization, projection job, or idempotency replay row.

<a id="harnessintake"></a>

## `harness.intake`

Use this to start, classify, or resume ordinary user work.

Stage meaning: not required for the internal Engineering Checkpoint, which may use an owner-valid setup path instead; active in MVP-1 for plain-language start/resume behavior. Full discovery, design-support routing, and broad planning workflows are later material unless explicitly promoted.

Allowed actors: `user`, `lead_agent`, `operator`.

```yaml
IntakeRequest:
  envelope: ToolEnvelope
  user_request: string
  requested_mode: advisor | direct | work | auto
  resume_policy: resume_active | create_new | supersede_active | reject_if_active
  acceptance_criteria: string[]
  constraints:
    allowed_paths: string[]
    non_goals: string[]
    sensitive_categories: string[]
  initial_context_refs: StateRecordRef[]

IntakeResponse:
  base: ToolResponseBase
  task_id: string
  created: boolean
  resumed: boolean
  state: StateSummary
  next_action: string
  change_unit_id: string | null
```

Core may create or resume a Task, set the work mode, and create an initial scoped boundary for write-capable direct or tracked work. Idempotent replay returns the same Task/resume decision; changed payload replay returns `STATE_CONFLICT`.

<a id="harnessstatus"></a>

## `harness.status`

Use this to answer "where are we now, what blocks progress, and what is the next safe action?"

Stage meaning: active for Engineering Checkpoint minimal status/blocker output; active for MVP-1 user-facing current position, pending user judgments, evidence summary, close readiness, and `next_actions`.

Allowed actors: `user`, `lead_agent`, `evaluator`, `operator`.

```yaml
StatusRequest:
  envelope: ToolEnvelope
  include:
    task: boolean
    gates: boolean
    projections: boolean
    pending_user_judgments: boolean
    guarantees: boolean
    user_judgments: boolean
    autonomy_boundary: boolean
    write_authority: boolean
    residual_risk: boolean

StatusResponse:
  base: ToolResponseBase
  active_task: StateSummary | null
  status_card: string
  next_actions: NextActionSummary[]
  pending_user_judgments: StateRecordRef[]
  active_user_judgment_refs: StateRecordRef[]
  autonomy_boundary_summary: AutonomyBoundarySummary | null
  write_authority_summary: WriteAuthoritySummary | null
  residual_risk_summary: ResidualRiskSummary | null
  evidence_summary: EvidenceSummary | null
  evidence_refs: StateRecordRef[]
  blocker_refs: StateRecordRef[]
  projection_freshness:
    status: current | stale | failed | unknown
    stale_refs: StateRecordRef[]
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]
```

`status_card` is a short readable view over current Core state and refs. It should stay compact, show source/freshness information, and avoid full schemas, DDL, history, templates, projection bodies, artifact bodies, logs, and future catalogs. It is not Core state and cannot create approval, acceptance, residual-risk acceptance, evidence, close readiness, Write Authorization, or close.

`next_actions` is the MVP-1 next-safe-action surface. It should name the smallest useful next action or unblocker in ordinary language, with exact enum values as secondary detail.

`evidence_summary` is the Core-owned compact MVP-1 evidence summary. `evidence_refs` carries the active minimal evidence coverage refs, normally `StateRecordRef.record_kind=evidence_summary`, plus artifact refs where the nested schema permits them. These fields are not a full Evidence Manifest table or report and do not replace verification, Manual QA, work acceptance, residual-risk acceptance, or close.

When status cannot reach Core, reports stale state, names an unsupported surface, or shows blockers such as out-of-scope work, missing judgment, missing evidence, close blocked, or residual risk present, it uses the canonical condition behavior in [Errors: MVP-1 guarantee and status taxonomy](errors.md#mvp-1-guarantee-and-status-taxonomy).

MVP-1 active `NextActionSummary.action_kind` values:

```text
ask_user | prepare_write | implement | request_acceptance | close_task | idle
```

Verification, Eval, Manual QA, reconcile, export/recover, and operations next-action kinds are later/profile-gated.

Status is read-only. It must not create state, make product writes compatible, create a Write Authorization, satisfy gates, create evidence, create approval, accept work, accept residual risk, create close readiness, enqueue projection repair, or close a Task.

<a id="harnessprepare_write"></a>

## `harness.prepare_write`

Use this before an agent writes product files. It checks the exact proposed write against current Core state and returns either a compatible internal single-use Write Authorization record or structured blockers. This is a cooperative Harness check, not OS permission, sandboxing, or preventive blocking.

Stage meaning: active for Engineering Checkpoint and MVP-1. In MVP-1, sensitive-action permission is represented through a compatible `user_judgment` with `judgment_type=sensitive_action_approval`; committed Approval records are later-profile material.

Allowed actors: `lead_agent`, `operator`.

```yaml
PrepareWriteRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  intended_operation: string
  intended_paths: string[]
  intended_tools: string[]
  intended_commands:
    - command: string
      command_class: string
      writes_product_files: boolean
  intended_network:
    - target: string
      direction: read | write
  intended_secrets:
    - secret_handle: string
      access_kind: read | write
  sensitive_categories: string[]
  baseline_ref: string | null

PrepareWriteResponse:
  base: ToolResponseBase
  decision: allowed | blocked | approval_required | decision_required | state_conflict
  state: StateSummary | null
  change_unit_id: string | null
  baseline_ref: string | null
  write_authorization_ref: StateRecordRef | null
  write_authorization: WriteAuthorizationSummary | null
  authorization_effect: none | would_create | created | returned
  active_user_judgment_refs: StateRecordRef[]
  blocked_reasons:
    - code: string
      message: string
      related_error: ErrorCode
  approval_request_candidate: ApprovalRequestCandidate | null
  user_judgment_candidate: UserJudgmentCandidate | null
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]
```

`decision=allowed` with `dry_run=false` must include `write_authorization_ref` and an active `write_authorization`; `dry_run=true` may return `authorization_effect=would_create` but creates no authorization. Here `allowed` means compatible with current Harness records for this API path; it does not mean OS permission or pre-execution blocking, and it is not the durable Write Authorization lifecycle status. Any response whose `decision` is not `allowed` must not include a Write Authorization.

`approval_request_candidate` and `user_judgment_candidate` are non-mutating candidate payloads. They do not create user judgments, Approval records, Write Authorizations, or projections.

An exact idempotent replay of a committed non-dry-run `decision=allowed` response returns the original response and original `write_authorization_ref` with `authorization_effect=returned`; it must not create a second Write Authorization or append another event. A same-key replay with a different canonical request hash returns `STATE_CONFLICT`.

Public transition summary: `harness.prepare_write` validates the envelope, validates idempotency and returns exact committed replay before new side effects, resolves the primary Task using the shared request rule, checks `expected_state_version` against `tasks.state_version` or `project_state.state_version` as appropriate, resolves the active Change Unit, checks intended operation/path/tool/command/network/secret/sensitive-category compatibility, checks baseline freshness, sensitive-action permission, user judgment and decision-gate coverage, Autonomy Boundary, surface capability, and active design-policy preconditions, then calculates `decision`. Only `dry_run=false` with `decision=allowed` creates `write_authorizations.status=active`; committed non-dry-run results append task events before returning.

<a id="harnessrecord_run"></a>

## `harness.record_run`

Use this after a shaping update, direct result, or implementation run. Implementation and direct product-write runs consume a compatible internal Write Authorization record returned by `harness.prepare_write`.

Stage meaning: active for Engineering Checkpoint with one compatible run and one artifact/evidence ref; active in MVP-1 for evidence summaries. Verification input, Feedback Loop updates, TDD Trace updates, and full Evidence Manifest behavior are later/profile-gated.

Allowed actors: `lead_agent`, `evaluator`, `operator`.

```yaml
RecordRunRequest:
  envelope: ToolEnvelope
  kind: shaping_update | implementation | direct
  task_id: string
  change_unit_id: string | null
  run_id: string | null
  baseline_ref: string | null
  write_authorization_id: string | null
  summary: string
  artifact_inputs: ArtifactInput[]
  payload: RecordRunPayload

RecordRunPayload:
  shaping_update: ShapingUpdatePayload | null
  implementation: ImplementationPayload | null
  direct: DirectPayload | null

RecordRunResponse:
  base: ToolResponseBase
  run_id: string | null
  state: StateSummary
  write_authorization_ref: StateRecordRef | null
  evidence_ref: StateRecordRef | null
  evidence_summary: EvidenceSummary | null
  run_summary_ref: StateRecordRef | null
  direct_result_ref: StateRecordRef | null
  registered_artifacts: ArtifactRef[]
  next_action: string
```

The payload branch must match `kind`. MVP-1 accepts `shaping_update`, `implementation`, and `direct`; `verification_input` is later-profile only.

`evidence_ref` points to the active minimal evidence coverage record, normally `StateRecordRef.record_kind=evidence_summary`, and `evidence_summary` returns the current Core-owned compact summary after the Run is recorded. Durable bytes returned by the same operation appear in `registered_artifacts`.

An exact idempotent replay of a committed `record_run` response returns the original response before current freshness checks, authorization consumption, Run creation, artifact registration, blocker/gate updates, projection enqueue, or event append. It must not consume a Write Authorization twice.

Public transition summary: `harness.record_run` validates the envelope, checks idempotency replay and returns exact committed replay before new side effects, resolves the primary Task using the shared request rule, checks `expected_state_version` against `tasks.state_version` or `project_state.state_version` as appropriate, checks `kind`, detects product writes, requires a compatible active Write Authorization for product writes, validates observed changed paths, commands, tools, and secret access, consumes the authorization when compatible, creates the Run record, registers or links `ArtifactRef` records, updates evidence summary and blockers/gates, appends a task event, and returns the response.

If Core rejects a write-capable run before commit, `run_id` is `null`, no artifacts are registered, and the response must not imply a Run exists. Core must not mark an invalid authorization as consumed. A violation/audit Run may be recorded only when Core deliberately records observed behavior after a product write; attempted authorization refs may appear in validator findings, violation payloads, or event payloads, but they do not satisfy evidence, QA, verification, work acceptance, or close readiness.

<a id="harnessrequest_user_judgment"></a>
<a id="harnessrequest_user_decision"></a>

## `harness.request_user_judgment`

Compatibility alias: `harness.request_user_decision`.

Use this to create a focused user judgment request when user-owned judgment, sensitive-action permission, work acceptance, or residual-risk acceptance blocks progress or close.

Stage meaning: not active in Engineering Checkpoint; active in MVP-1. Full-format Decision Packet presentation, committed Approval record lifecycle, waiver, reconcile, and full residual-risk profiles are later/profile-gated unless explicitly active.

Allowed actors: `lead_agent`, `evaluator`, `operator`.

```yaml
RequestUserJudgmentRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  judgment_type: product_choice | technical_choice | sensitive_action_approval | work_acceptance | residual_risk_acceptance
  presentation: short | full
  display_label: Product/UX judgment | Technical judgment | Sensitive action approval | Work acceptance | Residual risk acceptance
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary | null
  what_user_is_judging: string
  what_agent_may_decide_without_user: string[]
  affected_scope: UserJudgmentScope
  affected_gates: UserJudgmentGateRef[]
  affected_acceptance_criteria: UserJudgmentCriterionRef[]
  judgment_payload: UserJudgmentPayload
  expires_at: string | null

RequestUserJudgmentResponse:
  base: ToolResponseBase
  user_judgment_id: string
  user_judgment_ref: StateRecordRef
  user_judgment: UserJudgment
  approval_id: string | null
  reconcile_item_id: string | null
  state: StateSummary
  user_visible_summary: string
```

`approval_id` is `null` in minimum MVP-1. A sensitive-action approval judgment records scoped permission only after `harness.record_user_judgment` resolves it; it is not Write Authorization and does not settle product, technical, work-acceptance, or residual-risk judgment.

<a id="harnessrecord_user_judgment"></a>
<a id="harnessrecord_user_decision"></a>

## `harness.record_user_judgment`

Compatibility alias: `harness.record_user_decision`.

Use this to record the user's answer to an existing canonical `UserJudgment`.

Stage meaning: not active in Engineering Checkpoint; active in MVP-1 for user-owned judgments, sensitive-action approval judgment resolutions, and work acceptance when required. Committed Approval updates, waivers, reconcile outcomes, and richer residual-risk metadata are later/profile-gated unless explicitly active.

Allowed actors: `user`, `operator`.

```yaml
RecordUserJudgmentRequest:
  envelope: ToolEnvelope
  user_judgment_id: string
  judgment_type: product_choice | technical_choice | sensitive_action_approval | work_acceptance | residual_risk_acceptance
  selected_option_id: string | null
  judgment: RecordUserJudgmentPayload
  note: string
  waiver_reason: string | null
  accepted_risks: AcceptedRiskInput[]

RecordUserJudgmentPayload:
  value: selected | rejected | deferred | granted | denied | expired | accepted
  value_note: string | null

RecordUserJudgmentResponse:
  base: ToolResponseBase
  user_judgment_id: string
  user_judgment_ref: StateRecordRef
  user_judgment: UserJudgment
  state: StateSummary
  updated_records: StateRecordRef[]
  accepted_risk_refs: StateRecordRef[]
  next_action: string
```

`judgment_type` must match the stored `UserJudgment`. Free-form notes such as "go ahead" or "looks good" cannot broaden the answer into approval, acceptance, risk acceptance, waiver, or pre-write scope-check compatibility unless the pending judgment explicitly asks for that judgment type and the answer matches its allowed value.

In MVP-1, `accepted_risk_refs` contain the `user_judgment` and `blocker` refs that show the risk was visible and accepted for this close path. Rich `residual_risk` refs are later/profile-promoted; there is no standalone accepted-risk record kind.

<a id="harnessclose_task"></a>

## `harness.close_task`

Use this to ask Core whether a Task can complete, cancel, or be superseded.

Stage meaning: optional narrow blocker/status smoke for Engineering Checkpoint; active close-readiness and blocker response for MVP-1. Full assurance, QA, waiver, report freshness, export, and operations blockers are later/profile-gated.

Allowed actors: `user`, `lead_agent`, `operator`.

```yaml
CloseTaskRequest:
  envelope: ToolEnvelope
  task_id: string
  intent: complete | cancel | supersede
  requested_close_reason: completed_verified | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  user_note: string | null
  superseded_by_task_id: string | null

CloseTaskResponse:
  base: ToolResponseBase
  close_state: open | blocked | closed | cancelled | superseded
  closed: boolean
  close_reason: none | completed_verified | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  assurance_level: none | self_checked | detached_verified
  residual_risk_state: ResidualRiskSummary
  evidence_summary: EvidenceSummary | null
  acceptance_state:
    status: not_required | required | pending | accepted | rejected
    accepted_by_ref: StateRecordRef | null
    required_before_close: boolean
  profile_required_verification:
    active: boolean
    status: not_required | required | pending | passed | failed | waived_by_user | blocked
    required_profile: string | null
    related_refs: StateRecordRef[]
  state: StateSummary
  blockers:
    - code: ErrorCode
      category: open_run | scope | user_judgment | sensitive_action_approval | design_policy | evidence | verification | manual_qa | residual_risk_visibility | residual_risk_acceptance | work_acceptance | projection_freshness | artifact_availability
      message: string
      required_next_action: string
      related_refs: StateRecordRef[]
  final_report_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
```

MVP-1 close uses the core close state, blockers, residual-risk visibility, work-acceptance state when required, artifact availability, and the Core-owned `evidence_summary`. Close readiness is derived from current records. Verification, Manual QA, projection/report, and operations refs are active only when their profiles are enabled.

For `intent=complete`, a closed response requires a Task state compatible with the close intent, no unresolved close-relevant active Run, no unresolved or blocked required user judgment, `evidence_summary.status=sufficient` when evidence is required, recorded work acceptance when acceptance is required, visible close-relevant residual risk, and explicit residual-risk acceptance for `completed_with_risk_accepted`. Stale or blocked Write Authorization facts affect close only through the current Run, scope, artifact, evidence, or blocker records they affect; they are not close results by themselves. Projection freshness is display freshness, not canonical close state; callers must not close from stale projection prose.

`CloseTaskRequest` does not carry accepted-risk refs. For `completed_with_risk_accepted`, Core reads accepted state from the blocker that made the close-relevant risk visible and the residual-risk acceptance `user_judgment`; rich Residual Risk records are needed only when that later profile is active.

Successful close moves the Task to a terminal state. Failed close leaves it open and returns structured blockers. Repeated successful close with the same idempotency key returns the same terminal response; a conflicting close intent returns `STATE_CONFLICT`.
