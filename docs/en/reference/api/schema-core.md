# API Schema Core

## What this document helps you do

Use this reference for the active current MVP method-name set, shared API shapes, and closed schema value sets: the tool envelope, response branches, `ArtifactRef`, `StateRecordRef`, `ShapingReadiness`, `UserJudgment`, Write Authorization summary, `CompletionPolicy`, evidence summary, run summary, close blockers, next-action summary, and current MVP enum values.

This document describes future Harness Server behavior for planning and review. It does not mean the current documentation repository implements an MCP server. Future schema candidates stay in [Later Candidate Index](../../later/index.md#later-schema-candidates).

## Contract Map

| Need | Owner |
|---|---|
| Exact active method-name value set and shared schema value sets | This document |
| `ToolEnvelope.surface_id`, `LocalSurfaceRegistration`, `VerifiedSurfaceContext`, local surface access value sets, and capability-profile value sets used by guarantee display | This document |
| Method request/response behavior for active methods | [MVP API](mvp-api.md) |
| Public errors, precedence, idempotency, blocked behavior, and stale-state behavior | [API Errors](errors.md) |
| Core state semantics, shaping readiness meaning, and lifecycle meaning | [Core Model Reference](../core-model.md) |
| Storage tables, JSON `TEXT`, enum hardening, and artifact persistence | [Storage](../storage.md) |
| Security guarantee meanings | [Security Reference](../security.md) |
| Future API/schema candidates | [Later Candidate Index](../../later/index.md#later-schema-candidates) |

## Schema Notation Convention

The YAML-like blocks in this document are normative schema notation, not examples unless marked as examples.

- `field: Type` means the field is required and non-null inside the object or union branch that declares it. A field required in one response branch is not required in sibling branches unless those branches also declare it.
- `field: Type | null` means the field is required and may be JSON `null`.
- `Type[]` means the field is present and contains an array; an empty array is written as `[]`.
- `a | b | c` is a closed active enum for that field.
- Later, reserved, or profile-gated names must not appear in active enum notation or active value tables. They stay in [Later Candidate Index](../../later/index.md) until promoted by an owner document.
- Unlisted fields are rejected outside an explicitly named extension container.

Storage validation is a separate owner boundary. API payloads and API-shaped stored JSON validate against this API reference first; DDL, storage-only JSON, defaults, locks, and migrations validate against [Storage](../storage.md).

The [Current MVP Value Sets](#current-mvp-value-sets) section owns the exact active method-name set and the active schema enum values declared by this document. Method-specific behavior remains with [MVP API](mvp-api.md), and public `ErrorCode` taxonomy remains with [API Errors](errors.md).

<a id="tool-envelope"></a>

## Tool Envelope

Every public tool request carries `ToolEnvelope`. Committed non-dry-run state-changing tools require a non-null `idempotency_key` and a current project-wide `expected_state_version` matching `project_state.state_version`. `harness.stage_artifact`, `harness.status`, `harness.close_task intent=check`, and dry-run calls may set `idempotency_key` and `expected_state_version` to `null`. `harness.stage_artifact` creates only a temporary staging handle and is not a Core state transition. Read-only calls do not require or reserve idempotency keys. Method-level state effects are owned by [MVP API](mvp-api.md#active-mvp-method-behavior).

```yaml
ToolEnvelope:
  request_id: string
  idempotency_key: string | null
  expected_state_version: integer | null
  project_id: string
  task_id: string | null
  surface_id: string
  actor_kind: user | lead_agent
  dry_run: boolean
```

Envelope fields route and audit the call. `ToolEnvelope.surface_id` is required, but it is only a selector. It must match a server-verified local surface context before an API owner can rely on it. `surface_id` does not prove caller authority and does not grant capability, write authority, local access, user judgment, sensitive-action permission, final acceptance, residual-risk acceptance, artifact access, or close.

<a id="local-surface-access-values"></a>

## Local Surface Access Values

Local surface access values describe Harness API compatibility. They are not OS permissions, sandbox boundaries, tamper-proof guarantees, universal pre-tool blocking, or isolation.

`LocalSurfaceRegistration` is the conceptual registration fact for one local surface in one project. It is persisted by storage as registration data, not accepted from a tool request as authority. Product Repository files, projections, generated Markdown, chat text, and agent memory cannot create, modify, or refresh a surface registration.

```yaml
LocalSurfaceRegistration:
  project_id: string
  surface_id: string
  surface_instance_id: string
  transport_kind: local_mcp_stdio | local_http
  transport_binding_fingerprint: string
  access_secret_hash: string | null
  capability_profile_hash: string
  status: active | disabled | stale | revoked
  local_access_posture: registered_local | unavailable | mismatch | revoked
  registered_at: string
  last_verified_at: string | null
```

`VerifiedSurfaceContext` is derived by the server for one concrete request and one access class. `VerifiedSurfaceContext.access_class` is a single request-level value in the active MVP; one public API request is not modeled as requiring multiple `access_class` values. It is not a request payload, not a Markdown assertion, and not an agent-memory fact. The server derives it from the local transport/session/binding and the stored `LocalSurfaceRegistration`.

```yaml
VerifiedSurfaceContext:
  project_id: string
  surface_id: string
  surface_instance_id: string
  access_class: read_status | core_mutation | write_authorization | run_recording | artifact_registration | artifact_read
  verified: boolean
  failure_reason: unavailable | mismatch | revoked | insufficient_capability | null
```

`registered_local` is a posture resulting from successful local registration and verification. It is not a free-form label, caller claim, generated-file marker, or permission override. `surface_id` must select a same-project registration, and the verified context must match that registration before mutating API access or artifact body reads can proceed.

`LocalSurfaceRegistration.local_access_posture` is a closed current MVP value set:

| Value | Meaning |
|---|---|
| `registered_local` | A recent server verification matched the local transport/session/binding to the registered local surface for this project closely enough for API owners to evaluate access classes. |
| `unavailable` | Required MCP/Core or surface reachability cannot currently be established. |
| `mismatch` | A reachable local transport/session/binding does not match the registered local surface binding for the project. |
| `revoked` | Local access for the registered surface was explicitly revoked and must not be used until a new valid registration replaces it. |

`LocalSurfaceRegistration.status` is a closed current MVP value set:

| Value | Meaning |
|---|---|
| `active` | The registered surface may be considered for current API access checks. |
| `disabled` | The surface remains recorded but must not be used for current API access. |
| `stale` | The surface registration or capability posture must be refreshed before current API access can rely on it. |
| `revoked` | The surface registration is no longer valid for current API access. |

The active local API access-class labels are `read_status`, `core_mutation`, `write_authorization`, `run_recording`, `artifact_registration`, and `artifact_read`.

| `access_class` | Active request-level use |
|---|---|
| `read_status` | Derived state, status, projection, and read-only close-check reads. |
| `core_mutation` | Core state mutation not otherwise specialized, including task creation, scope update, user-judgment recording, and mutating close paths. |
| `write_authorization` | `harness.prepare_write` path-level product-file write authorization preparation. |
| `run_recording` | `harness.record_run`: recording a run result, consuming a compatible Write Authorization when needed, linking existing artifacts, and promoting eligible staged artifacts. |
| `artifact_registration` | `harness.stage_artifact`: staging new artifact bytes or a safe notice into a temporary `StagedArtifactHandle`. |
| `artifact_read` | Artifact body/content reads from persisted `ArtifactRef` records through an owner path. |

`ArtifactInput[]` is payload validated by `harness.record_run`; it does not change that request's `access_class` from `run_recording`. Staged artifact promotion is governed by the `run_recording` request plus staged-handle validity checks, while artifact body reads remain separate and use `artifact_read`. Method-level conditions for these classes are owned by [MVP API](mvp-api.md#shared-request-rules); public error selection is owned by [API Errors](errors.md). `VerifiedSurfaceContext.failure_reason=unavailable`, `mismatch` or `revoked`, and `insufficient_capability` must remain distinguishable so callers can receive `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, and `CAPABILITY_INSUFFICIENT` respectively.

<a id="capability-profile-value-sets"></a>

## Capability Profile Value Sets

Agent Integration owns `capability_profile` field semantics, refresh rules, connector fallback behavior, and surface recipes. Schema Core owns the active value sets used by that profile and by `GuaranteeDisplay`.

```yaml
capability_profile:
  surface_id: reference-local-mcp
  surface_status: active
  local_access_posture: registered_local
  cooperative_prepare_write_supported: true
  changed_path_detection_supported: true
  changed_path_detection_verification: not_run | passed | failed | stale
  manual_artifact_attachment_supported: true
  native_artifact_capture_supported: false
  guarantee_level_default: cooperative
  guarantee_level_max_when_verified: detective
```

`changed_path_detection_verification=passed` is the only value that can support a `detective` display, and only inside the verified changed-path detection scope. `not_run`, legacy `planned_not_run` wording, `failed`, and `stale` are not passing states. `native_artifact_capture_supported=false` keeps the active artifact path limited to `harness.stage_artifact` staging and owner promotion/linking; it does not add `captured_artifact` or native capture authority.

<a id="common-response"></a>

## Response Branches

Every public tool response is exactly one response branch. Method-specific success fields attach only to a method result branch built on `ToolResultBase`; rejected and dry-run branches must not invent success-only fields such as a write decision, run summary, or staged artifact handle.

```yaml
ToolResultBase:
  request_id: string
  idempotency_key: string | null
  project_id: string
  task_id: string | null
  state_version: integer
  dry_run: boolean
  response_kind: result
  effect_kind: read_only | core_committed | staging_created | no_effect
  errors: ToolError[]
  validator_results: ValidatorResult[]
  events: EventRef[]

ToolRejectedResponse:
  request_id: string
  idempotency_key: string | null
  project_id: string
  task_id: string | null
  state_version: integer | null
  dry_run: boolean
  response_kind: rejected
  effect_kind: no_effect
  errors: ToolError[]
  validator_results: ValidatorResult[]
  events: []

ToolDryRunResponse:
  request_id: string
  idempotency_key: string | null
  project_id: string
  task_id: string | null
  state_version: integer | null
  dry_run: true
  response_kind: dry_run
  effect_kind: no_effect
  errors: ToolError[]
  validator_results: ValidatorResult[]
  events: []
  dry_run_summary: DryRunSummary

DryRunSummary:
  method: string
  summary: string
  would_create: string[]
  would_update: string[]
  would_return: string[]

ToolError:
  code: ErrorCode
  message: string
  retryable: boolean
  details: object

EventRef:
  event_id: string
  event_seq: integer
  event_type: string
  task_id: string | null
  state_version: integer
```

`ToolResultBase` is the base for actual method results. Its `state_version` is the project-wide version: the resulting `project_state.state_version` after a committed Core mutation, or the current project-wide version observed for read-only results and temporary staging results. `effect_kind=staging_created` means a temporary staged artifact handle was created by `harness.stage_artifact`; it is not a Core state transition, event, replay row, or `state_version` increment.

`ToolRejectedResponse` is the response for pre-commit failures, including `STATE_VERSION_CONFLICT`, request validation failure, unavailable Core or local MCP surface, local access failure, capability failure, and invalid staged artifact handle. It has `effect_kind=no_effect`, contains no method-specific success fields, and creates no current record, event, artifact, evidence summary, Write Authorization, close state, `tool_invocations` replay row, or `state_version` increment. `ToolRejectedResponse.errors` is always non-empty. `ToolRejectedResponse.events` is always `[]`.

For `ToolRejectedResponse.state_version`, if Core could read the current project state before rejecting, the value is the observed project-wide `project_state.state_version`. If Core or the local MCP surface is unavailable before project state can be read, `state_version` may be `null`.

`ToolDryRunResponse` is the response for a `dry_run=true` call that reports what the method would validate or change without committing it. It has `effect_kind=no_effect`, `events=[]`, and no method-specific success fields. It creates no current record, event, artifact, evidence summary, Write Authorization, close state, `tool_invocations` replay row, or `state_version` increment. `DryRunSummary` names would-create, would-update, or would-return items only as explanatory dry-run output; those strings are not created records, event refs, artifact refs, or authority.

`ToolError` keeps public error identity, retry guidance, and structured details. `EventRef` appears only in result branches that actually have event refs; rejected and dry-run branches always use `events=[]`.

<a id="state-summary"></a>

## State Summary

```yaml
StateSummary:
  mode: advisor | direct | work
  lifecycle_phase: shaping | ready | executing | waiting_user | blocked | completed | cancelled | superseded
  result: none | advice_only | completed | cancelled | superseded
  close_reason: none | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  assurance_level: none | self_checked
  shaping_readiness: ShapingReadiness
  gates:
    scope_gate: not_required | required | pending | passed | failed | blocked
    decision_gate: not_required | required | pending | resolved | deferred | blocked
    approval_gate: not_required | required | pending | granted | denied | expired
    evidence_gate: not_required | none | partial | sufficient | stale | blocked
    acceptance_gate: not_required | required | pending | accepted | rejected

GuaranteeDisplay:
  level: cooperative | detective
  notes: string[]

ShapingReadiness:
  goal_summary_known: boolean
  non_goals_known: boolean
  affected_area_or_paths_known: boolean
  acceptance_criteria_known: boolean
  autonomy_boundary_known: boolean
  first_change_unit_known: boolean
  user_owned_blockers_named: boolean
  next_safe_action_known: boolean
```

`StateSummary.mode` mirrors persisted `tasks.mode` and is always a concrete task mode. `auto` is not a stored mode, displayed task mode, or status-summary mode. `StateSummary.lifecycle_phase` mirrors persisted `Task.lifecycle_phase`. `intake` is an API method and start-handling step, not a lifecycle value. The terminal lifecycle phases are `completed`, `cancelled`, and `superseded`; `superseded` means the Task was replaced by another Task or route and must not return to active work. `StateSummary.close_reason` mirrors persisted `Task.close_reason`. `StateSummary.result` mirrors coarse `Task.result`; failed Runs, violations, blocked closes, evidence gaps, and blockers remain in Run status, `CloseBlocker`, evidence state, blockers, or current Task state instead of becoming a terminal Task result. The `passed` and `failed` strings in this document are gate or validator statuses only, not `Task.result` values.

`StateSummary.shaping_readiness` is a derived active-state view. It is computed from current Task state, active or proposed Change Unit state, pending `UserJudgment` candidates or records, blockers, evidence summary, and next-action state. It is not a persisted Task field, not a separate `StateRecordRef.record_kind`, and not a committed `Discovery Brief`, `Question Queue`, `Assumption Register`, or similar planning artifact. A `false` field stays visible, but it blocks only when the unknown or stale item affects the first safe Change Unit or the next safe action.

Before the first Change Unit is created for write-capable work, `user_owned_blockers_named=true` means any blocking user-owned issue has been identified as a `product_decision`, `technical_decision`, `scope_decision`, or `sensitive_approval`, or no user-owned blocker is currently needed for the next safe action. `next_safe_action_known=true` means the response can name the next owner-path action, such as inspection, `harness.request_user_judgment`, `harness.update_scope`, or `harness.prepare_write`.

`Task.close_reason` values are not interchangeable labels. `completed_self_checked` means required evidence is sufficient, required `final_acceptance` is resolved, and no close-affecting `residual_risk_acceptance` is required. `completed_with_risk_accepted` means required evidence is sufficient, required `final_acceptance` is resolved, and compatible `residual_risk_acceptance` exists for close-affecting visible residual risk. `cancelled` and `superseded` are terminal but not successful completion, and they do not satisfy `CompletionPolicy` evidence, final acceptance, or residual-risk acceptance requirements.

Task mode values have these reader-facing meanings:

- `advisor`: advice, review, or planning without product writes.
- `direct`: small direct change.
- `work`: tracked work.

`IntakeRequest.requested_mode=auto` is only an intake input asking the server to classify the request. The server must resolve it to exactly one of `advisor`, `direct`, or `work` before writing `tasks.mode`, producing `StateSummary.mode`, or returning intake/status summaries.

Rendered labels are not canonical schema values. `GuaranteeDisplay.level` is a display claim about the documented surface capability and proof level; it does not grant permission or state authority. The active MVP guarantee-display values are only `cooperative` and `detective`. `cooperative` is the default. `detective` can be displayed only when the relevant active capability check has passed; for the baseline `reference-local-mcp` profile, that means `changed_path_detection_verification=passed` and only within verified changed-path detection scope. Stronger display names are later candidates, not current MVP schema values.

<a id="staterecordref"></a>

## StateRecordRef

```yaml
StateRecordRef:
  record_kind: project | task | change_unit | run | write_authorization | user_judgment | evidence_summary | blocker
  record_id: string
```

Durable evidence bytes use `ArtifactRef`, not `StateRecordRef`. Active current MVP user-owned judgments are represented by `record_kind=user_judgment` with the matching `UserJudgment.judgment_kind`.

<a id="artifactref"></a>

## ArtifactRef

`ArtifactRef` points to a durable evidence file persisted through Harness storage. It is not a caller-supplied arbitrary path.

```yaml
ArtifactRef:
  artifact_id: string
  kind: diff | log | screenshot | checkpoint | other
  uri: string
  sha256: string
  size_bytes: integer
  content_type: string
  redaction_state: none | redacted | secret_omitted | blocked
  task_id: string
  run_id: string | null
  relation_owner: ArtifactRelationOwner
  created_at: string
  produced_by: lead_agent | harness
  retention_class: task | project | temporary

ArtifactRelationOwner:
  task_id: string
  run_id: string | null
  record_kind: task | change_unit | run | user_judgment | evidence_summary | blocker
  record_id: string
  relation: string
```

`uri` resolves through Harness storage, normally as `harness-artifact://{project_id}/{artifact_id}`. Raw secrets, tokens, and full sensitive logs must not be stored as evidence. If content is redacted, omitted, or blocked, `sha256` and `size_bytes` describe the committed safe bytes, not a hidden original.

<a id="artifactinput"></a>

## ArtifactInput

`ArtifactInput` is accepted by `harness.record_run` only as a documented `StagedArtifactHandle` from the active `harness.stage_artifact` utility or as a previously persisted `ArtifactRef`. `ArtifactInput[]` is consumed under the `run_recording` request access class; it does not add a second access class to `record_run`. It never grants arbitrary file read authority, and `record_run` never reads artifact body content. Artifact body reads use a separate owner path with `VerifiedSurfaceContext.access_class=artifact_read`. `harness.stage_artifact` is the active MVP staging utility for new artifact bytes, not native artifact capture and not a general filesystem-read API.

```yaml
ArtifactInput:
  artifact_input_id: string
  source_kind: staged_artifact | existing_artifact
  relation: string
  staged_artifact_handle: StagedArtifactHandle | null
  existing_artifact_ref: ArtifactRef | null
  display_name: string | null
  content_type: string
  expected_sha256: string | null
  expected_size_bytes: integer | null

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

StageArtifactResponse: StageArtifactResult | ToolRejectedResponse | ToolDryRunResponse

StageArtifactResult:
  base: ToolResultBase
  staged_artifact_handle: StagedArtifactHandle
  expires_at: string

StagedArtifactHandle:
  handle_id: string
  project_id: string
  task_id: string
  created_by_surface_id: string
  created_by_surface_instance_id: string
  sha256: string
  size_bytes: integer
  content_type: string
  redaction_state: none | redacted | secret_omitted | blocked
  expires_at: string
```

Exactly one source field must match `source_kind`: `staged_artifact_handle` for `staged_artifact`, or `existing_artifact_ref` for `existing_artifact`. A missing source field, a source field that does not match `source_kind`, or both source fields present is a request-shape validation failure. A staged handle must be scoped to the same `project_id` and `task_id`, carry `content_type`, `sha256`, `size_bytes`, `redaction_state`, and `expires_at`, and be unexpired and unconsumed when `harness.record_run` uses it. Staged handle validation compares storage-owned `project_id`, `task_id`, `created_by_surface_id`, `created_by_surface_instance_id`, expiration, consumed status, `sha256`, `size_bytes`, and `redaction_state`; failures use public `VALIDATION_FAILED` with `ToolError.details.artifact_input_error`.

`created_by_surface_id` and `created_by_surface_instance_id` are server-recorded provenance fields. `harness.stage_artifact` records them from the successful request's `VerifiedSurfaceContext`; the caller does not choose them in `StageArtifactRequest`, and a user-provided object with those fields is not proof of authority. When `ArtifactInput` submits a `StagedArtifactHandle` back to `harness.record_run`, the server resolves it against the storage-owned staging record and requires the current verified `surface_id` and `surface_instance_id` to match `created_by_surface_id` and `created_by_surface_instance_id`. The active MVP does not support cross-surface staged artifact handoff. A handle with the right shape is still rejected when `project_id`, `task_id`, `created_by_surface_id`, `created_by_surface_instance_id`, expiration, consumed status, `sha256`, `size_bytes`, or `redaction_state` do not match the stored staged artifact and request expectations.

`StageArtifactResult` is the successful result branch for `harness.stage_artifact`; its `base.response_kind` is `result` and its `base.effect_kind` is `staging_created`. Validation failure, local surface failure, capability failure, or any request that cannot safely create a staged handle returns `ToolRejectedResponse` and does not include `staged_artifact_handle`. A dry run returns `ToolDryRunResponse` and also does not include `staged_artifact_handle`.

`harness.stage_artifact` may create a temporary `StagedArtifactHandle`, but it is not a Core state transition by itself. It creates no evidence, satisfies no gate, updates no evidence summary, and cannot make `harness.close_task` pass. `StagedArtifactHandle` is not a bearer token that any local caller may use. `harness.record_run` is the only active path that can consume a valid staged handle and promote it to a persistent `ArtifactRef`; that promotion is authorized by `run_recording` plus same-project, same-Task, server-recorded `created_by_surface_id` / `created_by_surface_instance_id` against current verified `surface_id` / `surface_instance_id`, unexpired, unconsumed, integrity-compatible handle checks. Projection files, generated Markdown, chat text, Product Repository files, and agent memory cannot create or refresh staged-handle provenance.

Raw file paths, raw logs, arbitrary local path strings, `captured_artifact`, captured handles, native artifact capture, raw capture-adapter outputs, raw secrets, tokens, and full sensitive logs are outside the active MVP and are rejected as artifact authority before mutation. New artifact bytes enter the active MVP only through `harness.stage_artifact`; `existing_artifact` only links a previously persisted `ArtifactRef` that is valid for the same project and allowed Task scope. It is not a path to new artifact bytes and does not register a new artifact body.

<a id="evidence-and-pre-write-scope-schemas"></a>

## Evidence And Pre-Write Scope Schemas

```yaml
CompletionPolicy:
  evidence_required: boolean
  final_acceptance_required: boolean
  residual_risk_acceptance_required_when_visible: boolean
  product_write_completion: boolean
  user_visible_result: boolean

EvidenceCoverageItem:
  claim: string
  required_for_close: boolean
  coverage_state: supported | unsupported | partial | not_applicable | stale | blocked
  supporting_state_refs: StateRecordRef[]
  supporting_artifact_refs: ArtifactRef[]
  gap_blocker_refs: StateRecordRef[]
  note: string | null

EvidenceSummary:
  evidence_summary_ref: StateRecordRef | null
  task_id: string
  change_unit_id: string | null
  completion_policy: CompletionPolicy
  status: not_required | none | partial | sufficient | stale | blocked
  coverage_items: EvidenceCoverageItem[]
  supporting_run_refs: StateRecordRef[]
  supporting_artifact_refs: ArtifactRef[]
  gap_blocker_refs: StateRecordRef[]
  summary: string
  updated_at: string

AuthorizedAttemptScope:
  task_id: string
  change_unit_id: string
  basis_state_version: integer
  surface_id: string
  intended_operation: string
  intended_paths: string[]
  product_file_write_intended: boolean
  sensitive_categories: SensitiveCategory[]
  baseline_ref: string | null
  related_user_judgment_refs: StateRecordRef[]
  guarantee_level: cooperative | detective

SensitiveActionScope:
  sensitive_action_id: string
  action_kind: product_file_write | dependency_change | destructive_command | network_access | secret_access | deployment | system_access | other
  named_action: string
  command_or_tool: string | null
  intended_paths: string[]
  hosts: string[]
  dependencies: string[]
  secret_handles: string[]
  time_window: string | null
  scope_limit: string
  not_authorized: string[]
  capability_claim: cooperative_only | observed_by_surface | not_observable

WriteAuthorizationSummary:
  write_authorization_id: string
  attempt_scope: AuthorizedAttemptScope
  status: active | consumed | expired | stale | revoked
  consumed_by_run_id: string | null
  created_at: string
  consumed_at: string | null

WriteAuthoritySummary:
  active_change_unit_ref: StateRecordRef | null
  write_authorization_ref: StateRecordRef | null
  active_authorized_attempt_scope: AuthorizedAttemptScope | null
  approval_status: not_required | required | pending | granted | denied | expired | unknown
  guarantee_display: GuaranteeDisplay
```

`CompletionPolicy` is the compact active close policy for `close_task intent=complete` on a Task or Change Unit. It names whether evidence, final acceptance, residual-risk acceptance when visible, product-write completion, and a user-visible result are required for that completion path. `intent=cancel` and `intent=supersede` are not successful completion and do not satisfy this policy. `CompletionPolicy` is not a QA gate, verification gate, full Evidence Manifest, or permission to add separate assurance workflows.

`EvidenceSummary` is the compact active evidence record tied to that `CompletionPolicy`. Evidence sufficiency is not a vague prose judgment. When `completion_policy.evidence_required=false`, the evidence status should be `not_required`. `EvidenceSummary.status=sufficient` is allowed only when every `EvidenceCoverageItem` with `required_for_close=true` is present and has `coverage_state=supported` or `not_applicable`. If any required coverage item is `unsupported`, `partial`, `stale`, or `blocked`, `harness.close_task` must report a close blocker. If required evidence is missing entirely, the required item must be represented as an unsupported or blocked coverage item or through `gap_blocker_refs`; it cannot be hidden by omitting the item.

Optional coverage may remain explicit with `required_for_close=false`. Optional gaps can be visible without preventing `EvidenceSummary.status=sufficient`, but the required/optional distinction must be explicit even when the MVP summary is small.

Artifact availability and evidence sufficiency are related but distinct. An available persisted `ArtifactRef` does not make evidence sufficient unless a coverage item links it to the claim. A required coverage item that links to a missing, unavailable, integrity-failed, or unusable artifact cannot be sufficient, and `close_task` may also report `CloseBlocker.category=artifact_availability`. Final acceptance and residual-risk acceptance cannot substitute for missing required evidence, and evidence cannot create final acceptance or residual-risk acceptance.

`AuthorizedAttemptScope` is the exact scope stored in `write_authorizations.attempt_scope_json` and later compared by `harness.record_run`. `AuthorizedAttemptScope.basis_state_version` is the project-wide `project_state.state_version` used when `prepare_write` prepared the authorization. `WriteAuthorizationSummary.status` is the durable authorization lifecycle. `blocked` is not a Write Authorization status; blocked writes return blockers without a consumable authorization.

The current MVP `AuthorizedAttemptScope` is only for product-file write attempts. It records the intended product paths, Change Unit, project-wide basis state version, baseline, related user judgment refs, product-write sensitive categories, and honest guarantee level for the path-level write compatibility check. Command execution, dependency installation, network effects, secret access, deployment, destructive action, system access, tool observation, native artifact capture, pre-tool blocking, and isolation are not `AuthorizedAttemptScope` fields. Requests that require those unobservable guarantees must be rejected or blocked as validation or capability failures rather than represented as verified write scope.

`SensitiveActionScope` is the separate scope recorded for `judgment_kind=sensitive_approval`. It can describe permission for an intended command, dependency change, network access, secret access, deployment, destructive action, system access, product-file write, or other named sensitive action. Its `capability_claim` records only what the active surface can honestly claim about the action: `cooperative_only`, `observed_by_surface`, or `not_observable`. A sensitive approval does not mean Harness can observe, block, enforce, sandbox, or isolate the action unless a verified capability for that exact operation says so.

`WriteAuthoritySummary.approval_status` reports the status of any required separate sensitive-action approval. It is not the `WriteAuthorizationSummary.status` lifecycle and does not turn `SensitiveActionScope` into `AuthorizedAttemptScope`.

<a id="record-run-payloads"></a>

## Record-Run Payloads

```yaml
ObservedChanges:
  product_write: boolean
  changed_paths: string[]
  no_product_changes: boolean
  summary: string

RunSummary:
  run_ref: StateRecordRef
  kind: shaping_update | implementation | direct
  status: completed | interrupted | blocked | violation
  product_write: boolean
  write_authorization_ref: StateRecordRef | null
  evidence_summary_ref: StateRecordRef | null
  artifact_refs: ArtifactRef[]
  summary: string
  started_at: string | null
  completed_at: string
```

Only `status=completed` can support evidence through normal owner refs. `interrupted`, `blocked`, and `violation` are audit/recovery facts and do not satisfy evidence, final acceptance, residual-risk acceptance, or close by themselves.

<a id="userjudgment"></a>

## UserJudgment

```yaml
UserJudgment:
  user_judgment_id: string
  task_id: string
  change_unit_id: string | null
  status: proposed | pending_user | resolved | deferred | rejected | blocked | superseded
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | final_acceptance | residual_risk_acceptance | cancellation
  presentation: short
  question: string
  options: UserJudgmentOption[]
  context: UserJudgmentContext
  affected_refs: StateRecordRef[]
  required_for: next_action | write | run | close | acceptance | risk
  resolution: UserJudgmentResolution | null
  expires_at: string | null
  created_at: string
  updated_at: string
  resolved_at: string | null

UserJudgmentOption:
  option_id: string
  label: string
  meaning: approve | reject | defer | choose | cancel
  consequence: string

UserJudgmentContext:
  why_now: string
  source_refs: StateRecordRef[]
  evidence_summary_ref: StateRecordRef | null
  what_user_is_judging: string
  why_agent_cannot_decide: string
  no_decision_consequence: string

UserJudgmentResolution:
  selected_option_id: string
  answer: RecordUserJudgmentPayload
  note: string | null
```

`judgment_kind` is the canonical decision-type field. Rendered labels, including localized labels, are not schema values. `presentation=short` is the active MVP presentation. Expanded presentation bodies are not active API schema.

`UserJudgmentResolution.selected_option_id` and `UserJudgmentResolution.note` are stored copies of the canonical request-level `RecordUserJudgmentRequest.selected_option_id` and `RecordUserJudgmentRequest.note` fields. `RecordUserJudgmentPayload` carries decision-specific answer details and must not repeat the option selection or request note.

<a id="userjudgmentcandidate"></a>

## UserJudgmentCandidate

```yaml
UserJudgmentCandidate:
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | final_acceptance | residual_risk_acceptance | cancellation
  presentation: short
  question: string
  options: UserJudgmentOption[]
  context: UserJudgmentContext
  affected_refs: StateRecordRef[]
  required_for: next_action | write | run | close | acceptance | risk
```

A candidate is not a committed `user_judgment` row. It has no `StateRecordRef`, satisfies no gate, and creates no sensitive-action permission, final acceptance, residual-risk acceptance, evidence, Write Authorization, or close state.

```yaml
RecordUserJudgmentPayload:
  sensitive_action_scope: SensitiveActionScope | null
  accepted_result_refs: StateRecordRef[]
  cancellation_reason: string | null
```

For `judgment_kind=sensitive_approval`, `sensitive_action_scope` must match the pending judgment. Sensitive approval must not directly store `AuthorizedAttemptScope` as its approval scope; product-file Write Authorization remains a separate `prepare_write`/`record_run` contract. For `final_acceptance`, `accepted_result_refs` names the visible basis being accepted. For `cancellation`, `cancellation_reason` is required.

<a id="acceptedriskinput"></a>

## AcceptedRiskInput

```yaml
AcceptedRiskInput:
  visible_risk_ref: StateRecordRef
  accepted: boolean
  user_note: string | null
```

`AcceptedRiskInput` is valid only with `judgment_kind=residual_risk_acceptance`. The `visible_risk_ref` must point to a visible close-relevant `blocker` for the same Task. It does not create a standalone residual-risk record.

<a id="current-position-display-schemas"></a>

## Current-Position Display Schemas

```yaml
CloseBlocker:
  category: task | open_run | scope | user_judgment | sensitive_approval | write_compatibility | baseline | surface_capability | evidence | artifact_availability | final_acceptance | residual_risk_visibility | residual_risk_acceptance | cancellation | supersession | recovery
  code: ErrorCode
  message: string
  related_refs: StateRecordRef[]
  required_judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | final_acceptance | residual_risk_acceptance | cancellation | null
  next_action: string

NextActionSummary:
  action_kind: ask_user | update_scope | prepare_write | implement | request_acceptance | close_task | idle
  summary: string
  required_tool: harness.intake | harness.status | harness.update_scope | harness.prepare_write | harness.stage_artifact | harness.record_run | harness.request_user_judgment | harness.record_user_judgment | harness.close_task | null
  related_refs: StateRecordRef[]
  blocker_code: ErrorCode | null
```

`CloseBlocker` is a structured blocker result. Prose-only status text, reports, or rendered views are not blocker results. For `harness.close_task intent=complete`, Core calculates blocker categories in the deterministic order owned by [Core Model](../core-model.md#close_task). `cancellation` and `supersession` categories describe conflicts with those terminal intents; they are not successful-completion evidence and must not be mixed with `completed_self_checked` or `completed_with_risk_accepted`.

<a id="nextactionsummary"></a>

## NextActionSummary

`NextActionSummary` is defined in [Current-Position Display Schemas](#current-position-display-schemas). The active `action_kind` values are exactly:

```text
ask_user | update_scope | prepare_write | implement | request_acceptance | close_task | idle
```

<a id="validatorresult"></a>

## ValidatorResult

```yaml
ValidatorResult:
  validator_id: surface_capability_check
  validator_kind: capability
  status: passed | warning | failed | blocked | skipped
  guarantee_level: cooperative | detective
  checked_at: string
  target:
    task_id: string | null
    change_unit_id: string | null
    run_id: string | null
    artifact_id: string | null
  summary: string
  findings:
    - code: string
      severity: info | warning | error | blocker
      message: string
      path: string | null
      artifact_ref: ArtifactRef | null
  blocked_reasons: string[]
  suggested_next_action: string | null
```

The active stable validator ID is `surface_capability_check`. Validator output can affect blockers, fallback behavior, and guarantee display only through the active owner path named by the result, such as `CloseBlocker.category=surface_capability` when capability is truly the issue. A `status=blocked` result or `findings.severity=blocker` is not a design-policy blocker, does not activate `design_gate` or `design_policy`, and does not block close by severity alone. It does not create Write Authorization, user judgment, evidence, final acceptance, residual-risk acceptance, or close.

`ValidatorResult.status=passed` is the only validator status that can support the verified capability state used for `detective` display. `skipped`, `warning`, `failed`, and `blocked` do not justify a stronger label. For changed-path detection specifically, the profile-level `changed_path_detection_verification` value must be `passed`; `not_run`, legacy `planned_not_run` wording, `failed`, and `stale` keep the display `cooperative` or produce `CAPABILITY_INSUFFICIENT`, depending on the method.

<a id="sensitive-categories"></a>

## Sensitive Categories

Sensitive categories explain why sensitive-action approval may be needed for a product-file write. They are product-write classifications inside `AuthorizedAttemptScope`, not the approval scope for commands, hosts, dependencies, secret handles, deployments, destructive actions, or system access. They do not decide product, technical, scope, QA, verification, acceptance, residual-risk, or policy questions. They also do not claim that Harness observed commands, network effects, or secret access. The active `SensitiveCategory` enum is:

```text
auth_change
permission_model_change
schema_change
dependency_change
public_api_change
destructive_write
production_config_change
ci_cd_change
infra_or_deployment_change
privacy_or_pii_change
data_export
telemetry_or_logging_change
license_or_compliance_change
billing_or_cost_change
model_or_prompt_policy_change
policy_override
```

<a id="current-mvp-value-sets"></a>

## Current MVP Value Sets

These values are active current MVP schema values. Method-level capability and access-class checks may still reject a value in a concrete request. Values not listed here are not active current MVP values. This table is the copyable current MVP value set for first validators. Rendered labels are not canonical schema values. Public `ErrorCode` values are owned by [API Errors](errors.md), not by this table.

| Field | Current MVP values |
|---|---|
| Active method set | `harness.intake`, `harness.status`, `harness.update_scope`, `harness.prepare_write`, `harness.stage_artifact`, `harness.record_run`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.close_task` |
| `ToolEnvelope.actor_kind` | `user`, `lead_agent` |
| `response_kind` | `result`, `rejected`, `dry_run` |
| `effect_kind` | `read_only`, `core_committed`, `staging_created`, `no_effect` |
| Local API access classes | `read_status`, `core_mutation`, `write_authorization`, `run_recording`, `artifact_registration`, `artifact_read` |
| `LocalSurfaceRegistration.transport_kind` | `local_mcp_stdio`, `local_http` |
| `LocalSurfaceRegistration.local_access_posture` | `registered_local`, `unavailable`, `mismatch`, `revoked` |
| `LocalSurfaceRegistration.status` | `active`, `disabled`, `stale`, `revoked` |
| `VerifiedSurfaceContext.failure_reason` | `unavailable`, `mismatch`, `revoked`, `insufficient_capability`, `null` |
| `capability_profile.surface_id` | `reference-local-mcp` |
| `capability_profile.surface_status` | same values as `LocalSurfaceRegistration.status` |
| `capability_profile.local_access_posture` | same values as `LocalSurfaceRegistration.local_access_posture` |
| `capability_profile.changed_path_detection_verification` | `not_run`, `passed`, `failed`, `stale` |
| `capability_profile.guarantee_level_default` | `cooperative` |
| `capability_profile.guarantee_level_max_when_verified` | `detective` |
| `IntakeRequest.requested_mode` | `advisor`, `direct`, `work`, `auto` |
| `StateSummary.mode` and persisted `tasks.mode` | `advisor`, `direct`, `work` |
| `Task.lifecycle_phase` and `StateSummary.lifecycle_phase` | `shaping`, `ready`, `executing`, `waiting_user`, `blocked`, `completed`, `cancelled`, `superseded` |
| `Task.result` and `StateSummary.result` | `none`, `advice_only`, `completed`, `cancelled`, `superseded` |
| `Task.close_reason` and `StateSummary.close_reason` | `none`, `completed_self_checked`, `completed_with_risk_accepted`, `cancelled`, `superseded` |
| `StatusResponse.close_state` | `none`, `ready`, `blocked`, `closed`, `cancelled`, `superseded` |
| `CloseTaskResponse.close_state` | `ready`, `blocked`, `closed`, `cancelled`, `superseded` |
| `CloseTaskRequest.intent` | `check`, `complete`, `cancel`, `supersede` |
| `CloseTaskRequest.close_reason` | same values as `Task.close_reason`, plus `null`; method behavior determines which values are valid for each `intent` |
| `StateSummary.assurance_level` | `none`, `self_checked` |
| `StateSummary.gates.scope_gate` | `not_required`, `required`, `pending`, `passed`, `failed`, `blocked` |
| `StateSummary.gates.decision_gate` | `not_required`, `required`, `pending`, `resolved`, `deferred`, `blocked` |
| `StateSummary.gates.approval_gate` | `not_required`, `required`, `pending`, `granted`, `denied`, `expired` |
| `StateSummary.gates.evidence_gate` | `not_required`, `none`, `partial`, `sufficient`, `stale`, `blocked` |
| `StateSummary.gates.acceptance_gate` | `not_required`, `required`, `pending`, `accepted`, `rejected` |
| `StateRecordRef.record_kind` | `project`, `task`, `change_unit`, `run`, `write_authorization`, `user_judgment`, `evidence_summary`, `blocker` |
| `ArtifactRef.kind` | `diff`, `log`, `screenshot`, `checkpoint`, `other` |
| `ArtifactRef.produced_by` | `lead_agent`, `harness` |
| `ArtifactRef.retention_class` | `task`, `project`, `temporary` |
| `ArtifactRelationOwner.record_kind` | `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` |
| `ArtifactInput.source_kind` | `staged_artifact`, `existing_artifact` |
| `EvidenceCoverageItem.coverage_state` | `supported`, `unsupported`, `partial`, `not_applicable`, `stale`, `blocked` |
| `EvidenceSummary.status` | `not_required`, `none`, `partial`, `sufficient`, `stale`, `blocked` |
| `AuthorizedAttemptScope.guarantee_level` | `cooperative`, `detective` |
| `SensitiveActionScope.action_kind` | `product_file_write`, `dependency_change`, `destructive_command`, `network_access`, `secret_access`, `deployment`, `system_access`, `other` |
| `SensitiveActionScope.capability_claim` | `cooperative_only`, `observed_by_surface`, `not_observable` |
| `WriteAuthorizationSummary.status` | `active`, `consumed`, `expired`, `stale`, `revoked` |
| `WriteAuthoritySummary.approval_status` | `not_required`, `required`, `pending`, `granted`, `denied`, `expired`, `unknown` |
| `RunSummary.kind` | `shaping_update`, `implementation`, `direct` |
| `RunSummary.status` | `completed`, `interrupted`, `blocked`, `violation` |
| `UserJudgment.status` | `proposed`, `pending_user`, `resolved`, `deferred`, `rejected`, `blocked`, `superseded` |
| `UserJudgment.judgment_kind` | `product_decision`, `technical_decision`, `scope_decision`, `sensitive_approval`, `final_acceptance`, `residual_risk_acceptance`, `cancellation` |
| `UserJudgment.presentation` | `short` |
| `UserJudgment.required_for` | `next_action`, `write`, `run`, `close`, `acceptance`, `risk` |
| `UserJudgmentCandidate.judgment_kind` | same values as `UserJudgment.judgment_kind` |
| `UserJudgmentCandidate.presentation` | `short` |
| `UserJudgmentCandidate.required_for` | same values as `UserJudgment.required_for` |
| `UserJudgmentOption.meaning` | `approve`, `reject`, `defer`, `choose`, `cancel` |
| `ArtifactRef.redaction_state` | `none`, `redacted`, `secret_omitted`, `blocked` |
| `CloseBlocker.category` | `task`, `open_run`, `scope`, `user_judgment`, `sensitive_approval`, `write_compatibility`, `baseline`, `surface_capability`, `evidence`, `artifact_availability`, `final_acceptance`, `residual_risk_visibility`, `residual_risk_acceptance`, `cancellation`, `supersession`, `recovery` |
| `CloseBlocker.required_judgment_kind` | same values as `UserJudgment.judgment_kind`, plus `null` |
| `NextActionSummary.action_kind` | `ask_user`, `update_scope`, `prepare_write`, `implement`, `request_acceptance`, `close_task`, `idle` |
| `NextActionSummary.required_tool` | active method set values, plus `null` |
| `GuaranteeDisplay.level` | `cooperative`, `detective` |
| `ValidatorResult.validator_id` | `surface_capability_check` |
| `ValidatorResult.validator_kind` | `capability` |
| `ValidatorResult.status` | `passed`, `warning`, `failed`, `blocked`, `skipped` |
| `ValidatorResult.guarantee_level` | `cooperative`, `detective` |
| `ValidatorResult.findings.severity` | `info`, `warning`, `error`, `blocker` |
| `SensitiveCategory` | `auth_change`, `permission_model_change`, `schema_change`, `dependency_change`, `public_api_change`, `destructive_write`, `production_config_change`, `ci_cd_change`, `infra_or_deployment_change`, `privacy_or_pii_change`, `data_export`, `telemetry_or_logging_change`, `license_or_compliance_change`, `billing_or_cost_change`, `model_or_prompt_policy_change`, `policy_override` |

For `GuaranteeDisplay.level`, `cooperative` is the default current MVP value. `detective` is also a current MVP value, but only where the active surface can honestly observe the relevant fact and the relevant capability check has actually passed. For the baseline profile, `detective` requires `changed_path_detection_verification=passed` and is limited to verified changed-path detection scope. Neither value means OS permission, arbitrary-tool sandboxing, tamper-proof storage, pre-tool blocking, or isolation.

Schema Core intentionally does not reserve inactive enum members inside active tables. User-judgment kinds, gate fields, validator IDs, actor/source values such as `captured_artifact`, stronger guarantee labels, command/network/secret observation or blocking fields not listed here, and API methods not listed in this section are inactive until promoted by an owner document and added to the relevant active owner contract.

<a id="later-candidate-value-names"></a>

## Later Candidate Value Names

Later candidate value names stay catalog-only in [Later Candidate Index](../../later/index.md#later-schema-candidates) until a promoted owner adds exact active fields, value sets, validators, fallback behavior, and proof expectations here or in another active owner document. This active API reference intentionally does not define later schema bodies.
