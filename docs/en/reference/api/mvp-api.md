# Active MVP API

## What this document helps you do

Use this reference to look up the active current MVP API surface. It owns method-level request, response, state effect, storage owner, error, and security boundary summaries for the active method-name value set owned by [API Schema Core](schema-core.md#current-mvp-value-sets).

This document describes future Harness Server behavior for planning and review. No Harness runtime or server implementation exists in this repository today. Future API or schema candidates are cataloged in [Later Candidate Index](../../later/index.md), not in this active reference. Storage DDL and full shared schema bodies are owned outside this method reference.

## Main Idea

The active MVP API is a small local MCP surface for one user work loop. It can intake work, show status, check proposed product writes against current Core state, record runs and evidence refs, ask and record user-owned judgment, and close only when active blockers allow it.

The API does not provide OS permissions, arbitrary-tool sandboxing, tamper-proof files, pre-tool blocking, or security isolation. `harness.prepare_write` returns a cooperative Harness record/check only.

## Active MVP Method Behavior

The exact active method-name value set is owned by [API Schema Core](schema-core.md#current-mvp-value-sets). This page owns the behavior of those current methods:

| Method | Active role |
|---|---|
| [`harness.intake`](#harnessintake) | Start, resume, or classify ordinary user work. |
| [`harness.status`](#harnessstatus) | Return current state summary, blockers, pending judgments, evidence summary, close state, and next safe actions. |
| [`harness.prepare_write`](#harnessprepare_write) | Check a proposed product write against current scope, state, sensitive-action permission, baseline, and surface capability. |
| [`harness.record_run`](#harnessrecord_run) | Record shaping, direct, or implementation work plus compact evidence and artifact refs. |
| [`harness.request_user_judgment`](#harnessrequest_user_judgment) | Create one pending user-owned judgment request. |
| [`harness.record_user_judgment`](#harnessrecord_user_judgment) | Record the user's answer to an existing pending `UserJudgment`. |
| [`harness.close_task`](#harnessclose_task) | Check close readiness and close, cancel, or supersede only when blockers allow it. |

## Shared Request Rules

All methods use [`ToolEnvelope`](schema-core.md#tool-envelope) and [`ToolResponseBase`](schema-core.md#common-response). State-changing methods require a non-null `idempotency_key` and a current `expected_state_version`. `harness.status` is read-only and may use `expected_state_version: null`.

When a method has a tool-specific `task_id`, Core resolves the primary Task in this order: tool-specific `task_id`, `ToolEnvelope.task_id`, then active Task. Task-scoped mutations compare `expected_state_version` with `tasks.state_version`; project-scoped mutations with no selected Task compare it with `project_state.state_version`.

`dry_run=true` is never authoritative. It may return diagnostics or a would-change result, but it creates no current record, `task_events` row, artifact, consumable Write Authorization, evidence summary, close state, or idempotency replay row.

Error codes, primary error precedence, idempotency, stale-state behavior, close blocker ordering, and user-facing error labels are owned by [API Errors](errors.md). Shared schemas and active value sets are owned by [API Schema Core](schema-core.md).

<a id="harnessintake"></a>

## `harness.intake`

- **Owns:** Task start/resume/classification and the initial active scope boundary for write-capable work.
- **Does not own:** Product writes, evidence sufficiency, user judgment resolution, Write Authorization, final acceptance, residual-risk acceptance, or close.
- **When to call:** At the beginning of ordinary work, or when the caller needs to resume, supersede, or reject an existing active Task.
- **Request:**

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
```

- **Response:**

```yaml
IntakeResponse:
  base: ToolResponseBase
  task_ref: StateRecordRef
  change_unit_ref: StateRecordRef | null
  state: StateSummary
  next_actions: NextActionSummary[]
```

- **State effect:** A committed non-dry-run call may create or resume `tasks`, set `project_state.active_task_id`, create an initial `change_units` row for write-capable `direct` or `work`, update blockers, append events, and create a committed idempotency row. Dry-run and pre-commit failure create none of these.
- **Errors:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `NO_ACTIVE_TASK`, `VALIDATOR_FAILED`.
- **Storage owner:** `project_state`, `tasks`, `change_units`, `blockers`, `task_events`, and `tool_invocations`.
- **Security boundary:** Intake records scope and mode. It does not authorize local access, sensitive actions, product writes, or stronger guarantee levels.

<a id="harnessstatus"></a>

## `harness.status`

- **Owns:** Read-only current-position output over Core state and refs.
- **Does not own:** State mutation, readable-view repair, write compatibility, evidence creation, user judgment resolution, final acceptance, residual-risk acceptance, or close.
- **When to call:** Before choosing the next action, after a state-changing call, or when the caller needs blockers, pending judgments, evidence summary, write-authority summary, close status, or guarantee display.
- **Request:**

```yaml
StatusRequest:
  envelope: ToolEnvelope
  include:
    task: boolean
    pending_user_judgments: boolean
    write_authority: boolean
    evidence: boolean
    close: boolean
    guarantees: boolean
```

- **Response:**

```yaml
StatusResponse:
  base: ToolResponseBase
  active_task: StateSummary | null
  status_card: string
  next_actions: NextActionSummary[]
  pending_user_judgments: StateRecordRef[]
  write_authority_summary: WriteAuthoritySummary | null
  evidence_summary: EvidenceSummary | null
  blocker_refs: StateRecordRef[]
  close_state: ready | blocked | closed | cancelled | superseded | none
  close_blockers: CloseBlocker[]
  guarantee_display: GuaranteeDisplay
```

- **State effect:** None. `harness.status` does not create `tool_invocations` replay rows.
- **Errors:** `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`, `NO_ACTIVE_TASK`, `PROJECTION_STALE` when a requested readable view is stale or failed.
- **Storage owner:** Read-only over `project_state`, `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `evidence_summaries`, `artifacts`, `artifact_links`, and `blockers`.
- **Security boundary:** Without a promoted profile, status displays only the current MVP `GuaranteeDisplay.level` values `cooperative` or `detective`. `preventive` and `isolated` may appear only as profile-gated display values supported by the schema and security owners. Stale status text, chat, rendered views, and cached summaries are not authority.

<a id="harnessprepare_write"></a>

## `harness.prepare_write`

- **Owns:** The cooperative pre-write scope check and durable single-use Write Authorization when the proposed attempt is compatible.
- **Does not own:** OS permission, sandboxing, tamper-proof enforcement, pre-tool blocking, user judgment creation, evidence sufficiency, run recording, or close.
- **When to call:** Immediately before a product-file write or other write-capable action that must match current Task, Change Unit, baseline, sensitive-action permission, and surface capability.
- **Request:**

```yaml
PrepareWriteRequest:
  envelope: ToolEnvelope
  task_id: string | null
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
```

- **Response:**

```yaml
PrepareWriteResponse:
  base: ToolResponseBase
  decision: allowed | blocked | approval_required | decision_required | state_conflict
  state: StateSummary | null
  write_authorization_ref: StateRecordRef | null
  write_authorization: WriteAuthorizationSummary | null
  authorization_effect: none | would_create | created | returned
  active_user_judgment_refs: StateRecordRef[]
  blocked_reasons: CloseBlocker[]
  user_judgment_candidate: UserJudgmentCandidate | null
  guarantee_display: GuaranteeDisplay
```

- **State effect:** A committed non-dry-run `decision=allowed` creates exactly one `write_authorizations.status=active` row and a replay row. A committed blocked response may update blockers, but it must not create a consumable authorization. Dry-run and pre-commit failure create no current record, authorization, blocker row, event, artifact, evidence summary, or replay row.
- **Errors:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `DECISION_REQUIRED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `BASELINE_STALE`, `VALIDATOR_FAILED`.
- **Storage owner:** `write_authorizations`, `blockers`, `tasks` or `project_state` version clocks, `task_events`, and `tool_invocations`.
- **Security boundary:** `decision=allowed` means compatible with Harness records for this attempt. It does not mean the operating system will block incompatible writes or that arbitrary tools are isolated.

<a id="harnessrecord_run"></a>

## `harness.record_run`

- **Owns:** Run recording, compatible Write Authorization consumption, artifact registration, compact evidence-summary updates, and run-related blockers.
- **Does not own:** New scope, user judgment resolution, final acceptance, residual-risk acceptance, separate assurance records, or close.
- **When to call:** After shaping work, a direct answer/result, or implementation work. Product-write runs must provide a compatible active Write Authorization from `harness.prepare_write`.
- **Request:**

```yaml
RecordRunRequest:
  envelope: ToolEnvelope
  task_id: string | null
  change_unit_id: string | null
  kind: shaping_update | implementation | direct
  run_id: string | null
  baseline_ref: string | null
  write_authorization_id: string | null
  summary: string
  observed_changes: ObservedChanges
  artifact_inputs: ArtifactInput[]
  evidence_updates: EvidenceCoverageItem[]
```

- **Response:**

```yaml
RecordRunResponse:
  base: ToolResponseBase
  run_summary: RunSummary
  registered_artifacts: ArtifactRef[]
  evidence_summary: EvidenceSummary | null
  blocker_refs: StateRecordRef[]
  state: StateSummary
```

- **State effect:** A compatible committed call may create `runs`, `artifacts`, `artifact_links`, and `evidence_summaries`, update blockers, consume `write_authorizations.status=active`, append events, and create a committed replay row. Rejected calls must not create a Run, register artifacts, update evidence, or consume an invalid authorization.
- **Errors:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID`, `SCOPE_VIOLATION`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `EVIDENCE_INSUFFICIENT`, `VALIDATOR_FAILED`.
- **Storage owner:** `runs`, `write_authorizations`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers`, `task_events`, and `tool_invocations`.
- **Security boundary:** A run can record what the surface observed. If a surface cannot observe paths, commands, network, secret access, artifact capture, blocking, or isolation facts, the API must not mark those facts verified.

<a id="harnessrequest_user_judgment"></a>

## `harness.request_user_judgment`

- **Owns:** Creation of a pending `UserJudgment` for one focused user-owned decision.
- **Does not own:** The user's answer, sensitive-action permission, Write Authorization, evidence, final acceptance, residual-risk acceptance, or close.
- **When to call:** When progress, write compatibility, acceptance, risk handling, or close depends on a user-owned judgment that cannot be inferred from existing records.
- **Request:**

```yaml
RequestUserJudgmentRequest:
  envelope: ToolEnvelope
  task_id: string | null
  change_unit_id: string | null
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | final_acceptance | residual_risk_acceptance | cancellation
  presentation: short
  question: string
  options: UserJudgmentOption[]
  context: UserJudgmentContext
  affected_refs: StateRecordRef[]
  required_for: next_action | write | run | close | acceptance | risk
  expires_at: string | null
```

- **Response:**

```yaml
RequestUserJudgmentResponse:
  base: ToolResponseBase
  user_judgment_ref: StateRecordRef
  user_judgment: UserJudgment
  blocker_refs: StateRecordRef[]
  state: StateSummary
```

- **State effect:** A committed non-dry-run call creates one pending `user_judgments` row, may link or update affected blockers, appends an event, and creates a replay row. A candidate returned by another method is not a pending judgment until this method commits.
- **Errors:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`, `VALIDATOR_FAILED`.
- **Storage owner:** `user_judgments`, `blockers`, `task_events`, and `tool_invocations`.
- **Security boundary:** The request presents a question. It grants no permission and resolves no gate until `harness.record_user_judgment` records a matching answer.

<a id="harnessrecord_user_judgment"></a>

## `harness.record_user_judgment`

- **Owns:** Resolution, rejection, deferral, or blocking of an existing pending `UserJudgment`.
- **Does not own:** A broader decision than the pending `judgment_kind`, product writes, evidence, Write Authorization, close, or any judgment not explicitly asked.
- **When to call:** After the user answers a specific pending `UserJudgment`.
- **Request:**

```yaml
RecordUserJudgmentRequest:
  envelope: ToolEnvelope
  user_judgment_id: string
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | final_acceptance | residual_risk_acceptance | cancellation
  selected_option_id: string
  answer: RecordUserJudgmentPayload
  note: string | null
  accepted_risks: AcceptedRiskInput[]
```

- **Response:**

```yaml
RecordUserJudgmentResponse:
  base: ToolResponseBase
  user_judgment_ref: StateRecordRef
  user_judgment: UserJudgment
  updated_refs: StateRecordRef[]
  state: StateSummary
```

- **State effect:** A committed non-dry-run call updates `user_judgments.status`, records the answer, updates only covered blockers and affected state, appends an event, and creates a replay row. It creates no standalone accepted-risk row in active MVP.
- **Errors:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_UNRESOLVED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `VALIDATOR_FAILED`.
- **Storage owner:** `user_judgments`, `blockers`, affected `tasks` or `change_units`, `task_events`, and `tool_invocations`.
- **Security boundary:** Broad phrases such as "go ahead" or "looks good" do not become product decisions, sensitive-action approval, final acceptance, residual-risk acceptance, cancellation, scope expansion, or later/reserved QA waiver and verification-risk routes unless the pending judgment explicitly asked for that kind and the recorded answer matches it.

<a id="harnessclose_task"></a>

## `harness.close_task`

- **Owns:** Active close-readiness check and terminal Task close/cancel/supersede when blockers allow it.
- **Does not own:** Evidence creation, user judgment creation, final acceptance creation, residual-risk acceptance creation, export, release handoff, projection/report freshness, or implementation validation beyond active blockers.
- **When to call:** When the caller needs to know whether work can close, or when the user intends to complete, cancel, or supersede the active Task.
- **Request:**

```yaml
CloseTaskRequest:
  envelope: ToolEnvelope
  task_id: string
  intent: check | complete | cancel | supersede
  close_reason: completed_self_checked | completed_with_risk_accepted | cancelled | superseded | null
  superseding_task_id: string | null
  user_note: string | null
```

- **Response:**

```yaml
CloseTaskResponse:
  base: ToolResponseBase
  close_state: ready | blocked | closed | cancelled | superseded
  state: StateSummary
  blockers: CloseBlocker[]
  evidence_summary: EvidenceSummary | null
  artifact_refs: ArtifactRef[]
  next_actions: NextActionSummary[]
```

- **State effect:** `intent=check` is read-only. A committed non-dry-run terminal close updates `tasks.lifecycle_phase`, `tasks.result`, `tasks.closed_at`, affected `change_units`, blockers, project active-task state when needed, events, and replay. A blocked close may record blockers but must leave the Task open. Dry-run creates no close state or replay row.
- **Errors:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `EVIDENCE_INSUFFICIENT`, `ARTIFACT_MISSING`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `VALIDATOR_FAILED`.
- **Storage owner:** `tasks`, `change_units`, `blockers`, `runs`, `evidence_summaries`, `artifacts`, `artifact_links`, `user_judgments`, `task_events`, and `tool_invocations`.
- **Security boundary:** Close is a Core state transition, not a report. It cannot be inferred from chat, status text, final acceptance alone, residual-risk acceptance alone, evidence alone, or a rendered view.
