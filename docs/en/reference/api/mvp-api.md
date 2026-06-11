# Active MVP API

## What this document helps you do

Use this reference to look up the active current MVP API surface. It owns method-level request, response, state effect, storage owner, error, and security boundary summaries for the active method-name value set owned by [API Value Sets](schema-value-sets.md).

This file currently owns all active MVP method behavior. Method-specific owner documents should be created when the [Active MVP API method split threshold](../../maintain/authoring-guide.md#active-mvp-api-method-split-threshold) is met.

This document describes future Harness Server behavior for planning and review. No Harness runtime or server implementation exists in this repository today. Future API or schema candidates are cataloged in [Later Candidate Index](../../later/index.md), not in this active reference. Storage DDL and full shared schema bodies are owned outside this method reference.

## Main idea

The active MVP API is a small local MCP surface for one user work loop. It can intake work, show status, update active scope, check proposed product writes against current Core state, record runs and evidence refs, ask and record user-owned judgment, and close only when active blockers allow it.

The API returns cooperative Harness record/check behavior only. Security non-claims and guarantee wording belong to [Security](../security.md).

The specification requires requirement shaping to use the active Task, Change Unit, `user_judgment`, evidence summary, blocker paths, next actions, and the derived `ShapingReadiness` view.

The API must not introduce separate active committed planning artifacts to move from a vague request to a safe first Change Unit, including:

- Discovery Brief
- Question Queue
- Assumption Register
- similar committed planning artifacts

<a id="active-mvp-method-behavior"></a>

## Active MVP method behavior

The exact active method-name value set is owned by [API Value Sets](schema-value-sets.md). This page owns the behavior of those current methods:

| Method | Active role |
|---|---|
| [`harness.intake`](#harnessintake) | Start, resume, or classify ordinary user work. |
| [`harness.status`](#harnessstatus) | Return current state summary, blockers, pending judgments, evidence summary, close state, and next safe actions. |
| [`harness.update_scope`](#harnessupdate_scope) | Update active Task scope and the active Change Unit after intake. |
| [`harness.prepare_write`](#harnessprepare_write) | Check a proposed product-file write against current scope, state, required separate sensitive-action permission, baseline, and surface capability. |
| [`harness.stage_artifact`](#harnessstage_artifact) | Stage caller-provided safe artifact bytes or a safe notice as a temporary handle for later `record_run` promotion. |
| [`harness.record_run`](#harnessrecord_run) | Record shaping, direct, or implementation work plus compact evidence and artifact refs. |
| [`harness.request_user_judgment`](#harnessrequest_user_judgment) | Create one pending user-owned judgment request. |
| [`harness.record_user_judgment`](#harnessrecord_user_judgment) | Record the user's answer to an existing pending `UserJudgment`. |
| [`harness.close_task`](#harnessclose_task) | Check close readiness and close, cancel, or supersede only when blockers allow it. |

This page names the method role and method-specific result behavior. For the canonical branch, storage-effect, `dry_run`, replay, and state-version rules, see [API Schema Core](schema-core.md), [Storage Effects](../storage-effects.md), and [Storage Versioning](../storage-versioning.md).

<a id="shared-request-rules"></a>

## Shared request rules

All methods use [`ToolEnvelope`](schema-core.md#tool-envelope). Each public method response is exactly one response branch: the concrete method-specific `MethodResult`, `ToolRejectedResponse`, or `ToolDryRunResponse`. The method result schema names the concrete result for actual read results, successful staging results, Core committed results, or committed blocked results when the method state-effect table allows that blocked commit. Method results use [`ToolResultBase`](schema-core.md#common-response) with `response_kind=result`; `ToolRejectedResponse` and `ToolDryRunResponse` use the shared response schemas from [common response branches](schema-core.md#common-response) and do not inherit method-specific result-only fields.

Examples below are compact branch examples, not full schema definitions. Minimal request examples include the fields needed to construct a valid call for that method. Representative response examples show branch-critical fields and may omit schema-owned nested fields that do not affect the behavior being illustrated; use the linked schema owners for full schema shapes.

Committed non-dry-run state-changing calls require a non-null `idempotency_key` and a current project-wide `expected_state_version`; read-only calls, valid dry-run previews, and staging utility calls use the exceptions defined by their owners.

Response branch selection is owned by [common response branches](schema-core.md#common-response). Storage and replay effects are owned by [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md). Public errors, stale-state precedence, and close blocker routing are owned by [API Errors](errors.md).

When a method has a tool-specific `task_id`, Core resolves the primary Task from the method field before `ToolEnvelope.task_id` and then the active Task. That resolution selects owner records; it does not create a separate state clock.

Local access classes are Harness API compatibility classes, not OS permission classes. Active `access_class` values are owned by [access class values](schema-value-sets.md#access-class-values); connector derivation and capability posture are owned by [Agent Integration](../agent-integration.md) and [Security](../security.md).

Each public API request has exactly one request-level access class. Nested payloads such as `ArtifactInput[]` do not add a second access class; artifact staging, promotion, and body-read boundaries are owned by [API Artifact Schemas](schema-artifacts.md) and [Artifact Storage](../storage-artifacts.md).

<a id="harnessintake"></a>

## `harness.intake`

### Purpose

Start, resume, supersede, or reject an ordinary user work loop and resolve the requested mode to a concrete `advisor`, `direct`, or `work` Task state. `harness.intake` may create the first scope candidate for write-capable work, but later scope changes belong to `harness.update_scope`.

### Required inputs

- `ToolEnvelope` with `project_id`, `surface_id`, `request_id`, `dry_run`, and, for non-dry-run commits, non-null `idempotency_key` and current `expected_state_version`.
- `user_request`, `requested_mode`, and `resume_policy`.
- Any known `acceptance_criteria`, `constraints.allowed_paths`, `constraints.non_goals`, `constraints.sensitive_categories`, and `initial_context_refs`; use empty arrays when none are known.

### Access requirements

Requires `VerifiedSurfaceContext.access_class=core_mutation` and `verified=true` for a non-dry-run commit. The `surface_id` selects a registered local surface; it is not itself authority.

### State version behavior

A committed non-dry-run result increments project-wide `project_state.state_version` exactly once and creates the replay row for the idempotency key. A dry run, read failure, validation failure, local access failure, or stale `expected_state_version` creates no Task, Change Unit, event, replay row, blocker update, or state-version increment.

### Success result

Returns `IntakeResult` with `base.response_kind=result` and `base.effect_kind=core_committed`. The result includes `task_ref`, optional `change_unit_ref`, current `state`, and `next_actions`. If `requested_mode=auto`, the persisted and displayed mode must be the resolved concrete mode, never `auto`.

### Blocked result

The method may return a committed `IntakeResult` that records shaping or blocker state instead of a write-ready path. Blocking questions must be represented through Task, Change Unit, user-judgment, evidence, blocker, or next-action fields rather than through separate Discovery Brief, Question Queue, or Assumption Register artifacts.

### Rejected result

Returns `ToolRejectedResponse` for pre-commit failures such as validation failure, stale `expected_state_version`, unavailable Core or local surface, local access mismatch, missing active-task compatibility, or validator failure. Public error code meaning and precedence are owned by [API Errors](errors.md).

### Dry-run behavior

For `dry_run=true`, a valid state-effecting preview returns `ToolDryRunResponse`, not `IntakeResult`. Branch shape is owned by [API Schema Core](schema-core.md); no-effect persistence semantics are owned by [Storage Effects](../storage-effects.md).

### Storage effect

On commit, the method may persist intake-owned Task or Change Unit state. Exact storage effects are owned by [Storage Effects](../storage-effects.md), and storage record shapes are owned by [Storage Records](../storage-records.md).

### Minimal valid request

```yaml
method: harness.intake
params:
  envelope:
    project_id: proj_123
    task_id: null
    actor_kind: agent
    surface_id: surface_local
    request_id: req_intake_001
    idempotency_key: idem_intake_001
    expected_state_version: 17
    dry_run: false
    locale: en-US
  user_request: "Update the API reference examples for the MVP docs."
  requested_mode: auto
  resume_policy: create_new
  acceptance_criteria:
    - "Each active method has a minimal request and representative response."
  constraints:
    allowed_paths:
      - docs/en/reference/api/mvp-api.md
    non_goals:
      - "Runtime implementation"
    sensitive_categories: []
  initial_context_refs: []
```

### Representative response

Result branch (`IntakeResult`, committed):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 18
  events:
    - event_id: evt_1001
      event_kind: task_intake
task_ref:
  record_kind: task
  record_id: task_456
  project_id: proj_123
  task_id: task_456
  state_version: 18
change_unit_ref: null
state:
  project_id: proj_123
  state_version: 18
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 18
  mode: work
  lifecycle:
    lifecycle_phase: shaping
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Update MVP API method examples."
  scope_summary: null
  active_change_unit_ref: null
  blocker_refs: []
next_actions:
  - action: harness.update_scope
    reason: "Create the first active Change Unit before write checking."
```

### Owner links

- Request envelope and response branches: [`ToolEnvelope`](schema-core.md#tool-envelope) and [common response branches](schema-core.md#common-response).
- State refs, `StateSummary`, `ShapingReadiness`, and next actions: [API State Schemas](schema-state.md).
- Active method names, mode values, `resume_policy`, `response_kind`, `effect_kind`, and access classes: [API Value Sets](schema-value-sets.md).
- Public errors and state-version conflicts: [API Errors](errors.md).
- Persistence effects: [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).

<a id="harnessupdate_scope"></a>

## `harness.update_scope`

### Purpose

Update the active Task's goal summary, scope boundary, non-goals, acceptance criteria, autonomy boundary, baseline reference, and active Change Unit after intake. This is the active path that turns shaping into a first safe Change Unit when user-owned blockers have been handled.

### Required inputs

- `ToolEnvelope` with non-null `idempotency_key` and current `expected_state_version` for non-dry-run commits.
- `task_id`.
- Any top-level scope fields to change. `null` means leave the current value unchanged; an empty array replaces that list with an empty list.
- `change_unit.operation` and the fields needed by that operation.
- `related_scope_decision_refs` when the update applies a resolved `judgment_kind=scope_decision`.

### Access requirements

Requires `VerifiedSurfaceContext.access_class=core_mutation` and `verified=true` for a non-dry-run commit. The request must identify a compatible same-project Task and, when creating or replacing an active Change Unit, enough scope to make the next safe action honest.

### State version behavior

A committed non-dry-run result increments `project_state.state_version` exactly once. If the update makes any active Write Authorization stale because scope, baseline, acceptance criteria, non-goals, autonomy boundary, Change Unit, or project state no longer matches its basis, Core marks it `status=stale`; it does not consume, revoke, expire, or silently reuse it.

### Success result

Returns `UpdateScopeResult` with `base.response_kind=result` and `base.effect_kind=core_committed`. The result includes `task_ref`, optional `change_unit_ref`, linked scope-decision refs, stale Write Authorization refs, blocker refs, current `state`, and `next_actions`.

### Blocked result

The method may commit method-owned blocker or current-row updates when scope is still not ready. A committed blocked scope result must identify the missing user-owned judgment category, such as `product_decision`, `technical_decision`, `scope_decision`, or `sensitive_approval`, rather than hiding it behind vague ambiguity.

### Rejected result

Returns `ToolRejectedResponse` for pre-commit failures such as stale `expected_state_version`, invalid Task identity, invalid Change Unit operation, missing required scope, scope violation, unresolved required decision, autonomy-boundary violation, stale baseline, local access failure, or validator failure. Public error code meaning and precedence are owned by [API Errors](errors.md).

### Dry-run behavior

For `dry_run=true`, a valid state-effecting preview returns `ToolDryRunResponse`. Branch shape is owned by [API Schema Core](schema-core.md); no-effect persistence semantics are owned by [Storage Effects](../storage-effects.md).

### Storage effect

On commit, the method may persist scope-owned current state and stale-authorization consequences. Exact storage effects are owned by [Storage Effects](../storage-effects.md).

### Minimal valid request

```yaml
method: harness.update_scope
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_scope_001
    idempotency_key: idem_scope_001
    expected_state_version: 18
    dry_run: false
    locale: en-US
  task_id: task_456
  goal_summary: "Restructure active MVP API method reference sections."
  scope_boundary: "Only docs/en/reference/api/mvp-api.md and docs/ko/reference/api/mvp-api.md."
  non_goals:
    - "Implementing runtime API code"
  acceptance_criteria:
    - "Every active method follows the standard section pattern."
  autonomy_boundary: "Documentation-only edits."
  baseline_ref: baseline_docs_2026_06_10
  change_unit:
    operation: create_active
    scope_summary: "Replace method bodies with uniform reference sections."
    affected_areas:
      - "API reference docs"
    affected_paths:
      - docs/en/reference/api/mvp-api.md
      - docs/ko/reference/api/mvp-api.md
    constraints:
      - "Preserve method identifiers and owner links."
  related_scope_decision_refs: []
```

### Representative response

Result branch (`UpdateScopeResult`, committed):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 19
  events:
    - event_id: evt_1002
      event_kind: scope_updated
task_ref:
  record_kind: task
  record_id: task_456
  project_id: proj_123
  task_id: task_456
  state_version: 19
change_unit_ref:
  record_kind: change_unit
  record_id: cu_001
  project_id: proj_123
  task_id: task_456
  state_version: 19
linked_scope_decision_refs: []
stale_write_authorization_refs: []
blocker_refs: []
state:
  project_id: proj_123
  state_version: 19
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 19
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Restructure active MVP API method reference sections."
  scope_summary: "Only paired MVP API reference docs."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_001
    project_id: proj_123
    task_id: task_456
    state_version: 19
next_actions:
  - action: harness.prepare_write
    reason: "Check the first documentation write against active scope."
```

### Owner links

- Request envelope and response branches: [API Schema Core](schema-core.md).
- State refs, `StateSummary`, `ShapingReadiness`, blockers, and next actions: [API State Schemas](schema-state.md).
- Scope-related user judgment shapes: [API Judgment Schemas](schema-judgment.md).
- Active value sets and access classes: [API Value Sets](schema-value-sets.md).
- Public errors: [API Errors](errors.md).
- Persistence effects and stale authorization behavior: [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).

<a id="harnessstatus"></a>

## `harness.status`

### Purpose

Return a read-only current-position view over Core state: active Task summary, blockers, pending user judgments, Write Authorization summary, evidence summary, close state, close-readiness findings, guarantee display, and next safe actions.

### Required inputs

- `ToolEnvelope` with `project_id`, `surface_id`, `request_id`, and `dry_run`; `idempotency_key` and `expected_state_version` may be `null`.
- `include` flags selecting which summaries the caller needs.

### Access requirements

Requires a same-project active local surface with `VerifiedSurfaceContext.access_class=read_status` when protected Core detail is returned. A stale projection, chat summary, generated Markdown file, or cached text is not state authority.

### State version behavior

No state change occurs and `project_state.state_version` never increments. The result may report the current observed state version, but the method creates no event, replay row, close mutation, artifact effect, staged-handle consumption, evidence update, or Write Authorization change.

### Success result

Returns `StatusResult` with `base.response_kind=result` and `base.effect_kind=read_only`. `StatusResult.close_blockers` are read-only `CloseReadinessBlocker[]` observations when `include.close=true`; they are not stored close results.

### Blocked result

There is no committed blocked branch. Blockers and close blockers in a `StatusResult` are computed response fields only.

### Rejected result

Returns `ToolRejectedResponse` only when the read cannot be safely served, such as unavailable Core, local access mismatch, insufficient capability for the requested protected detail, missing active Task for a Task-scoped read, or stale/unavailable projection when such a view was requested. Public error code meaning and precedence are owned by [API Errors](errors.md).

### Dry-run behavior

`dry_run=true` does not create a `ToolDryRunResponse` branch for this read-only method. A valid request returns the same `StatusResult` shape with `base.dry_run=true` and `base.effect_kind=read_only`; branch rules are owned by [API Schema Core](schema-core.md).

### Storage effect

This is a read-only method. Exact no-effect persistence semantics are owned by [Storage Effects](../storage-effects.md).

### Minimal valid request

```yaml
method: harness.status
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_status_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: en-US
  include:
    task: true
    pending_user_judgments: true
    write_authority: true
    evidence: true
    close: true
    guarantees: true
```

### Representative response

Result branch (`StatusResult`, read-only):

```yaml
base:
  response_kind: result
  effect_kind: read_only
  dry_run: false
  state_version: 19
  events: []
active_task:
  project_id: proj_123
  state_version: 19
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 19
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Restructure active MVP API method reference sections."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_001
    project_id: proj_123
    task_id: task_456
    state_version: 19
status_card: "Task is ready for pre-write checking."
next_actions:
  - action: harness.prepare_write
    reason: "A product-file documentation edit is next."
pending_user_judgments: []
write_authority_summary: null
evidence_summary: null
blocker_refs: []
close_state: blocked
close_blockers:
  - category: evidence
    code: EVIDENCE_INSUFFICIENT
    message: "No run evidence has been recorded yet."
    related_refs: []
guarantee_display:
  level: cooperative
  notes:
    - "No stronger local guarantee is active."
```

### Owner links

- Request envelope and response branches: [API Schema Core](schema-core.md).
- Status state, close-readiness shapes, evidence summaries, and guarantee display: [API State Schemas](schema-state.md).
- Active values and access classes: [API Value Sets](schema-value-sets.md).
- Public errors and close blocker routing: [API Errors](errors.md) and [`close_task` blocker mapping](errors.md#harnessclose_task-close-blockers).
- Persistence effects: [Storage Effects](../storage-effects.md).

<a id="harnessprepare_write"></a>

## `harness.prepare_write`

### Purpose

Check one proposed product-file write against current Task, active Change Unit, scope, baseline, required separate sensitive-action approval, and verified local surface capability. When the check is allowed, it creates a consumable single-use Write Authorization. When it is not allowed, it denies or defers that Write Authorization path. Security non-claims belong to [Security](../security.md).

### Required inputs

- `ToolEnvelope` with non-null `idempotency_key` and current `expected_state_version` for non-dry-run commits.
- `task_id` and `change_unit_id`, or `null` only when owner resolution can unambiguously use the active Task and active Change Unit.
- `intended_operation`, `intended_paths`, `product_file_write_intended`, `sensitive_categories`, and `baseline_ref`.

### Access requirements

Requires `VerifiedSurfaceContext.access_class=write_authorization` and `verified=true`. The method also requires compatible active scope, baseline, required user-owned judgments, any separate `sensitive_approval`, and the local surface capability needed for the intended product-file write check.

### State version behavior

A committed `decision=allowed` increments `project_state.state_version` exactly once and creates exactly one active Write Authorization for the path-level `AuthorizedAttemptScope`. A committed `decision=blocked`, `decision=approval_required`, or `decision=decision_required` may increment the state version only to persist method-owned write-decision reason state; it must not create a consumable Write Authorization. Pre-commit rejection and dry run increment nothing.

### Success result

Returns `PrepareWriteResult` with `base.response_kind=result` and `base.effect_kind=core_committed`. For `decision=allowed`, `write_authorization_ref` and `write_authorization` are non-null and `authorization_effect` is `created` or `returned` for an idempotent replay.

### Blocked result

Committed blocked decisions are `PrepareWriteResult` values with `decision=blocked`, `decision=approval_required`, or `decision=decision_required`. `write_decision_reasons` must be non-empty. These reasons are not `CloseReadinessBlocker` values and do not evaluate close readiness. No consumable Write Authorization is created.

### Rejected result

Returns `ToolRejectedResponse` for failures before decision evaluation or commit, including stale `expected_state_version`, idempotency request-hash conflict, request validation failure, missing active Task or Change Unit, local access failure, Core unavailability, stale baseline, invalid requested guarantee, or capability failure. `STATE_VERSION_CONFLICT` is always a rejected response error, never a write decision reason.

### Dry-run behavior

For `dry_run=true`, a valid preview returns `ToolDryRunResponse`. Branch shape is owned by [API Schema Core](schema-core.md); no-effect persistence semantics are owned by [Storage Effects](../storage-effects.md).

### Storage effect

On commit, the method may persist Write Authorization or write-decision state according to the method result. Exact storage effects are owned by [Storage Effects](../storage-effects.md).

### Minimal valid request

```yaml
method: harness.prepare_write
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_prepare_001
    idempotency_key: idem_prepare_001
    expected_state_version: 19
    dry_run: false
    locale: en-US
  task_id: task_456
  change_unit_id: cu_001
  intended_operation: "replace method reference sections"
  intended_paths:
    - docs/en/reference/api/mvp-api.md
    - docs/ko/reference/api/mvp-api.md
  product_file_write_intended: true
  sensitive_categories: []
  baseline_ref: baseline_docs_2026_06_10
```

### Representative response

Result branch (`PrepareWriteResult`, `decision=allowed`):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 20
  events:
    - event_id: evt_1003
      event_kind: write_authorization_created
decision: allowed
state:
  project_id: proj_123
  state_version: 20
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 20
write_authorization_ref:
  record_kind: write_authorization
  record_id: wa_001
  project_id: proj_123
  task_id: task_456
  state_version: 20
write_authorization:
  authorization_id: wa_001
  status: active
  basis_state_version: 19
  authorized_paths:
    - docs/en/reference/api/mvp-api.md
    - docs/ko/reference/api/mvp-api.md
authorization_effect: created
active_user_judgment_refs: []
write_decision_reasons: []
user_judgment_candidate: null
guarantee_display:
  level: cooperative
  notes:
    - "Write Authorization is a Harness compatibility record, not OS permission."
```

### Owner links

- Request envelope, common result branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `WriteAuthorizationSummary`, state summaries, and refs: [API State Schemas](schema-state.md).
- `SensitiveActionScope` and user-owned approval boundaries: [API Judgment Schemas](schema-judgment.md).
- Active values and access classes: [API Value Sets](schema-value-sets.md).
- Public errors, `STATE_VERSION_CONFLICT`, and blocked/dry-run behavior: [API Errors](errors.md).
- Persistence effects and state clocks: [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).

<a id="harnessstage_artifact"></a>

## `harness.stage_artifact`

### Purpose

Stage caller-provided safe artifact bytes or a safe notice into a temporary `StagedArtifactHandle` for the same project and Task. Staging is input preparation only; it does not create canonical evidence, persistent `ArtifactRef`, gate satisfaction, final acceptance, residual-risk acceptance, or close readiness.

### Required inputs

- `ToolEnvelope` with `project_id`, `task_id`, `surface_id`, `request_id`, and `dry_run`; `idempotency_key` and `expected_state_version` may be `null`.
- `task_id`, `display_name`, `content_type`, `redaction_state`, `safe_bytes_or_notice`, `expected_sha256`, `expected_size_bytes`, and `relation_hint`.

### Access requirements

Requires `VerifiedSurfaceContext.access_class=artifact_registration`, `verified=true`, compatible `project_id` and `task_id`, and `manual_artifact_attachment_supported=true`. A future server records `created_by_surface_id` and `created_by_surface_instance_id` from the verified local surface; the caller does not provide those as authority.

### State version behavior

A successful staging result does not change Core state and does not increment `project_state.state_version`. It also creates no `tool_invocations` replay row. Rejected and dry-run requests have no storage effect.

### Success result

Returns `StageArtifactResult` with `base.response_kind=result` and `base.effect_kind=staging_created`. The result contains a temporary `staged_artifact_handle` and `expires_at`; it does not contain a persistent `ArtifactRef`.

### Blocked result

There is no committed blocked branch. Invalid staging requests are rejected before any Core mutation. Staging availability or capability problems do not create blockers.

### Rejected result

Returns `ToolRejectedResponse` for invalid request shape, checksum or size mismatch, unsafe artifact input, unsupported redaction state, unavailable Core or local surface, local access mismatch, or insufficient artifact registration capability. Public error code meaning and precedence are owned by [API Errors](errors.md).

### Dry-run behavior

For `dry_run=true`, a valid staging preview returns `ToolDryRunResponse`, not `StageArtifactResult`. Branch shape is owned by [API Schema Core](schema-core.md); no-effect staging semantics are owned by [Storage Effects](../storage-effects.md) and [Artifact Storage](../storage-artifacts.md).

### Storage effect

On success, the method creates a temporary staging result only. Exact storage effects are owned by [Storage Effects](../storage-effects.md), and artifact lifecycle details are owned by [Artifact Storage](../storage-artifacts.md).

### Minimal valid request

```yaml
method: harness.stage_artifact
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_stage_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: en-US
  task_id: task_456
  display_name: "Documentation check summary"
  content_type: text/plain
  redaction_state: none
  safe_bytes_or_notice: "No runtime code was changed."
  expected_sha256: null
  expected_size_bytes: null
  relation_hint: "run_note"
```

### Representative response

Result branch (`StageArtifactResult`, staging created):

```yaml
base:
  response_kind: result
  effect_kind: staging_created
  dry_run: false
  state_version: null
  events: []
staged_artifact_handle:
  handle_id: sah_001
  project_id: proj_123
  task_id: task_456
  created_by_surface_id: surface_local
  created_by_surface_instance_id: surface_instance_01
  content_type: text/plain
  sha256: sha256:example
  size_bytes: 28
  redaction_state: none
  expires_at: "2026-06-10T12:30:00Z"
  consumed: false
expires_at: "2026-06-10T12:30:00Z"
```

### Owner links

- Request envelope, response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `StagedArtifactHandle`, `ArtifactInput`, and `ArtifactRef`: [API Artifact Schemas](schema-artifacts.md).
- Active artifact values and access classes: [API Value Sets](schema-value-sets.md).
- Public errors: [API Errors](errors.md).
- Persistence effects and artifact lifecycle: [Storage Effects](../storage-effects.md) and [Artifact Storage](../storage-artifacts.md).

<a id="harnessrecord_run"></a>

## `harness.record_run`

### Purpose

Record shaping work, a direct answer/result, or implementation work; update compact evidence coverage; consume a compatible Write Authorization when recording a product write; link existing artifacts; and promote eligible staged handles to persistent `ArtifactRef` records where allowed.

### Required inputs

- `ToolEnvelope` with non-null `idempotency_key` and current `expected_state_version` for non-dry-run commits.
- `task_id`, `change_unit_id`, `kind`, `run_id`, `baseline_ref`, `write_authorization_id`, `summary`, `observed_changes`, `artifact_inputs`, and `evidence_updates`.
- Product-write runs require a compatible active Write Authorization from `harness.prepare_write`.
- New artifact bytes must already be represented by a valid `StagedArtifactHandle`; `record_run` does not stage new bytes.

### Access requirements

Requires `VerifiedSurfaceContext.access_class=run_recording` and `verified=true`. `ArtifactInput[]` does not add `artifact_registration`. For `source_kind=staged_artifact`, the current verified `surface_id` and `surface_instance_id` must match the staged handle's recorded provenance; the active MVP has no cross-surface staged artifact handoff.

### State version behavior

A compatible committed result increments `project_state.state_version` exactly once. Product-write recording consumes the active Write Authorization only when the current state version still matches the authorization basis and observed changed paths are compatible with the authorized attempt. A stale `expected_state_version` or stale authorization basis is rejected before consumption.

### Success result

Returns `RecordRunResult` with `base.response_kind=result` and `base.effect_kind=core_committed`. The result includes `run_summary`, any `registered_artifacts`, updated `evidence_summary`, `blocker_refs`, and current `state`.

### Blocked result

The method may commit compatible run-related blocker state when the run is recordable but the result creates or preserves blockers, such as evidence gaps. It must not use a committed blocked result to hide invalid staged handles, missing Write Authorization, stale state, stale authorization basis, or local access failures; those are rejected before commit.

### Rejected result

Returns `ToolRejectedResponse` for stale `expected_state_version`, stale Write Authorization basis, missing or invalid Write Authorization for product writes, invalid staged handle, incompatible staged-handle provenance, missing artifact, scope violation, baseline staleness, local access failure, insufficient capability, or validator failure. Invalid staged handles are validation failures with artifact-input details, not local access mismatch unless request-level local access itself failed.

### Dry-run behavior

For `dry_run=true`, a valid preview returns `ToolDryRunResponse`. Branch shape is owned by [API Schema Core](schema-core.md); no-effect persistence and promotion semantics are owned by [Storage Effects](../storage-effects.md) and [Artifact Storage](../storage-artifacts.md).

### Storage effect

On commit, the method may persist run, evidence, blocker, authorization-consumption, and artifact-linking results. Exact storage effects are owned by [Storage Effects](../storage-effects.md), and artifact promotion details are owned by [Artifact Storage](../storage-artifacts.md).

### Minimal valid request

```yaml
method: harness.record_run
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_run_001
    idempotency_key: idem_run_001
    expected_state_version: 20
    dry_run: false
    locale: en-US
  task_id: task_456
  change_unit_id: cu_001
  kind: implementation
  run_id: null
  baseline_ref: baseline_docs_2026_06_10
  write_authorization_id: wa_001
  summary: "Replaced method sections with the standard API reference pattern."
  observed_changes:
    changed_paths:
      - docs/en/reference/api/mvp-api.md
      - docs/ko/reference/api/mvp-api.md
    product_file_write_observed: true
    sensitive_categories: []
    baseline_ref: baseline_docs_2026_06_10
  artifact_inputs: []
  evidence_updates:
    - claim: "Each active method follows the standard section pattern."
      required_for_close: true
      coverage_state: supported
      supporting_refs: []
      supporting_artifact_refs: []
      gap_refs: []
```

### Representative response

Result branch (`RecordRunResult`, committed):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 21
  events:
    - event_id: evt_1004
      event_kind: run_recorded
run_summary:
  run_ref:
    record_kind: run
    record_id: run_001
    project_id: proj_123
    task_id: task_456
    state_version: 21
  kind: implementation
  summary: "Replaced method sections with the standard API reference pattern."
  observed_changes:
    changed_paths:
      - docs/en/reference/api/mvp-api.md
      - docs/ko/reference/api/mvp-api.md
    product_file_write_observed: true
    sensitive_categories: []
    baseline_ref: baseline_docs_2026_06_10
  artifact_refs: []
registered_artifacts: []
evidence_summary:
  status: sufficient
  coverage_items:
    - claim: "Each active method follows the standard section pattern."
      required_for_close: true
      coverage_state: supported
      supporting_refs:
        - record_kind: run
          record_id: run_001
          project_id: proj_123
          task_id: task_456
          state_version: 21
      supporting_artifact_refs: []
      gap_refs: []
  artifact_refs: []
blocker_refs: []
state:
  project_id: proj_123
  state_version: 21
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 21
```

### Owner links

- Request envelope, response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `RunSummary`, `EvidenceSummary`, `EvidenceCoverageItem`, `StateSummary`, and refs: [API State Schemas](schema-state.md).
- `ArtifactInput`, `StagedArtifactHandle`, and `ArtifactRef`: [API Artifact Schemas](schema-artifacts.md).
- Write Authorization and close-relevant evidence boundaries: [Core Model](../core-model.md).
- Active values and access classes: [API Value Sets](schema-value-sets.md).
- Public errors: [API Errors](errors.md).
- Persistence effects and artifact promotion: [Storage Effects](../storage-effects.md) and [Artifact Storage](../storage-artifacts.md).

<a id="harnessrequest_user_judgment"></a>

## `harness.request_user_judgment`

### Purpose

Create one pending `UserJudgment` for a focused user-owned decision. The method asks the user; the agent must not answer, infer, broaden, or decide the judgment for the user.

### Required inputs

- `ToolEnvelope` with non-null `idempotency_key` and current `expected_state_version` for non-dry-run commits.
- `task_id`, `change_unit_id`, `judgment_kind`, `presentation`, `question`, `options`, `context`, `affected_refs`, `required_for`, and `expires_at`.
- A focused question with mutually understandable options and enough context for the user to judge the exact issue.

### Access requirements

Requires `VerifiedSurfaceContext.access_class=core_mutation` and `verified=true`. The request must target a compatible same-project Task and optional Change Unit.

### State version behavior

A committed non-dry-run result increments `project_state.state_version` exactly once and creates the pending judgment. A candidate returned by another method is not durable until this method commits. Dry run and rejection create no pending judgment, blocker update, event, replay row, or state-version increment.

### Success result

Returns `RequestUserJudgmentResult` with `base.response_kind=result` and `base.effect_kind=core_committed`. The result includes `user_judgment_ref`, the pending `user_judgment`, affected `blocker_refs`, and current `state`.

### Blocked result

There is no separate committed blocked response branch. If a judgment cannot be created because the request is invalid or prerequisites cannot be verified, the method rejects before commit.

### Rejected result

Returns `ToolRejectedResponse` for invalid question shape, invalid `judgment_kind`, missing Task, unresolved prerequisite decision, local access failure, insufficient capability, stale `expected_state_version`, or validator failure. Public error code meaning and precedence are owned by [API Errors](errors.md).

### Dry-run behavior

For `dry_run=true`, a valid preview returns `ToolDryRunResponse`. Branch shape is owned by [API Schema Core](schema-core.md); no-effect persistence semantics are owned by [Storage Effects](../storage-effects.md).

### Storage effect

On commit, the method may persist pending judgment and related blocker state. Exact storage effects are owned by [Storage Effects](../storage-effects.md).

### Minimal valid request

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
  judgment_kind: final_acceptance
  presentation: short
  question: "Do you accept the visible result basis?"
  options:
    - option_id: accept
      label: "Accept"
      description: "Record final acceptance for this Task."
      consequence: "Close readiness can evaluate final acceptance as satisfied."
      is_default: true
    - option_id: revise
      label: "Revise"
      description: "Keep the Task open for more changes."
      consequence: "Close remains blocked on final acceptance."
      is_default: false
  context:
    summary: "The requested changes are ready for final acceptance."
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

### Representative response

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
  judgment_kind: final_acceptance
  status: pending
  presentation: short
  question: "Do you accept the visible result basis?"
  options: []
  context:
    summary: "The requested changes are ready for final acceptance."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "Current Task constraints apply"
  affected_refs: []
  required_for: close
  resolution: null
  expires_at: null
  created_at: "2026-06-10T12:00:00Z"
  resolved_at: null
blocker_refs: []
state:
  project_id: proj_123
  state_version: 22
```

### Owner links

- Request envelope, response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `UserJudgment`, options, context, and answer payloads: [API Judgment Schemas](schema-judgment.md).
- State refs and summaries: [API State Schemas](schema-state.md).
- Judgment kinds and active values: [API Value Sets](schema-value-sets.md).
- User-owned judgment and non-substitution rules: [Core Model](../core-model.md).
- Public errors and persistence effects: [API Errors](errors.md) and [Storage Effects](../storage-effects.md).

<a id="harnessrecord_user_judgment"></a>

## `harness.record_user_judgment`

### Purpose

Record the user's answer to one existing pending `UserJudgment`. The method resolves, rejects, defers, blocks, or marks the specific pending judgment according to the user's answer; it does not broaden the answer into unrelated approval, scope expansion, acceptance, residual-risk acceptance, or Write Authorization.

### Required inputs

- `ToolEnvelope` with non-null `idempotency_key` and current `expected_state_version` for non-dry-run commits.
- `user_judgment_id`, matching `judgment_kind`, `selected_option_id`, `answer`, `note`, and `accepted_risks`.
- `answer` must contain only the decision-specific payload branch for the pending `judgment_kind`; `selected_option_id` and `note` stay at request level.

### Access requirements

Requires `VerifiedSurfaceContext.access_class=core_mutation` and `verified=true`. The pending judgment must belong to the same project and compatible Task selected by the request.

### State version behavior

A committed non-dry-run result increments `project_state.state_version` exactly once and updates the addressed `user_judgments` row. Dry run and rejection create no judgment resolution, blocker update, event, replay row, or state-version increment.

### Success result

Returns `RecordUserJudgmentResult` with `base.response_kind=result` and `base.effect_kind=core_committed`. The result includes `user_judgment_ref`, updated `user_judgment`, `updated_refs`, current `state`, and `next_actions`.

### Blocked result

The addressed judgment may be committed as `rejected`, `deferred`, `blocked`, or otherwise blocker-producing when that is the user's answer or the compatible result of the focused judgment. This result updates only covered blockers and judgment-dependent summaries. A resolved `scope_decision` still requires `harness.update_scope` before active scope or active Change Unit fields change.

### Rejected result

Returns `ToolRejectedResponse` for stale `expected_state_version`, unknown or non-pending judgment, `judgment_kind` mismatch, invalid selected option, invalid answer payload, expired or incompatible approval, local access failure, or validator failure. Public error code meaning and precedence are owned by [API Errors](errors.md).

### Dry-run behavior

For `dry_run=true`, a valid preview returns `ToolDryRunResponse`. Branch shape is owned by [API Schema Core](schema-core.md); no-effect persistence semantics are owned by [Storage Effects](../storage-effects.md).

### Storage effect

On commit, the method may persist judgment resolution and dependent blocker or summary state. Exact storage effects are owned by [Storage Effects](../storage-effects.md).

### Minimal valid request

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
  judgment_kind: final_acceptance
  selected_option_id: accept
  answer:
    product_decision: null
    technical_decision: null
    scope_decision: null
    sensitive_action_scope: null
    final_acceptance:
      accepted: true
      basis: "Reviewed the visible result basis."
    residual_risk_acceptance: null
    cancellation: null
  note: "Accepted."
  accepted_risks: []
```

### Representative response

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
  judgment_kind: final_acceptance
  status: resolved
  presentation: short
  question: "Do you accept the visible result basis?"
  options: []
  context:
    summary: "The requested changes are ready for final acceptance."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints: []
  affected_refs: []
  required_for: close
  resolution:
    selected_option_id: accept
    answer:
      final_acceptance:
        accepted: true
        basis: "Reviewed the visible result basis."
    note: "Accepted."
    accepted_risks: []
    resolved_by_actor_kind: user
  expires_at: null
  created_at: "2026-06-10T12:00:00Z"
  resolved_at: "2026-06-10T12:05:00Z"
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
    reason: "Evaluate close readiness after final acceptance."
```

### Owner links

- Request envelope, response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `UserJudgment`, `RecordUserJudgmentPayload`, `SensitiveActionScope`, and `AcceptedRiskInput`: [API Judgment Schemas](schema-judgment.md).
- State refs and summaries: [API State Schemas](schema-state.md).
- Judgment values and active method-local values: [API Value Sets](schema-value-sets.md).
- User-owned judgment, final acceptance, residual-risk acceptance, and non-substitution rules: [Core Model](../core-model.md).
- Public errors and persistence effects: [API Errors](errors.md) and [Storage Effects](../storage-effects.md).

<a id="harnessclose_task"></a>

## `harness.close_task`

### Purpose

Evaluate close readiness for an active Task and, when the selected intent allows it and blockers are absent, commit `complete`, `cancel`, or `supersede`. `harness.close_task` may return close blockers; close is a Core state transition, not a report inferred from chat, status text, acceptance alone, residual-risk acceptance alone, evidence alone, or a rendered view.

### Required inputs

- `ToolEnvelope` with `project_id`, `surface_id`, `request_id`, and `dry_run`.
- `task_id`, `intent`, `close_reason`, `superseding_task_id`, and `user_note`.
- For `intent=complete`, `intent=cancel`, or `intent=supersede` with `dry_run=false`, non-null `idempotency_key` and current `expected_state_version`.
- For `intent=check`, `idempotency_key` and `expected_state_version` may be `null`, and `close_reason` must be `null`.

### Access requirements

`intent=check` requires `VerifiedSurfaceContext.access_class=read_status` for protected close-readiness detail. Mutating intents require `VerifiedSurfaceContext.access_class=core_mutation`, `verified=true`, compatible Task identity, valid lifecycle, and any close-relevant owner records.

### State version behavior

`intent=check` is always read-only and never increments state, including when `dry_run=true`. A committed terminal close or committed blocked close for mutating intents increments `project_state.state_version` exactly once. Close preflight rejection, stale `expected_state_version`, stale close-relevant `WriteAuthorization.basis_state_version`, idempotency request-hash conflict, and dry-run preview increment nothing.

### Success result

Returns `CloseTaskResult` with `base.response_kind=result`. For `intent=check`, `base.effect_kind=read_only` and `close_state` is a computed current close state. For a successful terminal mutation, `base.effect_kind=core_committed` and `close_state` is `closed`, `cancelled`, or `superseded`.

### Blocked result

After close preflight succeeds, `intent=complete` may return `CloseTaskResult(close_state=blocked)` with `blockers: CloseReadinessBlocker[]`. Mutating intents may persist blocker-state effects only when the method state-effect table allows that committed blocked result. The presence of `CloseReadinessBlocker` alone does not imply persistence. `STATE_VERSION_CONFLICT` is never a `CloseReadinessBlocker.code`.

### Rejected result

Returns `ToolRejectedResponse` for close preflight failures before close-readiness evaluation: validation failure, local access failure, stale `expected_state_version`, stale close-relevant Write Authorization basis, idempotency request-hash conflict, wrong-project or unreadable Task identity, unavailable Core, or insufficient capability. Rejected responses return no `CloseTaskResult.blockers` and create no close effect.

### Dry-run behavior

`intent=check` with `dry_run=true` remains the read-only `CloseTaskResult` branch. Mutating intents with `dry_run=true` use the common preview branch when valid; branch shape and planned-blocker representation are owned by [API Schema Core](schema-core.md) and [API Errors](errors.md).

### Storage effect

`intent=check` has no storage effect. Mutating close intents may persist close or blocker outcomes according to the method result. Exact storage effects are owned by [Storage Effects](../storage-effects.md).

### Minimal valid request

```yaml
method: harness.close_task
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_close_check_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: en-US
  task_id: task_456
  intent: check
  close_reason: null
  superseding_task_id: null
  user_note: null
```

### Representative response

Blocked read-only result branch (`CloseTaskResult`, `intent=check`):

```yaml
base:
  response_kind: result
  effect_kind: read_only
  dry_run: false
  state_version: 23
  events: []
close_state: blocked
state:
  project_id: proj_123
  state_version: 23
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 23
blockers:
  - category: evidence
    code: EVIDENCE_INSUFFICIENT
    message: "Required close evidence is not yet sufficient."
    related_refs: []
evidence_summary:
  status: insufficient
  coverage_items: []
  artifact_refs: []
artifact_refs: []
next_actions:
  - action: harness.record_run
    reason: "Record evidence before attempting close."
```

### Owner links

- Request envelope, common response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- Close-readiness shapes, `CloseReadinessBlocker`, `EvidenceSummary`, and `StateSummary`: [API State Schemas](schema-state.md).
- Close state, lifecycle, close reason, and blocker values: [API Value Sets](schema-value-sets.md).
- Full close-readiness evaluation order and close honesty: [Core Model close readiness](../core-model.md#close_task).
- Public errors and close blocker routing: [API Errors](errors.md) and [`close_task` blocker mapping](errors.md#harnessclose_task-close-blockers).
- Persistence effects and state-version behavior: [Storage Effects](../storage-effects.md) and [Storage Versioning](../storage-versioning.md).
