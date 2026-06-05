# MVP-1 API

## What this document helps you do

Use this short reference when you need the active MVP-1 public API surface without the later schema appendix.

This document describes future Harness Server behavior for planning and review. No Harness runtime or server implementation exists in this repository today. Current repository phase and implementation handoff status are tracked in [Implementation Overview](../../build/implementation-overview.md#documentation-acceptance-status).

## Main idea

MVP-1 exposes a small local MCP surface for the user work loop: intake ordinary work, show current status and next safe actions, check proposed writes cooperatively against current scope, record runs and evidence refs, route user-owned judgment, record the user's answer, and close only when the minimal contract allows it.

`harness.next` is not a separate MVP-1 method. MVP-1 callers read next safe actions from `harness.status.next_actions`. A separate `harness.next` method is later/compatibility material in [Schema Later](schema-later.md#harnessnext).

This API does not claim OS-level blocking, arbitrary-tool sandboxing, tamper-proof files, or pre-tool prevention. `harness.prepare_write` is a cooperative pre-write scope check against Core state. Any Write Authorization it returns is a Harness-level record/check, not OS permission, sandboxing, tamper-proof enforcement, or preventive blocking. Stronger preventive or isolated claims require an owner-promoted profile and proof in the relevant security and connector docs.

Active MVP-1 uses one registered reference `capability_profile` for `surface_id=reference-local-mcp`. The profile is routing and capability context, not write authority and not a replacement for Core gates. It affects validator results, blocked reasons, fallback behavior, and guarantee display. If a requested write or guarantee claim depends on an unsupported profile field, the API must lower the display, return `CAPABILITY_INSUFFICIENT` or a structured blocker, and avoid creating a Write Authorization.

Status output follows the three-part model: `harness.status.status_card` is the user status card, agent surfaces may derive an `agent-context-packet` from current status and refs, and Core state is the only operational source of truth. Status cards, next-action text, rendered templates, agent packets, and projections are read-only views; stale views are not authority. The active user-facing compact outputs are exactly `status-card`, `judgment-request`, `run-evidence-summary`, and `close-result`. The active agent-facing compact output is exactly `agent-context-packet`. Detailed report surfaces stay later/profile.

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

MVP-1 request validators use the active schema blocks and value-set summaries in [Schema Core](schema-core.md#stage-specific-active-value-sets). Later enum values and extension branches are defined separately in [Schema Later](schema-later.md) and are not valid for the active MVP-1 validator.

Error codes, MVP-1 status/error condition names, user-facing message patterns, primary error precedence, idempotency replay, and stale-state behavior are owned by [Errors](errors.md). Security meanings for guarantee levels are owned by [Security Reference: Honest guarantee display](../security.md#honest-guarantee-display). `dry_run=true` is non-authoritative for every state-changing tool: it may return validation diagnostics or a would-change summary, but it creates no current record, `task_events` row, artifact, consumable Write Authorization, projection job, or idempotency replay row.

## Active MVP transition matrix

This matrix ties the active MVP public methods to Core ownership, storage side effects, replay behavior, dry-run behavior, and public errors. Detailed request/response schemas remain in each method section below and in [Schema Core](schema-core.md). Detailed lifecycle meaning remains in [Core Model](../core-model.md). Physical persistence remains in [Storage](../storage.md). If a local method paragraph appears broader than this matrix for the active MVP path, read it through this matrix.

In this section, "committed" means `dry_run=false`, no primary `ToolError`, and Core has accepted the state mutation. A blocked decision can still be a committed response when the row-level side effects below explicitly allow blocker storage. "Failure" means validation failure, unavailable Core/MCP access, stale state, same-key/different-hash replay, or another pre-commit error. Failure creates no authoritative state unless the method row explicitly names a committed violation/audit exception.

State-changing methods use committed idempotency rows in `tool_invocations`, scoped by `(project_id, tool_name, idempotency_key)`. The stored `request_hash` is the canonical request hash defined in [Errors: Idempotency](errors.md#idempotency). Same key plus same hash returns the original committed response before new freshness checks or side effects. Same key plus different hash returns `STATE_CONFLICT`. Dry-run calls and pre-commit failures do not create or update replay rows and do not reserve keys. `harness.status` is read-only and does not participate in committed replay.

`UserJudgmentCandidate` is non-mutating candidate/presentation material. It has no `StateRecordRef` and does not satisfy a gate. A committed `harness.request_user_judgment` creates the pending `user_judgments` row. A committed `harness.record_user_judgment` records the user's answer. Neither candidates nor broad prose such as "go ahead" can create sensitive-action permission, final acceptance, residual-risk acceptance, QA waiver, verification-risk acceptance, cancellation, scope change, evidence, close, or Write Authorization unless the pending `judgment_kind`, affected object, scope, and recorded value match.

| Method | Request input | Primary state owner | State version check basis | Idempotency replay basis | Related error codes |
|---|---|---|---|---|---|
| `harness.intake` | `IntakeRequest`: envelope, `user_request`, `requested_mode`, `resume_policy`, `acceptance_criteria`, constraints, `initial_context_refs`. | Core-owned Task and scope state: `project_state`, `tasks`, and initial `change_units` when write-capable work is started. | Existing or resumed Task: `tasks.state_version`. Creating without a resolved primary Task: `project_state.state_version`. | `tool_invocations` row for committed non-dry-run intake; replay returns the same `task_id`, resume/create/supersede decision, and `change_unit_id`. | `VALIDATION_FAILED`, `STATE_CONFLICT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `NO_ACTIVE_TASK`, `VALIDATOR_FAILED`. |
| `harness.status` | `StatusRequest`: envelope plus `include` flags. | None; read-only derived view over current Core rows. | No mutation check. `expected_state_version` may be `null`; supplied stale readable context is reported, not repaired. | None; no `tool_invocations` row. | `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`, `NO_ACTIVE_TASK`, `PROJECTION_STALE` when a requested readable view is stale or failed. |
| `harness.prepare_write` | `PrepareWriteRequest`: envelope, Task/Change Unit, intended operation, paths, tools, commands/classes, product-file-write intent, network, secret scope, sensitive categories, `baseline_ref`. | Core pre-write compatibility state: `write_authorizations` for allowed committed attempts; `blockers` for committed write blockers; Task/Change Unit scope records as inputs. | Resolved primary Task: `tasks.state_version`. No primary Task: `project_state.state_version`. The resulting `WriteAuthorization.basis_state_version` stores the compatibility basis, not necessarily the response state version. | `tool_invocations` row for committed non-dry-run response. Exact replay of committed `decision=allowed` returns the original `write_authorization_ref` with `authorization_effect=returned`; no second authorization is created. | `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `DECISION_REQUIRED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `BASELINE_STALE`, `VALIDATOR_FAILED`. |
| `harness.record_run` | `RecordRunRequest`: envelope, `kind`, Task/Change Unit, optional caller `run_id`, `baseline_ref`, `write_authorization_id`, summary, `artifact_inputs`, and matching payload branch. | Core run/evidence state: `runs`, compatible `write_authorizations`, `artifacts`, `artifact_links`, `evidence_summaries`, and `blockers`. | Resolved primary Task: `tasks.state_version`. No primary Task: `project_state.state_version`. Product-write compatibility also compares the stored `WriteAuthorization.basis_state_version` and full `attempt_scope_json` against current scope and observed facts. | `tool_invocations` row for committed non-dry-run response. Exact replay returns the original Run/evidence response before authorization consumption, Run creation, artifact registration, evidence/blocker update, event append, or projection enqueue. | `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID`, `SCOPE_VIOLATION`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `EVIDENCE_INSUFFICIENT`, `VALIDATOR_FAILED`. |
| `harness.request_user_judgment` | `RequestUserJudgmentRequest`: envelope, Task/Change Unit, `judgment_kind`, `presentation`, context, question, user/agent boundary text, affected scope/gates/criteria, payload, expiry. | Core user-judgment state: pending `user_judgments` and any affected `blockers`. | Resolved primary Task: `tasks.state_version`. No primary Task: `project_state.state_version`. | `tool_invocations` row for committed non-dry-run judgment request; replay returns the same `user_judgment_ref` and presentation summary. | `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`, `PROJECTION_STALE`, `VALIDATOR_FAILED`. |
| `harness.record_user_judgment` | `RecordUserJudgmentRequest`: envelope, `user_judgment_id`, matching `judgment_kind`, selected option, judgment payload, note, optional waiver reason, and `accepted_risks`. | Core user-judgment state: resolved `user_judgments`, affected `blockers`, and affected Task/Change Unit decision state when the stored pending judgment explicitly covers it. | The Task that owns the stored `user_judgment`: `tasks.state_version`. If no Task owner exists, `project_state.state_version`. | `tool_invocations` row for committed non-dry-run answer; replay returns the same resolved judgment response and must not re-resolve, duplicate accepted-risk refs, or append another event. | `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_UNRESOLVED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `VALIDATOR_FAILED`. |
| `harness.close_task` | `CloseTaskRequest`: envelope, `task_id`, close `intent`, requested close reason, user note, optional superseding Task. | Core close state: terminal/open `tasks`, current `blockers`, and derived close result over active scope, Runs, user judgments, evidence, artifacts, final acceptance, and residual-risk state. | Target Task: `tasks.state_version`. | `tool_invocations` row for committed non-dry-run close attempt. Exact replay of a successful terminal close returns the same terminal response; conflicting intent or changed payload returns `STATE_CONFLICT`. | `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_UNRESOLVED`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `EVIDENCE_INSUFFICIENT`, `ARTIFACT_MISSING`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `VALIDATOR_FAILED`. |

| Method | Rows created | Rows updated | Events appended | Response refs returned | Side effects forbidden on failure | Side effects forbidden during dry-run | Close/status blockers affected |
|---|---|---|---|---|---|---|---|
| `harness.intake` | New `tasks`; initial `change_units` for write-capable `direct` or `work`; `blockers` only for a committed initial blocker; `tool_invocations` for committed replay. | `project_state.active_task_id` / `project_state.state_version`; existing `tasks` and `change_units` when resuming or superseding. | Task create/resume/supersede and initial scope/blocker events for committed mutations. | `task_id`, `change_unit_id`, `state`, `next_action`. | No Task, Change Unit, blocker, project active-task update, event, artifact, Write Authorization, evidence summary, projection job, or replay row. | No Task, Change Unit, blocker, event, state-version advance, projection job, or replay row. | May create or resolve active-task, initial-scope, or initial-question blockers visible through `harness.status`; does not create evidence, acceptance, residual-risk acceptance, or close readiness. |
| `harness.status` | None. | None. | None. | Existing refs only: pending/active `user_judgments`, evidence refs, blocker refs, write-authority summary refs, residual-risk refs, and source state refs as requested. | No mutation of any kind. | Same as read-only behavior; no mutation of any kind. | Returns current blockers, pending judgments, evidence gaps, guarantee/capability conditions, and close-readiness blockers; does not affect them. |
| `harness.prepare_write` | `write_authorizations.status=active` only for committed `dry_run=false` and `decision=allowed`; `blockers` when Core commits a non-error blocked decision; `tool_invocations` for committed replay. | `tasks.state_version` or `project_state.state_version`; affected `blockers`; older active `write_authorizations` only when Core marks them stale for the affected scope. | Committed decision, blocker, authorization creation/staling events. | `write_authorization_ref`, `write_authorization`, `active_user_judgment_refs`, `blocked_reasons.related_error`, non-mutating `user_judgment_candidate`, `state`, `baseline_ref`. | For validation/MCP/state conflict/pre-commit failure: no Write Authorization, blocker row, task event, artifact, evidence summary, projection job, state-version advance, or replay row. A committed blocked decision still must not create a consumable authorization. | No Write Authorization, blocker/current record, task event, artifact, evidence summary, projection job, state-version advance, or replay row; `authorization_effect=would_create` is candidate-only. | May open or resolve write-compatibility, missing-scope, sensitive-action, user-judgment, Autonomy Boundary, baseline, capability, or design-policy blockers. These affect status and may later block close only through their owner records. |
| `harness.record_run` | For a compatible committed Run: `runs`, allowed `artifacts`, `artifact_links`, `evidence_summaries` when absent, `blockers` for recorded gaps, and `tool_invocations`. Explicit violation/audit recording may create `runs.status=violation` and blocker/event rows only when Core deliberately records observed after-the-fact behavior; it does not satisfy evidence or close. | Compatible product-write Runs consume `write_authorizations.status=active` by setting `status=consumed` and `consumed_by_run_id`; update `evidence_summaries`, `blockers`, artifact availability, and affected state version. | Run recording, authorization consumption, artifact/evidence update, blocker/gate update, and explicit violation/audit events. | `run_id`, `write_authorization_ref`, `evidence_ref`, `run_summary_ref`, `direct_result_ref`, `registered_artifacts`, `state`. | For pre-commit rejection: no Run row, artifact, artifact link, evidence summary, authorization consumption, blocker/gate update, task event, projection job, state-version advance, or replay row. Invalid authorizations must never be marked consumed. | No authorization consumption, Run, artifact, artifact link, evidence summary, blocker/gate update, task event, projection job, state-version advance, or replay row. | Updates evidence sufficiency, artifact availability, open-run, scope/authorization, capability, and recovery blockers. A rejected/invalid write-capable Run cannot satisfy evidence, QA, verification, final acceptance, residual-risk acceptance, or close readiness. |
| `harness.request_user_judgment` | Pending `user_judgments`; affected `blockers` only when Core commits the blocker/request linkage; `tool_invocations` for committed replay. | Affected `blockers` and state version; no Approval, Write Authorization, evidence, acceptance, or residual-risk record is resolved by the request itself. | User judgment requested and blocker/request-link events. | `user_judgment_ref`, `user_judgment`, `state`; `approval_id=null` and `reconcile_item_id=null` in minimum MVP-1. | No pending `user_judgment`, blocker update, Approval, reconcile item, Write Authorization, artifact, evidence summary, acceptance, residual-risk acceptance, close, event, or replay row. | No pending `user_judgment`, blocker update, event, state-version advance, or replay row; any returned presentation is candidate-only and has no committed `StateRecordRef`. | Opens or keeps visible product, technical, scope, sensitive-action, QA-waiver, verification-risk, final-acceptance, residual-risk-acceptance, or cancellation blockers. It does not resolve them. |
| `harness.record_user_judgment` | No standalone accepted-risk row in active MVP; `tool_invocations` for committed replay. | `user_judgments.status`, resolution fields, affected `blockers`, affected Task/Change Unit decision state when explicitly covered, and affected state version. Sensitive-action approval updates permission only through the resolved `user_judgment`. | User judgment resolved/deferred/rejected/blocked/superseded and affected blocker/gate recompute events. | `user_judgment_ref`, resolved `user_judgment`, `updated_records`, `accepted_risk_refs`, `state`. | No judgment resolution, blocker update, scope/task update, sensitive-action permission, waiver, final acceptance, residual-risk acceptance, close, event, state-version advance, or replay row. | No judgment resolution, blocker update, permission, waiver, final acceptance, residual-risk acceptance, close, event, state-version advance, or replay row. | May resolve or create continued blockers for the exact pending `judgment_kind`. Final acceptance and residual-risk acceptance remain separate; sensitive approval does not settle product/technical/scope decisions or Write Authorization. |
| `harness.close_task` | `blockers` for a committed blocked close attempt when Core records close blockers; `tool_invocations` for committed replay. No separate `close_readiness` row. | Successful close updates `tasks.lifecycle_phase`, `tasks.result`, `tasks.closed_at`, possibly `project_state.active_task_id`, affected `change_units`, affected `blockers`, and state version. Blocked close leaves the Task open. | Close attempt, blocker update, cancellation/supersession, and successful close events. Active MVP does not require durable projection job storage. | `state`, structured `blockers.related_refs`, `evidence_summary`, `residual_risk_state`, `acceptance_state.accepted_by_ref`, `artifact_refs`; `final_report_refs` is normally `[]` unless a later/profile owner activates reports. | For validation/MCP/state conflict/pre-commit failure: no terminal Task update, blocker row, event, projection job, close record, state-version advance, or replay row. A committed blocked close still must not mark the Task terminal. | No terminal Task update, blocker row, event, projection job, close record, state-version advance, or replay row; any close result is a would-close diagnostic only. | Reads and may record blockers for active Task state, open Run, scope, user judgment, sensitive-action permission, active design policy, evidence sufficiency, artifact availability, final acceptance, residual-risk visibility, residual-risk acceptance, cancellation, or supersession. Verification, Manual QA, projection/report freshness, export, and operations blockers are later/profile-only. |

<a id="harnessintake"></a>

## `harness.intake`

Use this to start, classify, or resume ordinary user work.

Stage meaning: not required for the internal Engineering Checkpoint, which may use an owner-valid setup path instead; active in MVP-1 for plain-language start/resume behavior. MVP-1 requirements shaping persists through Task, Change Unit, and user-judgment boundaries. Committed Shared Design records, full design-support routing, and broad planning workflows are later material unless explicitly promoted.

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

Only committed `dry_run=false` intake mutates `project_state`, `tasks`, `change_units`, `blockers`, `task_events`, or `tool_invocations`. `dry_run=true` may return the classification, would-create/would-resume outcome, and next action, but it creates no Task, Change Unit, blocker, event, replay row, evidence, artifact, Write Authorization, projection job, acceptance, residual-risk acceptance, or close state.

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

`status_card` is a short readable view over current Core state and refs. It should stay compact, show source/freshness information, and avoid full schemas, DDL, history, templates, projection bodies, artifact bodies, logs, and future catalogs. It is not Core state and cannot create sensitive-action approval, final acceptance, residual-risk acceptance, evidence, close readiness, Write Authorization, or close.

`StatusResponse` must show `guarantee_display.level` whenever Core can answer. `include.guarantees=false` may reduce optional notes or expanded capability details, but it must not hide the active guarantee level or, when Core cannot answer, the clear `MCP_UNAVAILABLE`/capability condition that no authoritative state mutation claim can be made.

`next_actions` is the MVP-1 next-safe-action surface. It should name the smallest useful next action or unblocker in ordinary language, with exact enum values as secondary detail.

`evidence_summary` is the Core-owned compact MVP-1 evidence summary. `evidence_refs` carries the active minimal evidence coverage refs, normally `StateRecordRef.record_kind=evidence_summary`, plus artifact refs where the nested schema permits them. These fields are not a full Evidence Manifest table or report and do not replace verification, Manual QA, final acceptance, residual-risk acceptance, or close.

When status cannot reach Core, reports stale state, names an unsupported surface, or shows blockers such as out-of-scope work, missing judgment, missing evidence, close blocked, or residual risk present, it uses the canonical condition behavior in [Errors: MVP-1 guarantee and status taxonomy](errors.md#mvp-1-guarantee-and-status-taxonomy).

MVP-1 active `NextActionSummary.action_kind` values:

```text
ask_user | prepare_write | implement | request_acceptance | close_task | idle
```

Verification, Eval, Manual QA, reconcile, export/recover, and operations next-action kinds are later/profile-gated.

Status is read-only. It must not create state, make product writes compatible, create a Write Authorization, satisfy gates, create evidence, create approval, accept work, accept residual risk, create close readiness, enqueue projection repair, or close a Task.

<a id="harnessprepare_write"></a>

## `harness.prepare_write`

Use this before an agent writes product files. It checks the proposed `AuthorizedAttemptScope` against current Core state and returns either a compatible internal single-use Write Authorization record or structured blockers. This is a cooperative Harness check, not OS permission, sandboxing, or preventive blocking.

Stage meaning: active for Engineering Checkpoint and MVP-1. In MVP-1, sensitive-action permission is represented through a compatible `user_judgment` with `judgment_kind=sensitive_approval`; committed Approval records are later-profile material.

The connected surface `capability_profile` cannot make `decision=allowed` by itself. Active Task, active Change Unit, current state, compatible `prepare_write`, and a durable Write Authorization still come from Core. Product writes must not proceed silently when the recognized surface lacks a required capability such as native artifact capture, command observation, network observation, secret-access observation, pre-tool blocking, or isolation.

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
  product_file_write_intended: boolean
  intended_network:
    - target: string
      direction: read | write
  intended_secret_scope:
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
      required_judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | qa_waiver | verification_risk_acceptance | final_acceptance | residual_risk_acceptance | cancellation | null
  user_judgment_candidate: UserJudgmentCandidate | null
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]
```

The request fields describe the proposed part of [`AuthorizedAttemptScope`](schema-core.md#evidence-and-pre-write-scope-schemas). Core stamps the resolved `task_id`, `change_unit_id`, `basis_state_version`, `surface_id`, related user judgment refs, and guarantee level before creating a durable Write Authorization. `WriteAuthorizationSummary.attempt_scope`, `write_authorizations.attempt_scope_json`, and `record_run` comparison all use that same scope.

`decision=allowed` with `dry_run=false` must include `write_authorization_ref` and an active `write_authorization` whose `attempt_scope` is the stored `AuthorizedAttemptScope`; `dry_run=true` may return `authorization_effect=would_create` but creates no authorization. Here `allowed` means compatible with current Harness records for this API path; it does not mean OS permission or pre-execution blocking, and it is not the durable Write Authorization lifecycle status. Any response whose `decision` is not `allowed` must not include a Write Authorization.

`PrepareWriteResponse` must include `guarantee_display.level` whenever Core can answer. A cooperative or detective level means the surface must hold by instruction or report after-action detection as applicable; it is not a claim that arbitrary tools were prevented. If Core, required MCP access, or a required surface capability is unavailable, the response follows [Errors](errors.md) and must not create a Write Authorization, task event, artifact, projection job, or any authoritative state-mutation claim. `pre_tool_blocking_supported=false` prevents a `preventive` claim, and `isolation_supported=false` prevents an `isolated` claim.

`user_judgment_candidate` is a non-mutating [`UserJudgmentCandidate`](schema-core.md#userjudgmentcandidate). It does not create a user judgment, Approval record, Write Authorization, or projection. If sensitive-action permission is required, MVP-1 returns a `user_judgment_candidate` with `judgment_kind=sensitive_approval` and `judgment_payload.approval_scope`; there is no active MVP-1 `ApprovalRequestCandidate` field or committed Approval request lifecycle.

An exact idempotent replay of a committed non-dry-run `decision=allowed` response returns the original response and original `write_authorization_ref` with `authorization_effect=returned`; it must not create a second Write Authorization or append another event. A same-key replay with a different canonical request hash returns `STATE_CONFLICT`.

Public transition summary: `harness.prepare_write` validates the envelope, validates idempotency and returns exact committed replay before new side effects, resolves the primary Task using the shared request rule, checks `expected_state_version` against `tasks.state_version` or `project_state.state_version` as appropriate, resolves the active Change Unit, builds the candidate `AuthorizedAttemptScope`, checks intended operation/path/tool/command and command-class/product-file-write/network/secret/sensitive-category compatibility, checks baseline freshness, sensitive-action permission, user judgment and decision-gate coverage, Autonomy Boundary, surface capability, and active design-policy preconditions, then calculates `decision`. Only `dry_run=false` with `decision=allowed` creates `write_authorizations.status=active` and stores the full `attempt_scope_json`; committed non-dry-run results append task events before returning.

<a id="harnessrecord_run"></a>

## `harness.record_run`

Use this after a shaping update, direct result, or implementation run. Implementation and direct product-write runs consume a compatible internal Write Authorization record returned by `harness.prepare_write`.

Stage meaning: active for Engineering Checkpoint with one compatible run and one artifact/evidence ref; active in MVP-1 for evidence summaries. Verification input, Feedback Loop updates, TDD Trace updates, and full Evidence Manifest behavior are later/profile-gated.

`record_run` records what the active path can honestly support. When the reference `capability_profile` has `artifact_capture_supported=false`, `command_observation_supported=false`, `network_observation_supported=false`, or `secret_access_observation_supported=false`, native capture, command-observation, network-observation, and secret-access claims must be blocked, narrowed, or marked unverified. Manual artifact refs can support evidence only after the owner path registers them.

`artifact_inputs` accept only the sources defined by `ArtifactInput`: Harness staging, an approved capture adapter, or an existing committed `ArtifactRef`. Caller-supplied arbitrary absolute paths, raw secrets, tokens, and full sensitive logs must not be registered as evidence artifacts. Critical evidence without current owner relation, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `produced_by`, and `retention_class` metadata cannot make `evidence_summary.status=sufficient`.

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
  kind: shaping_update | implementation | direct
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

`RecordRunPayload`, `ShapingUpdatePayload`, `ImplementationPayload`, and `DirectPayload` are defined in [Schema Core: Record-run payloads](schema-core.md#record-run-payloads). `RecordRunRequest.kind`, `RecordRunPayload.kind`, and the one non-null payload branch must match one-to-one. MVP-1 accepts exactly `shaping_update`, `implementation`, and `direct`; `verification_input` is later-profile only.

For `kind=shaping_update`, MVP-1 stores Discovery and requirements-shaping output only as active Task updates, proposed or active Change Unit updates, and user-judgment candidates or records. The active API must not accept or return `record_kind=shared_design`, a committed Shared Design record, a required Shared Design projection, a Discovery Brief record, a Question Queue record, an Assumption Register record, or a First Safe Change Unit Candidate record.

`evidence_ref` points to the active minimal evidence coverage record, normally `StateRecordRef.record_kind=evidence_summary`, and `evidence_summary` returns the current Core-owned compact summary after the Run is recorded. Durable bytes returned by the same operation appear in `registered_artifacts`; Markdown summaries or projection text do not become canonical evidence state.

An exact idempotent replay of a committed `record_run` response returns the original response before current freshness checks, authorization consumption, Run creation, artifact registration, blocker/gate updates, projection enqueue, or event append. It must not consume a Write Authorization twice.

Public transition summary: `harness.record_run` validates the envelope, checks idempotency replay and returns exact committed replay before new side effects, resolves the primary Task using the shared request rule, checks `expected_state_version` against `tasks.state_version` or `project_state.state_version` as appropriate, checks `kind`, detects product writes, requires a compatible active Write Authorization for product writes, loads its stored `AuthorizedAttemptScope`, and compares observed product-file writes, changed paths, tools, commands, command classes, network accesses, secret accesses, sensitive categories, `baseline_ref`, `task_id`, `change_unit_id`, `basis_state_version`, `surface_id`, related user judgment refs, and guarantee level where the active surface can observe or attest them. It then classifies the result as compatible observed attempt, missing required authorization, stale authorization, observed attempt outside authorized scope, or insufficient surface capability. Only a compatible observed attempt consumes the authorization, creates the Run record, registers or links allowed `ArtifactRef` records, updates evidence summary and blockers/gates, appends a task event, and returns the committed response.

If an already registered artifact is missing, has missing required integrity/redaction metadata, or fails integrity validation with a diagnostic such as `hash_mismatch`, Core marks related evidence `stale` or `blocked`. If required evidence is affected, the close path remains blocked until replacement, recovery, waiver/risk handling, or another owner-approved resolution applies.

If Core rejects a write-capable run before commit, `run_id` is `null`, no artifacts are registered, and the response must not imply a Run exists. A pre-commit failed `record_run` must not create artifacts, artifact links, evidence summaries, Run rows, authorization consumption, blocker/gate updates, task events, projection jobs, state-version advances, or replay rows. Core must not mark an invalid authorization as consumed. A violation/audit Run is the only active-contract exception and may be recorded only when Core deliberately records observed behavior after a product write; attempted authorization refs may appear in validator findings, violation payloads, or event payloads, but they do not satisfy evidence, QA, verification, final acceptance, residual-risk acceptance, or close readiness.

<a id="harnessrequest_user_judgment"></a>
<a id="harnessrequest_user_decision"></a>

## `harness.request_user_judgment`

Compatibility alias: `harness.request_user_decision`.

Use this to create a focused user judgment request when user-owned product decision, technical decision, scope decision, sensitive-action approval, QA waiver, verification-risk acceptance, final acceptance, residual-risk acceptance, or cancellation blocks progress or close.

Stage meaning: not active in Engineering Checkpoint; active in MVP-1. Full-format Decision Packet presentation, committed Approval record lifecycle, reconcile, and rich residual-risk profiles are later/profile-gated unless explicitly active.

Allowed actors: `lead_agent`, `evaluator`, `operator`.

```yaml
RequestUserJudgmentRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | qa_waiver | verification_risk_acceptance | final_acceptance | residual_risk_acceptance | cancellation
  presentation: short | full
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary | null
  question: string
  what_user_is_judging: string
  why_agent_cannot_decide: string
  no_decision_consequence: string
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

`approval_id` is `null` in minimum MVP-1. A sensitive-action approval judgment records scoped permission only after `harness.record_user_judgment` resolves it; it is not Write Authorization and does not settle product, technical, scope, final-acceptance, waiver, or residual-risk judgment.

`harness.request_user_judgment` is the committed request path. It creates a pending `user_judgments` row only when `dry_run=false` and Core commits the request. A `UserJudgmentCandidate` returned by `harness.prepare_write`, `harness.status`, or dry-run validation is a presentation candidate with no `StateRecordRef`; it is not a pending judgment, does not grant sensitive-action permission, and does not satisfy any close/status blocker.

An exact idempotent replay of a committed judgment request returns the original `user_judgment_ref`, `user_judgment`, state, and user-visible summary before new state-version checks or side effects. Same-key/different-hash replay returns `STATE_CONFLICT`. `dry_run=true` may validate the question and return a would-request presentation, but creates no `user_judgments` row, blocker link, event, replay row, Approval, reconcile item, evidence, Write Authorization, acceptance, residual-risk acceptance, or close state.

<a id="harnessrecord_user_judgment"></a>
<a id="harnessrecord_user_decision"></a>

## `harness.record_user_judgment`

Compatibility alias: `harness.record_user_decision`.

Use this to record the user's answer to an existing canonical `UserJudgment`.

Stage meaning: not active in Engineering Checkpoint; active in MVP-1 for user-owned judgments, sensitive-action approval judgment resolutions, QA waiver/risk paths when policy allows them, verification-risk acceptance when required verification is waived, final acceptance when required, residual-risk acceptance when required, and cancellation. Committed Approval updates, reconcile outcomes, and richer residual-risk metadata are later/profile-gated unless explicitly active.

Allowed actors: `user`, `operator`.

```yaml
RecordUserJudgmentRequest:
  envelope: ToolEnvelope
  user_judgment_id: string
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | qa_waiver | verification_risk_acceptance | final_acceptance | residual_risk_acceptance | cancellation
  selected_option_id: string | null
  judgment: RecordUserJudgmentPayload
  note: string
  waiver_reason: string | null
  accepted_risks: AcceptedRiskInput[]

RecordUserJudgmentPayload:
  value: selected | rejected | deferred | granted | denied | expired | waived | accepted | cancelled
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

`judgment_kind` must match the stored `UserJudgment`. Free-form notes such as "yes, do it," "go ahead," or "looks good" cannot broaden the answer into sensitive-action approval, final acceptance, residual-risk acceptance, QA waiver, verification-risk acceptance, cancellation, scope change, or pre-write scope-check compatibility unless the pending judgment explicitly asks for that `judgment_kind`, the affected object and scope match, and the recorded user intent matches its allowed value.

In MVP-1, `accepted_risk_refs` contain the `user_judgment` and `blocker` refs that show the risk was visible and accepted for this close path. Rich `residual_risk` refs are later/profile-promoted; there is no standalone accepted-risk record kind.

`accepted_risks` uses [`AcceptedRiskInput`](schema-core.md#acceptedriskinput) only when `judgment_kind=residual_risk_acceptance`; it must be `[]` for every other judgment kind. Rich residual-risk lifecycle metadata stays later/profile-gated.

An exact idempotent replay of a committed answer returns the original resolved judgment response before any new state-version check, blocker update, gate recompute, event append, or replay write. Same-key/different-hash replay returns `STATE_CONFLICT`. `dry_run=true` validates the answer and reports the would-change effect, but it does not resolve the `user_judgment`, update blockers, grant sensitive-action permission, record a waiver, record final acceptance, record residual-risk acceptance, update scope/task state, append an event, advance state version, or create a replay row.

Public transition summary: `harness.record_user_judgment` validates the envelope, checks committed replay, loads the addressed `user_judgment`, verifies that the stored `judgment_kind`, affected object, scope, and pending status match the request, checks `expected_state_version` against the owning Task or project state, validates the selected value and payload for that `judgment_kind`, applies only the covered judgment effect, updates affected blockers/gates, appends the committed event, and returns the resolved judgment response. Broad notes cannot widen the recorded effect beyond the pending judgment contract.

<a id="harnessclose_task"></a>

## `harness.close_task`

Use this to ask Core whether a Task can complete, cancel, or be superseded.

Stage meaning: optional narrow blocker/status smoke for Engineering Checkpoint; active close-readiness and blocker response for MVP-1. Detached verification, Manual QA, full assurance, report freshness, export, and operations blockers are later/profile-gated.

Allowed actors: `user`, `lead_agent`, `operator`.

```yaml
CloseTaskRequest:
  envelope: ToolEnvelope
  task_id: string
  intent: complete | cancel | supersede
  requested_close_reason: completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  user_note: string | null
  superseded_by_task_id: string | null

CloseTaskResponse:
  base: ToolResponseBase
  close_state: open | blocked | closed | cancelled | superseded
  closed: boolean
  close_reason: none | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  assurance_level: none | self_checked
  residual_risk_state: ResidualRiskSummary
  evidence_summary: EvidenceSummary | null
  acceptance_state:
    status: not_required | required | pending | accepted | rejected
    accepted_by_ref: StateRecordRef | null
    required_before_close: boolean
  state: StateSummary
  blockers:
    - code: ErrorCode
      category: task | open_run | scope | user_judgment | sensitive_approval | design_policy | evidence | artifact_availability | final_acceptance | residual_risk_visibility | residual_risk_acceptance | cancellation | supersession
      required_judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | final_acceptance | residual_risk_acceptance | cancellation | null
      message: string
      required_next_action: string
      related_refs: StateRecordRef[]
  final_report_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
```

MVP-1 close uses the active Task, active scope, open-Run state, blockers, residual-risk visibility, final-acceptance state when required, artifact availability, and the Core-owned `evidence_summary`. Close readiness is derived from current records. `completed_verified`, `assurance_level=detached_verified`, `profile_required_verification`, verification blockers, Manual QA blockers, projection/report freshness blockers, and operations refs are later/profile-only extensions owned by [Schema Later](schema-later.md#later-close-and-assurance-extensions).

For `intent=complete`, a closed response requires a Task state compatible with the close intent, no unresolved close-relevant active Run, no unresolved or blocked required user judgment, `evidence_summary.status=sufficient` when evidence is required, recorded `judgment_kind=final_acceptance` when final acceptance is required, visible close-relevant residual risk, and explicit residual-risk acceptance for `completed_with_risk_accepted`. Close-required artifact refs must still be available and match their required owner relation, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `produced_by`, and `retention_class` metadata; missing artifacts or `hash_mismatch`-style integrity failures make the affected evidence stale or blocked. Stale or blocked Write Authorization facts affect close only through the current Run, scope, artifact, evidence, or blocker records they affect; they are not close results by themselves. Projection freshness is display freshness, not canonical close state; callers must not close from stale projection prose.

`CloseTaskRequest` does not carry accepted-risk refs. For `completed_with_risk_accepted`, Core reads accepted state from the blocker that made the close-relevant risk visible and the residual-risk acceptance `user_judgment`; rich Residual Risk records are needed only when that later profile is active.

Successful close moves the Task to a terminal state. A committed blocked close leaves it open and returns structured blockers. A validation, Core/MCP, stale-state, or same-key/different-hash failure creates no terminal Task update, blocker row, event, projection job, state-version advance, or replay row. Repeated successful close with the same idempotency key returns the same terminal response; a conflicting close intent returns `STATE_CONFLICT`.

`dry_run=true` checks the close intent and returns would-close or would-block diagnostics only. It must not mark the Task terminal, create or update close blockers, append close events, enqueue projection jobs, advance state version, create a close record, or create a replay row.
