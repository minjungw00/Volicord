# Active MVP API

## What this document helps you do

Use this reference to look up the active current MVP API surface. It owns method-level request, response, state effect, storage owner, error, and security boundary summaries for the active method-name value set owned by [API Schema Core](schema-core.md#current-mvp-value-sets).

This document describes future Harness Server behavior for planning and review. No Harness runtime or server implementation exists in this repository today. Future API or schema candidates are cataloged in [Later Candidate Index](../../later/index.md), not in this active reference. Storage DDL and full shared schema bodies are owned outside this method reference.

## Main Idea

The active MVP API is a small local MCP surface for one user work loop. It can intake work, show status, update active scope, check proposed product writes against current Core state, record runs and evidence refs, ask and record user-owned judgment, and close only when active blockers allow it.

The API does not provide OS permissions, arbitrary-tool sandboxing, tamper-proof files, pre-tool blocking, or security isolation. `harness.prepare_write` returns a cooperative Harness record/check only.

Requirement shaping uses the active Task, Change Unit, `user_judgment`, evidence summary, blocker paths, next actions, and the derived `ShapingReadiness` view. The API must not introduce separate active Discovery Brief, Question Queue, Assumption Register, or similar committed planning artifacts to move from a vague request to a safe first Change Unit.

<a id="active-mvp-method-behavior"></a>

## Active MVP Method Behavior

The exact active method-name value set is owned by [API Schema Core](schema-core.md#current-mvp-value-sets). This page owns the behavior of those current methods:

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

Method state effects are fixed by this matrix. "Event created",
"`tool_invocations` replay row created", and "`state_version` increments" mean a
new committed non-dry-run mutation. The version increment is always one
project-wide `project_state.state_version` increment by exactly 1. Idempotent
replay returns the existing committed response and does not create a second
event, replay row, or version increment. A committed blocked response has those
effects only in rows whose "Committed blocked response allowed" cell says yes.

| Method | Read-only or mutating | `dry_run` allowed | `idempotency_key` required | `expected_state_version` required | Committed blocked response allowed | Event created | `tool_invocations` replay row created | `state_version` increments |
|---|---|---|---|---|---|---|---|---|
| `harness.intake` | Mutating | Yes; never commits | Yes for non-dry-run | Yes for non-dry-run | Yes, when the method commits shaping/blocker state instead of a write-ready path | Yes, on commit | Yes, on first commit | Yes, on commit |
| `harness.status` | Read-only | Yes; returns read-only result | No | No; may be `null` | No; blockers are computed response fields only | No | No | No |
| `harness.update_scope` | Mutating | Yes; never commits | Yes for non-dry-run | Yes for non-dry-run | Yes, only for method-owned blocker/current-row updates; no scope authority is created by an unsatisfied precondition | Yes, on commit | Yes, on first commit | Yes, on commit |
| `harness.prepare_write` | Mutating | Yes; never commits | Yes for non-dry-run | Yes for non-dry-run | Yes, for committed `blocked`, `approval_required`, or `decision_required` blocker updates; no consumable Write Authorization is created | Yes, on committed `allowed` or committed blocker update | Yes, on first committed `allowed` or committed blocker update | Yes, on committed `allowed` or committed blocker update |
| `harness.stage_artifact` | Temporary artifact utility; not Core-state-changing | Yes; previews staging | No | No; may be `null` | No; invalid staging requests fail without Core mutation | No | No | No |
| `harness.record_run` | Mutating | Yes; never commits | Yes for non-dry-run | Yes for non-dry-run | Yes, only when recording a compatible Run or run-related blocker state; rejected attempts are pre-commit failures | Yes, on commit | Yes, on first commit | Yes, on commit |
| `harness.request_user_judgment` | Mutating | Yes; never commits | Yes for non-dry-run | Yes for non-dry-run | No separate blocked-response commit; the method either commits the pending judgment path or fails pre-commit | Yes, on commit | Yes, on first commit | Yes, on commit |
| `harness.record_user_judgment` | Mutating | Yes; never commits | Yes for non-dry-run | Yes for non-dry-run | Yes, when the addressed judgment is committed as rejected, deferred, blocked, or otherwise blocker-producing | Yes, on commit | Yes, on first commit | Yes, on commit |
| `harness.close_task intent=check` | Read-only | Yes; flag allowed, response branch remains `CloseTaskResult` | No | No; may be `null` | No; close blockers are computed response fields only | No | No | No |
| `harness.close_task intent=complete` | Mutating completion attempt | Yes; never commits | Yes for non-dry-run | Yes for non-dry-run | Yes, when complete blockers are persisted while the Task remains open | Yes, on completed commit or committed blocked complete | Yes, on first completed commit or committed blocked complete | Yes, on completed commit or committed blocked complete |
| `harness.close_task intent=cancel` | Mutating terminal cancellation attempt | Yes; never commits | Yes for non-dry-run | Yes for non-dry-run | Yes, only for blockers that invalidate cancellation itself while the Task remains open | Yes, on cancelled commit or committed blocked cancellation | Yes, on first cancelled commit or committed blocked cancellation | Yes, on cancelled commit or committed blocked cancellation |
| `harness.close_task intent=supersede` | Mutating terminal supersession attempt | Yes; never commits | Yes for non-dry-run | Yes for non-dry-run | Yes, only for blockers that invalidate supersession itself while the Task remains open | Yes, on superseded commit or committed blocked supersession | Yes, on first superseded commit or committed blocked supersession | Yes, on superseded commit or committed blocked supersession |

<a id="shared-request-rules"></a>

## Shared Request Rules

All methods use [`ToolEnvelope`](schema-core.md#tool-envelope). Each public method response is exactly one response branch: the concrete method-specific `MethodResult`, `ToolRejectedResponse`, or `ToolDryRunResponse`. The method result schema names the concrete result for actual read results, successful staging results, Core committed results, or committed blocked results when the method state-effect table allows that blocked commit. Method results use [`ToolResultBase`](schema-core.md#common-response) with `response_kind=result`; `ToolRejectedResponse` and `ToolDryRunResponse` use the shared response schemas from [API Schema Core](schema-core.md#common-response) and do not inherit method-specific result-only fields.

Committed non-dry-run state-changing calls require a non-null `idempotency_key` and a current project-wide `expected_state_version`. `harness.stage_artifact`, `harness.status`, `harness.close_task intent=check`, and dry-run calls may use `idempotency_key: null` and `expected_state_version: null`. `harness.stage_artifact` creates only storage-owned temporary staging; it is not a Core state transition and does not create a replay row or increment `project_state.state_version`.

Response branch selection is normative and follows this precedence:

1. A pre-commit failure returns `ToolRejectedResponse` regardless of `dry_run`.
2. A selected read-only operation returns its method-specific `MethodResult`, even when `dry_run=true`; the result uses `base.dry_run=true` and `base.effect_kind=read_only`.
3. A selected state-effecting operation or staging operation with `dry_run=true` returns `ToolDryRunResponse` when the request is otherwise valid and previewable.
4. A non-dry-run committed operation or successful staging operation returns its method-specific `MethodResult`.

Mixed-intent methods choose the response branch by the selected intent's state effect, not by the method name alone. For `harness.close_task`, `intent=check` is read-only, while `intent=complete`, `intent=cancel`, and `intent=supersede` are state-effecting intents.

Response branches map to state effect this way: a read-only result is `MethodResult` with `effect_kind=read_only`; a Core committed result is `MethodResult` with `effect_kind=core_committed`; successful `harness.stage_artifact` staging is `StageArtifactResult` with `effect_kind=staging_created`; a pre-commit failure is `ToolRejectedResponse` with `response_kind=rejected` and `effect_kind=no_effect`; a valid dry run of a selected state-effecting or staging operation is `ToolDryRunResponse` with `response_kind=dry_run` and `effect_kind=no_effect`.

`ToolRejectedResponse` is used for pre-commit failures, including stale `expected_state_version` / `STATE_VERSION_CONFLICT`, request validation failure, invalid staged artifact handle, unavailable MCP/Core or local surface, local surface mismatch, capability failure, and similar failures before a method commit. It does not include method-specific result-only fields such as `decision`, `task_ref`, `run_summary`, `staged_artifact_handle`, or `close_state`.

`ToolDryRunResponse` is used for valid `dry_run=true` calls when the selected operation has a state effect or storage-owned staging effect and Core can evaluate the request shape, local access, capabilities, and reachable state/preconditions enough to produce a preview. It has `effect_kind=no_effect`, no state effect, and no method-specific result-only fields or real generated refs such as `task_ref`, `run_summary`, `staged_artifact_handle`, `write_authorization_ref`, or `user_judgment_ref`. If a dry-run request fails validation, local access verification, capability verification, or state lookup before a read-only result or preview can be produced, the response is `ToolRejectedResponse` with `dry_run=true` and `effect_kind=no_effect`.

Explicit read-only examples: `harness.status` with `dry_run=true` returns `StatusResult` with `base.dry_run=true` and `base.effect_kind=read_only`; `harness.close_task` with `intent=check` and `dry_run=true` returns `CloseTaskResult` with `base.dry_run=true` and `base.effect_kind=read_only`. Conversely, `harness.close_task` with `intent=complete`, `intent=cancel`, or `intent=supersede` and `dry_run=true` returns `ToolDryRunResponse` when the request is otherwise valid and previewable.

Committed blocked outcomes are distinct from rejected responses. A committed blocked `harness.prepare_write` or `harness.close_task` outcome is a `MethodResult` when the method state-effect table allows the blocked commit. Stale `expected_state_version`, validation failure, bad staged handle, unavailable local surface, and similar pre-commit failures are `ToolRejectedResponse`.

When a method has a tool-specific `task_id`, Core resolves the primary Task in this order: tool-specific `task_id`, `ToolEnvelope.task_id`, then active Task. That resolution selects owner records; it does not select a separate state clock. Every fresh non-dry-run state mutation compares `ToolEnvelope.expected_state_version` with the current `project_state.state_version` before commit.
Mismatch returns `STATE_VERSION_CONFLICT`; no method defines a separate public stale-state error or storage-layer alias.

Read-only calls may compute and return blockers, close blockers, next actions, and diagnostics, but those values are response fields only. They must not store blockers, append `task_events`, create `tool_invocations` replay rows, increment `state_version`, mutate close state, create, update, or link artifacts, consume staged handles, or create or consume Write Authorizations.

`dry_run=true` is never authoritative. A valid dry run for a selected state-effecting or staging operation returns `ToolDryRunResponse.dry_run_summary` with descriptive `PlannedEffect` preview data, candidate blockers, would-be errors, and next actions. It creates no current record, `task_events` row, persistent artifact, staged handle, Write Authorization creation or consumption, evidence summary, close state, `tool_invocations` replay row, or state-version increment, and its preview descriptions must not contain fake refs for records that do not exist.

Only committed non-dry-run mutations create `tool_invocations` replay rows. A replay with the same `idempotency_key` and same request hash returns the existing committed response. The same key with a different request hash returns `STATE_VERSION_CONFLICT`. `dry_run` calls and pre-commit failures do not create or reserve replay rows.

Error codes, primary error precedence, idempotency, stale-state behavior, close blocker ordering, and user-facing error labels are owned by [API Errors](errors.md). Shared schemas and active value sets are owned by [API Schema Core](schema-core.md).

Local access classes are Harness API compatibility classes, not OS permission classes. `ToolEnvelope.surface_id` is required for every public request, but it is only a selector. It is not an authority proof and must match a server-derived [`VerifiedSurfaceContext`](schema-core.md#local-surface-access-values) before the API can rely on the surface. The server derives `VerifiedSurfaceContext` from the local transport/session/binding and the stored `LocalSurfaceRegistration`, not from user prose, generated Markdown, Product Repository files, projections, chat text, or agent memory. The same server-derived context is the only source for staged-handle `created_by_surface_id` and `created_by_surface_instance_id` provenance.

Every access class requires `surface_id` to select a same-project `LocalSurfaceRegistration` with `status=active` before the API can rely on that surface. Each public API request has exactly one request-level `VerifiedSurfaceContext.access_class`; nested payloads such as `ArtifactInput[]` do not add a second access class. Every mutating API requires `VerifiedSurfaceContext.verified=true` for the method's access class before commit. Artifact body reads also require `VerifiedSurfaceContext.verified=true` for `access_class=artifact_read`. When applicable, `project_id`, `surface_id`, `surface_instance_id`, `task_id`, and current project-wide `expected_state_version` must be mutually compatible before a protected read exposes Core details or a mutation commits.

| Access class | Covers | Minimum access conditions |
|---|---|---|
| `read_status` | Read-only status/projection methods, including `harness.status`, read-only status resources, and `harness.close_task intent=check`. | Same-project `LocalSurfaceRegistration`, `status=active`, Core/surface reachability for the requested read, `VerifiedSurfaceContext.access_class=read_status` for protected Core detail, and compatible `task_id` when a Task-scoped read is requested. A status read may return display-safe availability or mismatch diagnostics, but it must not invent state from stale text or expose protected Core detail when local access cannot be verified. |
| `core_mutation` | Core state mutation not otherwise specialized: task creation through `harness.intake`, `harness.update_scope`, `harness.request_user_judgment`, `harness.record_user_judgment`, and `harness.close_task` when it mutates state. | `read_status` conditions plus `VerifiedSurfaceContext.access_class=core_mutation`, `verified=true`, non-null `idempotency_key` and current project-wide `expected_state_version` for non-dry-run commits, and compatible `project_id`, `surface_id`, `surface_instance_id`, `task_id`, and owner records when applicable. |
| `write_authorization` | `harness.prepare_write`. | `VerifiedSurfaceContext.access_class=write_authorization`, `verified=true`, plus active Task/Change Unit compatibility, scope, baseline, required separate sensitive-action approval compatibility, and capability checks required for the intended product-file write attempt. |
| `run_recording` | `harness.record_run` only. | `VerifiedSurfaceContext.access_class=run_recording`, `verified=true`, plus compatible `task_id`, `change_unit_id`, `baseline_ref`, observed attempt facts, and a consumable active Write Authorization when the run records a product write. The same `run_recording` request covers recording the result, consuming a compatible Write Authorization when needed, linking compatible existing artifacts, and promoting eligible staged artifacts after staged-handle validity checks. Promotion also requires the current verified `surface_id` and `surface_instance_id` to match the staged handle's server-recorded `created_by_surface_id` and `created_by_surface_instance_id`; the active MVP has no cross-surface staged artifact handoff. `harness.record_run` does not require `artifact_registration`, even when `ArtifactInput[]` contains `source_kind=staged_artifact`. |
| `artifact_registration` | `harness.stage_artifact` only. | `VerifiedSurfaceContext.access_class=artifact_registration`, `verified=true`, compatible `project_id`/`task_id`, and `manual_artifact_attachment_supported=true` for staging new artifact bytes or a safe notice into a temporary `StagedArtifactHandle`. On success, the server records `created_by_surface_id` and `created_by_surface_instance_id` from `VerifiedSurfaceContext`; the caller does not submit those fields as authority. This is input staging, not persistent `ArtifactRef` promotion, not proof that arbitrary local files are safe or authorized, and not a second access class for `harness.record_run`. Caller-supplied raw filesystem paths, arbitrary local path strings, raw logs as authority claims, raw secrets, tokens, full sensitive logs, `captured_artifact` handles, raw capture-adapter outputs, and native capture claims are not accepted as artifact authority in the active MVP. |
| `artifact_read` | Artifact body reads from registered `ArtifactRef` records when an owner path exposes them. | Same-project `LocalSurfaceRegistration`, `status=active`, a registered `ArtifactRef`, compatible `project_id`/`task_id`, required redaction and availability checks, and a matching owner relation in `artifact_links`. Artifact body/content reads require `VerifiedSurfaceContext.access_class=artifact_read` and `verified=true`. Artifact body read is separate from staged artifact promotion, and raw artifact path reads are not granted by default. |

Use `MCP_UNAVAILABLE` when required MCP/Core or surface reachability itself is unavailable, corresponding to `VerifiedSurfaceContext.failure_reason=unavailable`. Use `LOCAL_ACCESS_MISMATCH` when registered local access expectations do not match the reachable transport/session/binding or when local access was revoked, corresponding to `failure_reason=mismatch` or `revoked`. Use `CAPABILITY_INSUFFICIENT` when the surface is recognized but lacks the capability required for the access class, observation, capture, blocking/isolation claim, changed-path detection claim, or active behavior, corresponding to `failure_reason=insufficient_capability`. For baseline changed-path detection, `changed_path_detection_verification=failed` or `stale` must produce `CAPABILITY_INSUFFICIENT` when the method requires that capability; `not_run` or legacy `planned_not_run` cannot support a `detective` label.

<a id="harnessintake"></a>

## `harness.intake`

- **Owns:** Task start/resume/classification and the initial scope candidate for write-capable work.
- **Does not own:** Later active scope updates, later active Change Unit updates, product writes, evidence sufficiency, user judgment resolution, Write Authorization, final acceptance, residual-risk acceptance, or close.
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

`requested_mode` is the caller's requested intake mode. `advisor` means advice, review, or planning without product writes. `direct` means a small direct change. `work` means tracked work. `auto` is input-only: it asks the server to classify `user_request` and resolve the Task to exactly one concrete mode, `advisor`, `direct`, or `work`, before persisting or displaying Task state.

- **Response:**

```yaml
IntakeResponse:
  one_of:
    - IntakeResult
    - ToolDryRunResponse
    - ToolRejectedResponse

IntakeResult:
  base: ToolResultBase
  task_ref: StateRecordRef
  change_unit_ref: StateRecordRef | null
  state: StateSummary
  next_actions: NextActionSummary[]
```

`IntakeResult.state.mode` exposes the resolved concrete mode. It must not be `auto`; later status summaries must also expose the resolved mode rather than the intake request value. `IntakeResult.state.shaping_readiness` exposes the derived readiness view for the current next safe action.
For this state-effecting method, `dry_run=true` returns `ToolDryRunResponse` when the request is otherwise valid and previewable; its `DryRunSummary` may describe the Task or Change Unit that would be created, updated, or resumed through `PlannedEffect` entries, but it does not create a Task or Change Unit and does not return a real `task_ref` or `change_unit_ref`.

- **State effect:** A committed non-dry-run call may create or resume `tasks`, set `project_state.active_task_id`, create an initial scope candidate in `change_units` for write-capable resolved `direct` or `work`, update blockers, append events, create a committed replay row, and increment `project_state.state_version` exactly once. If the request is still not writable, the Task remains or becomes `lifecycle_phase=shaping` with the current goal summary, known scope/non-goals, affected areas or paths, acceptance criteria, Autonomy Boundary, first Change Unit status, named user-owned blocker category when one blocks the next safe action, and one next safe action represented through active Task, Change Unit, user-judgment, evidence, or blocker fields. If the request is already concrete enough for write-capable work, intake may establish enough initial scope for a ready path, but the first product write still requires `harness.prepare_write`. Later changes to the active goal, scope boundary, non-goals, acceptance criteria, autonomy boundary, baseline, or active Change Unit belong to `harness.update_scope`. The method name is not a persisted lifecycle value; created or resumed Tasks must use the active `Task.lifecycle_phase` value set from [API Schema Core](schema-core.md#current-mvp-value-sets). `dry_run` and pre-commit failure create none of these and do not increment `state_version`.
- **Errors:** `VALIDATION_FAILED`, `STATE_VERSION_CONFLICT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `NO_ACTIVE_TASK`, `VALIDATOR_FAILED`.
- **Storage owner:** `project_state`, `tasks`, `change_units`, `blockers`, `task_events`, and `tool_invocations`.
- **Security boundary:** Intake records scope and the resolved concrete mode. It does not authorize local access, sensitive actions, product writes, or stronger guarantee levels.

<a id="harnessupdate_scope"></a>

## `harness.update_scope`

- **Owns:** Updating an active Task's goal summary, scope boundary, non-goals, acceptance criteria, autonomy boundary, baseline reference, and active Change Unit after intake.
- **Does not own:** Task start/classification, user judgment resolution, product writes, evidence, Write Authorization creation, Run recording, final acceptance, residual-risk acceptance, or close.
- **When to call:** After clarification changes active scope, after a resolved `judgment_kind=scope_decision` needs to be applied, or when the active Change Unit or baseline must be created or replaced before write compatibility can be checked.
- **Request:**

```yaml
UpdateScopeRequest:
  envelope: ToolEnvelope
  task_id: string
  goal_summary: string | null
  scope_boundary: string | null
  non_goals: string[] | null
  acceptance_criteria: string[] | null
  autonomy_boundary: string | null
  baseline_ref: string | null
  change_unit:
    operation: keep_active | create_active | replace_active
    scope_summary: string | null
    affected_areas: string[]
    affected_paths: string[]
    constraints: string[]
  related_scope_decision_refs: StateRecordRef[]
```

For top-level scope update fields, `null` means leave the current value unchanged; an empty array replaces that list with an empty list. `affected_areas` names product or repository areas when concrete paths are not yet safe to claim; `affected_paths` names allowed path candidates or exact intended paths when known. `create_active` and `replace_active` must provide enough non-null Change Unit scope to establish the new active boundary.

`related_scope_decision_refs` may link relevant resolved `user_judgment` records whose `judgment_kind=scope_decision`. Those refs explain why the scope changed; they do not mutate scope by themselves.

- **Response:**

```yaml
UpdateScopeResponse:
  one_of:
    - UpdateScopeResult
    - ToolDryRunResponse
    - ToolRejectedResponse

UpdateScopeResult:
  base: ToolResultBase
  task_ref: StateRecordRef
  change_unit_ref: StateRecordRef | null
  linked_scope_decision_refs: StateRecordRef[]
  stale_write_authorization_refs: StateRecordRef[]
  blocker_refs: StateRecordRef[]
  state: StateSummary
  next_actions: NextActionSummary[]
```

For this state-effecting method, `dry_run=true` returns `ToolDryRunResponse` when the request is otherwise valid and previewable; its `DryRunSummary` may report through `PlannedEffect` entries that compatible Write Authorizations would become stale, but it does not mark any Write Authorization stale and does not return real `stale_write_authorization_refs`.

- **State effect:** A committed non-dry-run call may update active Task shaping fields, create or replace the active `change_units` row, update `tasks.active_change_unit_id`, link relevant `scope_decision` user-judgment refs, update blockers, append events, create a committed replay row, and increment `project_state.state_version` exactly once. The update is the active path that turns a vague request into a writable first Change Unit once the current goal summary, active scope summary, allowed paths or affected areas, non-goals, acceptance criteria, Autonomy Boundary, required user-owned judgments, blocking question if any, next safe action, evidence expectation or gap, and close blockers are represented in owner state. When the project-wide state version or the updated Task, Change Unit, baseline, scope boundary, non-goals, acceptance criteria, or autonomy boundary no longer matches an active Write Authorization, Core marks that authorization `status=stale`; it does not consume, revoke, expire, or silently reuse it. `dry_run` and pre-commit failure create no current record, scope change, stale authorization, event, artifact, evidence summary, replay row, or state-version increment.
- **Readiness rule:** `harness.update_scope` may create or replace the first active Change Unit only when `state.shaping_readiness` can honestly show the first safe Change Unit and next safe action, or can show that any remaining false readiness field does not affect that first safe Change Unit. If a blocking user-owned issue remains, the response must identify whether the needed judgment is a `product_decision`, `technical_decision`, `scope_decision`, or `sensitive_approval` instead of hiding it in vague ambiguity.
- **Errors:** `VALIDATION_FAILED`, `STATE_VERSION_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `BASELINE_STALE`, `VALIDATOR_FAILED`.
- **Storage owner:** `tasks`, `change_units`, `write_authorizations`, `blockers`, `task_events`, and `tool_invocations`.
- **Security boundary:** Scope updates change Harness records only. They do not create Write Authorization, grant OS permission, approve sensitive actions, record evidence, or close work. Any stale Write Authorization must be refreshed through `harness.prepare_write` before a product write can be recorded.

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
  one_of:
    - StatusResult
    - ToolRejectedResponse

StatusResult:
  base: ToolResultBase
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

- **State effect:** None. `harness.status` may compute blockers, close blockers, next actions, and diagnostics for the response, but it does not store them, append events, create `tool_invocations` replay rows, increment `state_version`, mutate close state, create, update, or link artifacts, consume staged handles, or create or consume Write Authorizations. If `dry_run=true`, the method follows the shared read-only branch rule and returns the same read-only `StatusResult` shape with `base.dry_run=true` and `base.effect_kind=read_only`.
- **Shaping display:** Status must expose the current lifecycle position honestly. `shaping` means the request is not yet writable, `waiting_user` means one user-owned judgment is required before the next safe action, `ready` means write-capable work has an active Change Unit and can move toward pre-write checking, and `blocked` means an active blocker prevents progress. `StateSummary.shaping_readiness` must show which readiness items are known now: goal summary, non-goals, affected areas or paths, acceptance criteria, Autonomy Boundary, first Change Unit, named user-owned blockers, and next safe action. Read-only work may be ready for its next read-only action, but that does not imply write compatibility. The response should prefer one primary next safe action and one blocking question when a question is truly blocking; non-blocking curiosity questions do not become blockers.
- **Close-state boundary:** `none` is allowed only on `StatusResult.close_state` when no active close state is available. `CloseTaskResult.close_state` uses `ready`, `blocked`, `closed`, `cancelled`, or `superseded`.
- **Errors:** `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`, `NO_ACTIVE_TASK`, `PROJECTION_STALE` when a requested readable view is stale or unavailable.
- **Storage owner:** Read-only over `project_state`, `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `evidence_summaries`, `artifacts`, `artifact_links`, and `blockers`.
- **Security boundary:** Without a promoted profile, status displays only the current MVP `GuaranteeDisplay.level` values `cooperative` or `detective`. `cooperative` is the default. `detective` may be displayed only when the active surface can honestly observe the relevant fact and the relevant capability check has passed; for `reference-local-mcp`, that means `changed_path_detection_verification=passed` and only within verified changed-path detection scope. `not_run`, legacy `planned_not_run` wording, `failed`, or `stale` must downgrade the display to `cooperative` unless the request specifically requires the unsupported capability, in which case the method returns `CAPABILITY_INSUFFICIENT`. `preventive` and `isolated` are later/profile-gated display names and are not current MVP schema values. Stale status text, chat, rendered views, and cached summaries are not authority.

<a id="harnessprepare_write"></a>

## `harness.prepare_write`

- **Owns:** The cooperative pre-write scope check and durable single-use Write Authorization when the proposed attempt is compatible.
- **Does not own:** Sensitive-action approval creation or recording, OS permission, sandboxing, tamper-proof enforcement, pre-tool blocking, user judgment creation, evidence sufficiency, run recording, or close.
- **When to call:** Immediately before a product-file write that must match current Task, Change Unit, baseline, related user judgments, any required sensitive-action approval, and surface capability.
- **Request:**

```yaml
PrepareWriteRequest:
  envelope: ToolEnvelope
  task_id: string | null
  change_unit_id: string | null
  intended_operation: string
  intended_paths: string[]
  product_file_write_intended: boolean
  sensitive_categories: string[]
  baseline_ref: string | null
```

- **Response:**

```yaml
PrepareWriteResponse:
  one_of:
    - PrepareWriteResult
    - ToolDryRunResponse
    - ToolRejectedResponse

PrepareWriteResult:
  base: ToolResultBase
  decision: allowed | blocked | approval_required | decision_required
  state: StateSummary | null
  write_authorization_ref: StateRecordRef | null
  write_authorization: WriteAuthorizationSummary | null
  authorization_effect: none | would_create | created | returned
  active_user_judgment_refs: StateRecordRef[]
  blocked_reasons: CloseBlocker[]
  user_judgment_candidate: UserJudgmentCandidate | null
  guarantee_display: GuaranteeDisplay
```

- **State effect:** A committed non-dry-run `decision=allowed` creates exactly one `write_authorizations.status=active` row for the active path-level `AuthorizedAttemptScope`, appends an event, creates a replay row, and increments `project_state.state_version` exactly once. A committed `blocked`, `approval_required`, or `decision_required` response may update blockers, append an event, create a replay row, and increment `project_state.state_version` exactly once, but it must not create a consumable Write Authorization. `dry_run` and pre-commit failure create no current record, Write Authorization, blocker row, event, artifact, evidence summary, replay row, or state-version increment.
- **Committed blocked decisions:** `decision=blocked`, `decision=approval_required`, and `decision=decision_required` are committed `PrepareWriteResult` values only when the method state-effect table permits that blocked commit. They are not substitutes for pre-commit rejection.
- **State-version conflict:** A project-wide state-version mismatch is always a pre-commit `STATE_VERSION_CONFLICT` in `ToolRejectedResponse.errors`, never a `PrepareWriteResult.decision` value, and it creates no Write Authorization or blocker commit.
- **Dry run:** For this state-effecting method, `dry_run=true` returns `ToolDryRunResponse` when the request is otherwise valid and previewable. Its `DryRunSummary` may preview whether the non-dry-run path would create, reuse, or decline a Write Authorization, but it does not create a consumable Write Authorization and must not return a real `write_authorization_ref`.
- **Errors:** `VALIDATION_FAILED`, `STATE_VERSION_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `DECISION_REQUIRED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `BASELINE_STALE`, `VALIDATOR_FAILED`.
- **Storage owner:** `write_authorizations`, `blockers`, `project_state` state clock, `task_events`, and `tool_invocations`.
- **Security boundary:** `decision=allowed` means compatible with Harness records for this path-level product-file write attempt. It does not mean the operating system will block incompatible writes or that arbitrary tools are isolated. The active `PrepareWriteRequest` contains only the product-write path-level fields listed above and does not encode command, dependency, host, network, secret, deployment, destructive-action, or system-access approval scope. Those approvals are recorded separately as `SensitiveActionScope` through `judgment_kind=sensitive_approval`. Current-MVP requests that require command, network, secret-access, artifact-capture, pre-tool-blocking, isolation, or unverified changed-path detection guarantees must return `CAPABILITY_INSUFFICIENT` when the active surface lacks the capability or has `changed_path_detection_verification=failed` or `stale`, or `VALIDATION_FAILED` when the request shape or requested guarantee is invalid for the active profile. `changed_path_detection_verification=not_run` or legacy `planned_not_run` wording cannot justify `detective`; the method must use a `cooperative` display unless the request requires the stronger guarantee.

<a id="harnessstage_artifact"></a>

## `harness.stage_artifact`

- **Owns:** Temporary input staging of caller-provided safe artifact bytes or a safe notice for one project and Task under `access_class=artifact_registration`.
- **Does not own:** Core state transitions, evidence creation, evidence sufficiency, gate satisfaction, persistent `ArtifactRef` promotion, final acceptance, residual-risk acceptance, or close.
- **When to call:** Before `harness.record_run` when new artifact bytes need to become a temporary `StagedArtifactHandle` for later `ArtifactInput.source_kind=staged_artifact`.
- **Request:**

```yaml
StageArtifactRequest:
  envelope: ToolEnvelope
  task_id: string
  display_name: string
  content_type: string
  redaction_state: none | redacted | secret_omitted | blocked
  safe_bytes_or_notice: bytes | string
  expected_sha256: string | null
  expected_size_bytes: integer | null
  relation_hint: string | null
```

- **Response:**

```yaml
StageArtifactResponse:
  one_of:
    - StageArtifactResult
    - ToolDryRunResponse
    - ToolRejectedResponse

StageArtifactResult:
  base: ToolResultBase
  staged_artifact_handle: StagedArtifactHandle
  expires_at: string
```

- **State effect:** A successful non-dry-run call returns `StageArtifactResult` with `base.effect_kind=staging_created` and creates only a temporary `StagedArtifactHandle` backed by `artifact_staging` or an equivalent storage-owned staging manifest, scoped to `project_id` and `task_id`, with server-recorded `created_by_surface_id` and `created_by_surface_instance_id`, plus `content_type`, `sha256`, `size_bytes`, `redaction_state`, and `expires_at`. The provenance fields are recorded from the successful request's `VerifiedSurfaceContext`, not supplied by the user as authority claims. It creates no Core record, persistent `ArtifactRef`, evidence summary, blocker, event, `tool_invocations` replay row, close effect, or `project_state.state_version` increment. A later compatible `harness.record_run` request, using `access_class=run_recording`, is the only active path that can consume the handle and promote it to a persistent `ArtifactRef`.
- **Dry run:** For this staging method, `dry_run=true` returns `ToolDryRunResponse` when the request is otherwise valid and previewable. Its `DryRunSummary` may preview an `artifact_staging` planned effect, but it does not create storage staging, temporary bytes or notices, a `StagedArtifactHandle`, or a `staged_artifact_handle` result field.
- **Errors:** `VALIDATION_FAILED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`.
- **Storage owner:** `artifact_staging` or an equivalent storage-owned staging manifest plus temporary bytes or notices under `artifacts/tmp/`; persistent `artifacts` and `artifact_links` are created only by a later compatible `harness.record_run` under `run_recording`.
- **Security boundary:** The request carries caller-provided safe bytes or a safe notice, not arbitrary file authority or proof that local files are safe, authorized, or observed by Harness. Raw file paths, raw logs as authority claims, arbitrary local path strings, raw secrets, tokens, full sensitive logs, `captured_artifact` handles, raw capture-adapter outputs, and native capture claims are rejected as active MVP artifact authority. `manual_artifact_attachment_supported=true` means this staging path is available; `native_artifact_capture_supported=false` means the active MVP remains manual artifact staging plus owner promotion/linking, not native artifact capture.

<a id="harnessrecord_run"></a>

## `harness.record_run`

- **Owns:** Run recording under `access_class=run_recording`, compatible Write Authorization consumption, linking existing artifacts, promotion of eligible staged artifacts, compact evidence-summary updates, and run-related blockers.
- **Does not own:** New artifact bytes staging, `access_class=artifact_registration`, new scope, user judgment resolution, final acceptance, residual-risk acceptance, separate assurance records, artifact body reads, or close.
- **When to call:** After shaping work, a direct answer/result, or implementation work. Product-write runs must provide a compatible active Write Authorization from `harness.prepare_write`.
- **Access class:** Always requires `VerifiedSurfaceContext.access_class=run_recording` and only that request access class. `ArtifactInput[]` does not add `artifact_registration`; staged artifact promotion during `record_run` is authorized by `run_recording` plus staged-handle validity checks. The current verified `surface_id` and `surface_instance_id` must match the staged handle's server-recorded `created_by_surface_id` and `created_by_surface_instance_id`; the active MVP does not support cross-surface staged artifact handoff.
- **Artifact inputs:** New artifact bytes must already be represented by a valid `StagedArtifactHandle` created by `harness.stage_artifact`; `record_run` does not stage new bytes. That handle records input staging and server-recorded surface provenance, not native capture, arbitrary local-file authorization, or a bearer token usable by any local caller. `existing_artifact` is not a path to new artifact bytes and does not register a new artifact body; it links a previously persisted `ArtifactRef` into the run evidence only when the ref is valid for the same project and allowed Task scope. Invalid `source_kind`/source-field shape is rejected with `VALIDATION_FAILED`. Staged handle validation failures are also `VALIDATION_FAILED` with `ToolError.details.artifact_input_error`; they are not `LOCAL_ACCESS_MISMATCH` unless the request-level local surface verification itself failed, and they are not `CAPABILITY_INSUFFICIENT` unless the verified surface capability itself is missing. Projection files, generated Markdown, chat text, Product Repository files, and agent memory cannot create the required staged-handle provenance.
- **ArtifactInput validation order:** `record_run` applies this deterministic sequence before any committed response:
  1. Verify the request has `VerifiedSurfaceContext.verified=true` with `VerifiedSurfaceContext.access_class=run_recording`.
  2. Compare `ToolEnvelope.expected_state_version` with current `project_state.state_version`.
  3. Validate the referenced Task and Change Unit.
  4. When the run includes product file writes, validate the compatible Write Authorization.
  5. For each `ArtifactInput.source_kind=staged_artifact`, validate the submitted staged handle against storage-owned staging state.
  6. Validate staged-handle `project_id`, `task_id`, `created_by_surface_id`, `created_by_surface_instance_id`, expiration, consumed status, `sha256`, `size_bytes`, and `redaction_state`.
  7. Promote only validated staged handles to persistent `ArtifactRef` records.
  8. Mark promoted staged handles as consumed in the same committing transaction.
  9. For each `ArtifactInput.source_kind=existing_artifact`, validate that `existing_artifact_ref` is a persistent artifact in the same project and allowed Task scope.
  10. Do not read artifact body content during `record_run`; artifact body reads are handled by a separate read API that requires `VerifiedSurfaceContext.access_class=artifact_read`.
- **Evidence updates:** Each `EvidenceCoverageItem` must name the `claim`, whether it is `required_for_close`, its `coverage_state`, and any supporting or gap refs. Required close coverage is determined by the Task or Change Unit `CompletionPolicy`; `record_run` may update compact evidence coverage, but it does not close the Task or create final acceptance or residual-risk acceptance.
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
  one_of:
    - RecordRunResult
    - ToolDryRunResponse
    - ToolRejectedResponse

RecordRunResult:
  base: ToolResultBase
  run_summary: RunSummary
  registered_artifacts: ArtifactRef[]
  evidence_summary: EvidenceSummary | null
  blocker_refs: StateRecordRef[]
  state: StateSummary
```

- **State effect:** A compatible committed call may create `runs`, consume valid `artifact_staging` handles, promote those staged handles into persistent `artifacts`, add `artifact_links`, create `evidence_summaries`, update run-related blockers, consume `write_authorizations.status=active`, append events, create a committed replay row, and increment `project_state.state_version` exactly once. Product-write runs consume the active Write Authorization only when current `project_state.state_version` matches the authorization's project-wide `basis_state_version` and observed changed paths are compatible with the stored authorized attempt. If the authorization basis no longer matches current `project_state.state_version`, the attempt returns `STATE_VERSION_CONFLICT` before consumption. Rejected calls, dry runs, and pre-commit failures must not create a Run, promote or link artifacts, consume or update staged handles, update evidence, create or consume Write Authorization, append events, create replay rows, or increment `state_version`.
- **Dry run:** For this state-effecting method, `dry_run=true` returns `ToolDryRunResponse` when the request is otherwise valid and previewable. Its `DryRunSummary` may preview run, artifact, evidence, blocker, and Write Authorization effects, but it does not create or return a `run_summary`, consume or promote staged artifacts, link artifacts, update evidence, create or consume a Write Authorization, append events, create replay rows, or increment `state_version`.
- **Pre-commit rejection:** Invalid staged handles, missing or invalid Write Authorization, stale Write Authorization basis, and stale `expected_state_version` return `ToolRejectedResponse`. Rejected responses must not create `run_summary`, promote artifacts, consume staged handles, update evidence, create or consume Write Authorization, add events, create replay rows, or increment `state_version`.
- **Errors:** `VALIDATION_FAILED`, `STATE_VERSION_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID`, `SCOPE_VIOLATION`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `EVIDENCE_INSUFFICIENT`, `VALIDATOR_FAILED`.
- **Storage owner:** `runs`, `write_authorizations`, `artifact_staging`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers`, `task_events`, and `tool_invocations`.
- **Security boundary:** A run can record what the surface observed. In the baseline `reference-local-mcp` profile, product-write compatibility is detective only for observed changed paths after `changed_path_detection_verification=passed`, and only within that verified changed-path scope. `not_run`, legacy `planned_not_run` wording, `failed`, or `stale` cannot justify a `detective` run display. A failed or stale required check produces `CAPABILITY_INSUFFICIENT`; a method path that does not require the stronger claim must downgrade to `cooperative`. The API must not mark command execution, network activity, secret access, artifact capture, blocking, or isolation facts verified when the active surface cannot observe them. Artifact body reads are separate from run artifact promotion and require the `artifact_read` owner path.

<a id="harnessrequest_user_judgment"></a>

## `harness.request_user_judgment`

- **Owns:** Creation of a pending `UserJudgment` for one focused user-owned decision.
- **Does not own:** The user's answer, active scope mutation, active Change Unit mutation, sensitive-action permission, Write Authorization, evidence, final acceptance, residual-risk acceptance, or close.
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
  one_of:
    - RequestUserJudgmentResult
    - ToolDryRunResponse
    - ToolRejectedResponse

RequestUserJudgmentResult:
  base: ToolResultBase
  user_judgment_ref: StateRecordRef
  user_judgment: UserJudgment
  blocker_refs: StateRecordRef[]
  state: StateSummary
```

- **State effect:** A committed non-dry-run `RequestUserJudgmentResult` creates one pending `user_judgments` row, may link or update affected blockers, appends an event, creates a replay row, and increments `project_state.state_version` exactly once. The actual pending judgment exists only in this result branch after commit. A candidate returned by another method is not a pending judgment until this method commits. `dry_run` and pre-commit failure create no pending judgment, blocker update, event, replay row, or state-version increment.
- **Dry run:** For this state-effecting method, `dry_run=true` returns `ToolDryRunResponse` when the request is otherwise valid and previewable. Its `DryRunSummary` may preview a pending user-judgment planned effect, but it does not create a pending judgment and must not return a real `user_judgment_ref`.
- **Errors:** `VALIDATION_FAILED`, `STATE_VERSION_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`, `VALIDATOR_FAILED`.
- **Storage owner:** `user_judgments`, `blockers`, `task_events`, and `tool_invocations`.
- **Security boundary:** The request presents a question. It grants no permission and resolves no gate until `harness.record_user_judgment` records a matching answer. A `scope_decision` answer still requires `harness.update_scope` before active scope or the active Change Unit changes.

<a id="harnessrecord_user_judgment"></a>

## `harness.record_user_judgment`

- **Owns:** Resolution, rejection, deferral, or blocking of an existing pending `UserJudgment`.
- **Does not own:** A broader decision than the pending `judgment_kind`, active scope mutation, active Change Unit mutation, product writes, evidence, Write Authorization, close, or any judgment not explicitly asked.
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

`selected_option_id` and `note` are canonical request-level fields. `answer` must not repeat either one; `RecordUserJudgmentPayload` carries only decision-specific answer details.

- **Response:**

```yaml
RecordUserJudgmentResponse:
  one_of:
    - RecordUserJudgmentResult
    - ToolDryRunResponse
    - ToolRejectedResponse

RecordUserJudgmentResult:
  base: ToolResultBase
  user_judgment_ref: StateRecordRef
  user_judgment: UserJudgment
  updated_refs: StateRecordRef[]
  state: StateSummary
  next_actions: NextActionSummary[]
```

- **State effect:** A committed non-dry-run call updates `user_judgments.status`, records the request-level selected option, request note, and answer details, updates only covered blockers and judgment-dependent summaries, appends an event, creates a replay row, and increments `project_state.state_version` exactly once. It does not directly mutate active Task scope fields or the active Change Unit. If a resolved `scope_decision` means scope must change, the response's next action points to `harness.update_scope`. It creates no standalone accepted-risk row in active MVP. `dry_run` and pre-commit failure create no judgment resolution, blocker update, event, replay row, or state-version increment.
- **Dry run:** For this state-effecting method, `dry_run=true` returns `ToolDryRunResponse` when the request is otherwise valid and previewable. Its `DryRunSummary` may preview the judgment resolution and dependent blocker or next-action effects, but it does not resolve the judgment, update judgment state, update blockers, append events, create replay rows, or increment `state_version`.
- **Errors:** `VALIDATION_FAILED`, `STATE_VERSION_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_UNRESOLVED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `VALIDATOR_FAILED`.
- **Storage owner:** `user_judgments`, `blockers`, `task_events`, and `tool_invocations`.
- **Security boundary:** Broad phrases such as "go ahead" or "looks good" do not become product decisions, sensitive-action approval, final acceptance, residual-risk acceptance, cancellation, or scope expansion unless the pending active judgment explicitly asked for that kind and the recorded answer matches it. For `judgment_kind=sensitive_approval`, the answer records `RecordUserJudgmentPayload.sensitive_action_scope`, not `AuthorizedAttemptScope`. Approval of an intended command, dependency change, network access, secret access, deployment, destructive action, system access, or product-file write does not prove Harness observed, blocked, enforced, sandboxed, or isolated that action unless the active surface has a verified capability for the exact operation. Later-only judgment candidates are not active MVP judgment kinds.

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
  one_of:
    - CloseTaskResult
    - ToolDryRunResponse
    - ToolRejectedResponse

CloseTaskResult:
  base: ToolResultBase
  close_state: ready | blocked | closed | cancelled | superseded
  state: StateSummary
  blockers: CloseBlocker[]
  evidence_summary: EvidenceSummary | null
  artifact_refs: ArtifactRef[]
  next_actions: NextActionSummary[]
```

`CloseTaskResponse` branch use is intent-specific. `CloseTaskResult` is used for `intent=check` read-only results, including `intent=check` with `dry_run=true`; it is also used for non-dry-run committed `intent=complete`, `intent=cancel`, or `intent=supersede` results and for committed blocked close results. `ToolDryRunResponse` is used only for valid dry-run previews of `intent=complete`, `intent=cancel`, or `intent=supersede`; it is not used for `intent=check`. `ToolRejectedResponse` is used for pre-commit failures regardless of intent or `dry_run`.

Close concepts stay separate. `Task.lifecycle_phase` is the persisted lifecycle field; active values are `shaping`, `ready`, `executing`, `waiting_user`, `blocked`, `completed`, `cancelled`, and `superseded`. `CloseTaskResult.close_state` is response-level close status with values `ready`, `blocked`, `closed`, `cancelled`, and `superseded`. `Task.close_reason` stores close detail as `none`, `completed_self_checked`, `completed_with_risk_accepted`, `cancelled`, or `superseded`. `Task.result` stores only the coarse outcome `none`, `advice_only`, `completed`, `cancelled`, or `superseded`; unsuccessful Runs, violations, blocked closes, and evidence gaps remain in Run status, `CloseBlocker`, evidence state, or current Task state.

`intent` determines the API behavior:

| `intent` | API behavior |
|---|---|
| `check` | Always read-only. Returns `CloseTaskResult` with `base.effect_kind=read_only`; `dry_run=true` does not change the response branch and returns `CloseTaskResult` with `base.dry_run=true` and `base.effect_kind=read_only`. It computes close readiness and blockers for the response only. It creates no `task_events`, replay rows, `project_state.state_version` increment, `close_state` mutation, artifact creation/update/linking, staged-handle consumption, or Write Authorization creation/consumption. `close_reason` must be `null`. |
| `complete` | Runs the ordered complete blocker matrix from [Core Model](../core-model.md#close_task). If no blocker remains, stores `lifecycle_phase=completed`, `result=completed`, and the derived `close_reason`. A blocked complete that commits blocker state returns `CloseTaskResult` with `close_state=blocked`. |
| `cancel` | Terminal cancellation, not successful completion. Requires valid task identity, valid lifecycle, compatible local access, and no recovery constraint that prevents the transition. It does not require evidence sufficiency, final acceptance, or residual-risk acceptance. If `close_reason` is non-null it must be `cancelled`; the committed row stores `close_reason=cancelled` and `result=cancelled`. |
| `supersede` | Terminal replacement, not successful completion. Requires the same identity, lifecycle, local-access, and recovery checks as cancellation, plus a valid open same-project `superseding_task_id` when the active pointer will move. It does not require evidence sufficiency, final acceptance, or residual-risk acceptance. If `close_reason` is non-null it must be `superseded`; the committed row stores `close_reason=superseded` and `result=superseded`. |

For `intent=complete`, the API response reflects the ordered Core blocker matrix through `CloseTaskResult.blockers`. The first unsatisfied row supplies the primary close-blocker basis; later rows may still appear as secondary blockers, but they cannot satisfy or hide earlier rows.

| Order | `CloseBlocker.category` | Typical public error path |
|---:|---|---|
| 1 | `task` | `NO_ACTIVE_TASK` or `VALIDATION_FAILED` for invalid Task identity, project mismatch, terminal lifecycle, or impossible complete transition. |
| 2 | `open_run` | `VALIDATOR_FAILED` when an open, interrupted, violation, incompatible, or unrepaired Run blocks the close basis and no more specific typed code applies. |
| 3 | `scope` | `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, or `BASELINE_STALE` for missing, stale, or incompatible active scope, Change Unit, acceptance criteria, or `CompletionPolicy`. |
| 4 | `user_judgment` | `DECISION_REQUIRED` or `DECISION_UNRESOLVED` for unresolved required non-sensitive user-owned judgments. |
| 5 | `sensitive_approval` | `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, or `APPROVAL_EXPIRED` for missing, denied, expired, stale, or incompatible `sensitive_approval`. |
| 6 | `write_compatibility` | `STATE_VERSION_CONFLICT`, `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID`, `SCOPE_VIOLATION`, or `BASELINE_STALE` for project-wide version mismatch, missing or incompatible Write Authorization, product-write Run, paths, baseline, or observed write. |
| 7 | `baseline` or `surface_capability` | `BASELINE_STALE`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, or `LOCAL_ACCESS_MISMATCH` when the baseline or verified local surface cannot honestly support the close claim. |
| 8 | `evidence` | `EVIDENCE_INSUFFICIENT` when required evidence is absent, partial, stale, blocked, or otherwise insufficient under the active `CompletionPolicy`. |
| 9 | `artifact_availability` | `ARTIFACT_MISSING` when a close-relevant `ArtifactRef` is missing, unavailable, integrity-failed, blocked beyond the allowed safe notice, or unusable. |
| 10 | `final_acceptance` | `ACCEPTANCE_REQUIRED` when required `final_acceptance` is missing, rejected, stale, or not tied to the visible close basis. |
| 11 | `residual_risk_visibility` | `RESIDUAL_RISK_NOT_VISIBLE` when known close-affecting residual risk is not visible enough for the user to judge. |
| 12 | `residual_risk_acceptance` | `DECISION_REQUIRED` or `DECISION_UNRESOLVED` when visible close-affecting residual risk still requires compatible `residual_risk_acceptance`. |
| 13 | `recovery` | `STATE_VERSION_CONFLICT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, or `VALIDATOR_FAILED` when replay, recovery, corruption, local access, unresolved blocker state, or another repair constraint must be addressed. |
| 14 | none | If no blocker remains, commit the complete transition; otherwise return `close_state=blocked` and leave the Task open. |

For `intent=complete`, Core derives `close_reason` from the close basis. `completed_self_checked` means required evidence is sufficient, required `final_acceptance` is resolved, and no close-affecting `residual_risk_acceptance` is required. `completed_with_risk_accepted` means required evidence is sufficient, required `final_acceptance` is resolved, and compatible `residual_risk_acceptance` exists for close-affecting visible residual risk. A request-supplied `close_reason` must match the derived outcome; incompatible combinations fail validation instead of mixing completion, cancellation, and supersession.

Evidence sufficiency for `close_task` is derived from `EvidenceSummary.completion_policy` and `EvidenceSummary.coverage_items`. If `completion_policy.evidence_required=true`, `close_task` may treat `EvidenceSummary.status=sufficient` as valid only when every `EvidenceCoverageItem` with `required_for_close=true` has `coverage_state=supported` or `not_applicable`. If any required item is `unsupported`, `partial`, `stale`, or `blocked`, or if a required item is absent from the coverage set, `close_task` must return `close_state=blocked` with an evidence close blocker and may use `EVIDENCE_INSUFFICIENT` as the primary error. Artifact availability remains separate: missing, unavailable, integrity-failed, or unusable close-relevant artifacts can produce `ARTIFACT_MISSING` or an `artifact_availability` close blocker even when the evidence record is otherwise well formed.

Final acceptance and residual-risk acceptance are checked only after required evidence and close-relevant artifacts have passed. They cannot override an evidence close blocker, cannot turn an unsupported required coverage item into sufficient evidence, and cannot substitute for a missing required `ArtifactRef` or `StateRecordRef`.

- **Active-task pointer:** On committed `intent=supersede`, if the old Task is `project_state.active_task_id`, `superseding_task_id` must become `project_state.active_task_id` only when it names a valid open same-project Task; otherwise the active pointer must be cleared. The old superseded Task must not remain active. Even when this updates both Task lifecycle and `project_state.active_task_id`, the committed call is one state mutation with one project-wide version increment.
- **State effect:** `intent=check` is always read-only, including when the envelope has `dry_run=true`. It creates no `task_events`, replay rows, `project_state.state_version` increment, `close_state` mutation, artifact update, staged-handle consumption, or Write Authorization creation/consumption. A committed non-dry-run terminal close updates `tasks.lifecycle_phase`, `tasks.close_reason`, `tasks.result`, `tasks.closed_at`, affected `change_units`, blockers, project active-task state when needed, events, replay, and `project_state.state_version` exactly once. A committed blocked close attempt may record blockers, append an event, create a replay row, and increment `project_state.state_version` exactly once, but it must leave the Task open. Stale state, invalid request shape, local surface failure, and any precondition failure before a close-matrix commit return `ToolRejectedResponse`.
- **Dry run and branch selection:** `harness.close_task` is a mixed-intent method, so the response branch is chosen by the selected intent's state effect, not by the method name alone. `intent=check` with `dry_run=true` returns `CloseTaskResult` with `base.dry_run=true` and `base.effect_kind=read_only`; it must not return `ToolDryRunResponse`. `intent=complete`, `intent=cancel`, or `intent=supersede` with `dry_run=true` returns `ToolDryRunResponse` when the request is otherwise valid and previewable. Its `DryRunSummary` may preview terminal or blocked close effects, close blockers, and next actions, but it does not change close state, Task lifecycle, blockers, events, replay rows, artifact creation, update, or linking, staged-handle consumption, Write Authorization creation or consumption, or `state_version`.
- **Errors:** `VALIDATION_FAILED`, `STATE_VERSION_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `EVIDENCE_INSUFFICIENT`, `ARTIFACT_MISSING`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `VALIDATOR_FAILED`.
- **Storage owner:** `tasks`, `change_units`, `blockers`, `runs`, `evidence_summaries`, `artifacts`, `artifact_links`, `user_judgments`, `task_events`, and `tool_invocations`.
- **Security boundary:** Close is a Core state transition, not a report. It cannot be inferred from chat, status text, final acceptance alone, residual-risk acceptance alone, evidence alone, or a rendered view.
