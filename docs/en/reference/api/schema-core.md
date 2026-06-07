# API Schema Core

## What this document helps you do

Use this reference for the active current MVP method-name set, shared API shapes, and schema value sets: the tool envelope, common response, `ArtifactRef`, `StateRecordRef`, `UserJudgment`, Write Authorization summary, evidence summary, run summary, close blockers, next-action summary, current MVP enum values, profile-gated display value names, and later candidate value-name boundaries.

This document describes future Harness Server behavior for planning and review. It does not mean the current documentation repository implements an MCP server. Future schema candidates stay in [Later Candidate Index](../../later/index.md#later-schema-candidates).

## Contract Map

| Need | Owner |
|---|---|
| Exact active method-name value set and shared schema value sets | This document |
| Method request/response behavior for active methods | [MVP API](mvp-api.md) |
| Public errors, precedence, idempotency, blocked behavior, and stale-state behavior | [API Errors](errors.md) |
| Core state semantics and lifecycle meaning | [Core Model Reference](../core-model.md) |
| Storage tables, JSON `TEXT`, enum hardening, and artifact persistence | [Storage](../storage.md) |
| Security guarantee meanings | [Security Reference](../security.md) |
| Future API/schema candidates | [Later Candidate Index](../../later/index.md#later-schema-candidates) |

## Schema Notation Convention

The YAML-like blocks in this document are normative schema notation, not examples unless marked as examples.

- `field: Type` means the field is required and non-null.
- `field: Type | null` means the field is required and may be JSON `null`.
- `Type[]` means the field is present and contains an array; an empty array is written as `[]`.
- `a | b | c` is a closed active enum for that field unless the surrounding section explicitly marks the value as profile-gated or reserved.
- Unlisted fields are rejected outside an explicitly named extension container.

Storage validation is a separate owner boundary. API payloads and API-shaped stored JSON validate against this API reference first; DDL, storage-only JSON, defaults, locks, and migrations validate against [Storage](../storage.md).

The [Current MVP Value Sets](#current-mvp-value-sets) section owns the exact active method-name set and the active schema enum values declared by this document. Method-specific behavior remains with [MVP API](mvp-api.md), and public `ErrorCode` taxonomy remains with [API Errors](errors.md).

<a id="tool-envelope"></a>

## Tool Envelope

Every public tool request carries `ToolEnvelope`. State-changing tools require a non-null `idempotency_key` and a current `expected_state_version`. `harness.status` is read-only and may set `expected_state_version` to `null`.

```yaml
ToolEnvelope:
  request_id: string
  idempotency_key: string | null
  expected_state_version: integer | null
  project_id: string
  task_id: string | null
  surface_id: string
  actor_kind: user | lead_agent | evaluator | operator
  dry_run: boolean
```

Envelope fields route and audit the call. `surface_id` does not grant capability, write authority, local access, user judgment, sensitive-action permission, final acceptance, residual-risk acceptance, or close.

<a id="local-surface-access-values"></a>

## Local Surface Access Values

Local surface access values describe Harness API compatibility. They are not OS permissions, sandbox boundaries, tamper-proof guarantees, universal pre-tool blocking, or isolation.

`surfaces.local_access_posture` is a closed current MVP value set:

| Value | Meaning |
|---|---|
| `registered_local` | The caller/transport matches the registered local surface posture for this project closely enough for the API owner to evaluate the requested access class. |
| `unavailable` | Required MCP/Core or surface reachability cannot currently be established. |
| `mismatch` | A reachable caller/transport does not match the registered local surface posture for the project. |
| `revoked` | Local access for the registered surface was explicitly revoked and must not be used until a new valid registration replaces it. |

`surfaces.status` is a closed current MVP value set:

| Value | Meaning |
|---|---|
| `active` | The registered surface may be considered for current API access checks. |
| `disabled` | The surface remains recorded but must not be used for current API access. |
| `stale` | The surface registration or capability posture must be refreshed before current API access can rely on it. |
| `revoked` | The surface registration is no longer valid for current API access. |

The active local API access-class labels are `read_status`, `core_mutation`, `write_authorization`, `run_recording`, `artifact_registration`, and `artifact_read`. Method-level conditions for these classes are owned by [MVP API](mvp-api.md#shared-request-rules); public error selection is owned by [API Errors](errors.md).

<a id="common-response"></a>

## Common Response

```yaml
ToolResponseBase:
  request_id: string
  idempotency_key: string | null
  project_id: string
  task_id: string | null
  state_version: integer
  dry_run: boolean
  errors: ToolError[]
  validator_results: ValidatorResult[]
  events: EventRef[]

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

`ToolResponseBase.state_version` is the resulting affected-scope version for a committed mutation, or the current readable/would-be affected version for read-only and dry-run responses. `dry_run=true` creates no current records, events, artifacts, evidence summaries, Write Authorizations, close state, or idempotency replay rows.

## State Summary

```yaml
StateSummary:
  mode: advisor | direct | work
  lifecycle_phase: shaping | ready | executing | waiting_user | blocked | completed | cancelled | superseded
  result: none | advice_only | completed | cancelled | superseded
  close_reason: none | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  assurance_level: none | self_checked
  gates:
    scope_gate: not_required | required | pending | passed | failed | blocked
    decision_gate: not_required | required | pending | resolved | deferred | blocked
    approval_gate: not_required | required | pending | granted | denied | expired
    evidence_gate: not_required | none | partial | sufficient | stale | blocked
    acceptance_gate: not_required | required | pending | accepted | rejected

GuaranteeDisplay:
  level: cooperative | detective
  notes: string[]
```

`StateSummary.mode` mirrors persisted `tasks.mode` and is always a concrete task mode. `auto` is not a stored mode, displayed task mode, or status-summary mode. `StateSummary.lifecycle_phase` mirrors persisted `Task.lifecycle_phase`. `intake` is an API method and start-handling step, not a lifecycle value. `StateSummary.result` mirrors coarse `Task.result`; Run failure, violation, evidence gaps, and blockers remain in Run status, evidence state, blockers, or current Task state instead of becoming a terminal Task result.

Task mode values have these reader-facing meanings:

- `advisor`: advice, review, or planning without product writes.
- `direct`: small direct change.
- `work`: tracked work.

`IntakeRequest.requested_mode=auto` is only an intake input asking the server to classify the request. The server must resolve it to exactly one of `advisor`, `direct`, or `work` before writing `tasks.mode`, producing `StateSummary.mode`, or returning intake/status summaries.

Rendered labels are not canonical schema values. `GuaranteeDisplay.level` is a display claim about the documented surface capability and proof level; it does not grant permission or state authority. The active MVP guarantee-display values are `cooperative` and `detective`. `preventive` and `isolated` are profile-gated display value names, not default active MVP guarantees.

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

`ArtifactRef` points to a durable evidence file registered through Harness storage. It is not a caller-supplied arbitrary path.

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
  produced_by: lead_agent | evaluator | operator | harness
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

`ArtifactInput` is accepted by `harness.record_run` only as a staging, capture-adapter, or existing-artifact handle. It never grants arbitrary file read authority.

```yaml
ArtifactInput:
  artifact_input_id: string
  source_kind: staged_file | capture_adapter | existing_artifact
  relation: string
  staged_uri: string | null
  capture_ref: string | null
  existing_artifact_ref: ArtifactRef | null
  display_name: string | null
  content_type: string
  expected_sha256: string | null
  expected_size_bytes: integer | null
```

Exactly one source field must match `source_kind`. Invalid source shapes, caller-supplied arbitrary paths, raw secrets, tokens, and full sensitive logs are rejected before mutation.

<a id="evidence-and-pre-write-scope-schemas"></a>

## Evidence And Pre-Write Scope Schemas

```yaml
EvidenceCoverageItem:
  claim_or_criterion: string
  coverage_state: supported | unsupported | partial | not_applicable | stale | blocked
  supporting_state_refs: StateRecordRef[]
  supporting_artifact_refs: ArtifactRef[]
  gap_blocker_refs: StateRecordRef[]
  note: string | null

EvidenceSummary:
  evidence_summary_ref: StateRecordRef | null
  task_id: string
  change_unit_id: string | null
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
  related_user_judgment_refs: StateRecordRef[]
  guarantee_level: cooperative | detective

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

`EvidenceSummary` is the compact active evidence record. It is not a detailed evidence report, separate assurance result, final acceptance, residual-risk acceptance, or rendered view.

`AuthorizedAttemptScope` is the exact scope stored in `write_authorizations.attempt_scope_json` and later compared by `harness.record_run`. `WriteAuthorizationSummary.status` is the durable authorization lifecycle. `blocked` is not a Write Authorization status; blocked writes return blockers without a consumable authorization.

`intended_commands`, `intended_network`, and `intended_secret_scope` are declared intent/scope descriptors. They are not proof that the active MVP can observe command execution, network effects, secret access, blocking, or isolation. Observation support is profile-owned; unsupported observations must remain unverified or become capability blockers rather than active evidence.

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
  selected_option_id: string
  approval_scope: AuthorizedAttemptScope | null
  accepted_result_refs: StateRecordRef[]
  cancellation_reason: string | null
  note: string | null
```

For `judgment_kind=sensitive_approval`, `approval_scope` must match the pending judgment. For `final_acceptance`, `accepted_result_refs` names the visible basis being accepted. For `cancellation`, `cancellation_reason` is required.

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
  required_tool: harness.intake | harness.update_scope | harness.status | harness.prepare_write | harness.record_run | harness.request_user_judgment | harness.record_user_judgment | harness.close_task | null
  related_refs: StateRecordRef[]
  blocker_code: ErrorCode | null
```

`CloseBlocker` is a structured blocker result. Prose-only status text, reports, or rendered views are not blocker results.

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

The active stable validator ID is `surface_capability_check`. Validator output can affect blockers, fallback behavior, and guarantee display. It does not create Write Authorization, user judgment, evidence, final acceptance, residual-risk acceptance, or close.

<a id="sensitive-categories"></a>

## Sensitive Categories

Sensitive categories explain why sensitive-action approval may be needed. They do not decide product, technical, scope, QA, verification, acceptance, residual-risk, or policy questions.

```text
auth_change
permission_model_change
schema_change
dependency_change
public_api_change
destructive_write
network_write
external_service_write
secret_access
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

These values are valid without a promoted profile. Values not listed here are not default active MVP values. Rendered labels are not canonical schema values. Public `ErrorCode` values are owned by [API Errors](errors.md), not by this table.

| Field | Current MVP values |
|---|---|
| Active method set | `harness.intake`, `harness.update_scope`, `harness.status`, `harness.prepare_write`, `harness.record_run`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.close_task` |
| `ToolEnvelope.actor_kind` | `user`, `lead_agent`, `evaluator`, `operator` |
| Local API access classes | `read_status`, `core_mutation`, `write_authorization`, `run_recording`, `artifact_registration`, `artifact_read` |
| `surfaces.local_access_posture` | `registered_local`, `unavailable`, `mismatch`, `revoked` |
| `surfaces.status` | `active`, `disabled`, `stale`, `revoked` |
| `IntakeRequest.requested_mode` | `advisor`, `direct`, `work`, `auto` |
| `StateSummary.mode` and persisted `tasks.mode` | `advisor`, `direct`, `work` |
| `StateSummary.lifecycle_phase` | `shaping`, `ready`, `executing`, `waiting_user`, `blocked`, `completed`, `cancelled`, `superseded` |
| `StateSummary.result` | `none`, `advice_only`, `completed`, `cancelled`, `superseded` |
| `StateSummary.close_reason` | `none`, `completed_self_checked`, `completed_with_risk_accepted`, `cancelled`, `superseded` |
| `StatusResponse.close_state` | `none`, `ready`, `blocked`, `closed`, `cancelled`, `superseded` |
| `CloseTaskResponse.close_state` | `ready`, `blocked`, `closed`, `cancelled`, `superseded` |
| `StateSummary.assurance_level` | `none`, `self_checked` |
| `StateSummary.gates.scope_gate` | `not_required`, `required`, `pending`, `passed`, `failed`, `blocked` |
| `StateSummary.gates.decision_gate` | `not_required`, `required`, `pending`, `resolved`, `deferred`, `blocked` |
| `StateSummary.gates.approval_gate` | `not_required`, `required`, `pending`, `granted`, `denied`, `expired` |
| `StateSummary.gates.evidence_gate` | `not_required`, `none`, `partial`, `sufficient`, `stale`, `blocked` |
| `StateSummary.gates.acceptance_gate` | `not_required`, `required`, `pending`, `accepted`, `rejected` |
| `StateRecordRef.record_kind` | `project`, `task`, `change_unit`, `run`, `write_authorization`, `user_judgment`, `evidence_summary`, `blocker` |
| `ArtifactRef.kind` | `diff`, `log`, `screenshot`, `checkpoint`, `other` |
| `ArtifactRef.produced_by` | `lead_agent`, `evaluator`, `operator`, `harness` |
| `ArtifactRef.retention_class` | `task`, `project`, `temporary` |
| `ArtifactRelationOwner.record_kind` | `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` |
| `ArtifactInput.source_kind` | `staged_file`, `capture_adapter`, `existing_artifact` |
| `EvidenceCoverageItem.coverage_state` | `supported`, `unsupported`, `partial`, `not_applicable`, `stale`, `blocked` |
| `EvidenceSummary.status` | `not_required`, `none`, `partial`, `sufficient`, `stale`, `blocked` |
| `AuthorizedAttemptScope.intended_network.direction` | `read`, `write` |
| `AuthorizedAttemptScope.intended_secret_scope.access_kind` | `read`, `write` |
| `AuthorizedAttemptScope.guarantee_level` | `cooperative`, `detective` |
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
| Sensitive categories | `auth_change`, `permission_model_change`, `schema_change`, `dependency_change`, `public_api_change`, `destructive_write`, `network_write`, `external_service_write`, `secret_access`, `production_config_change`, `ci_cd_change`, `infra_or_deployment_change`, `privacy_or_pii_change`, `data_export`, `telemetry_or_logging_change`, `license_or_compliance_change`, `billing_or_cost_change`, `model_or_prompt_policy_change`, `policy_override` |

For `GuaranteeDisplay.level`, `cooperative` is the default current MVP value. `detective` is also a current MVP value, but only where the active surface can honestly observe the relevant fact. Neither value means OS permission, arbitrary-tool sandboxing, tamper-proof storage, pre-tool blocking, or isolation.

`qa_waiver` and `verification_risk_acceptance` are later/reserved user-judgment candidate names, not active current MVP `UserJudgment.judgment_kind` values. They remain catalog-only in [Later Candidate Index](../../later/index.md) until a future owner promotes exact active schema values, request behavior, fallback behavior, and proof expectations. The active `StateSummary.gates` object intentionally has no `design_gate`, `verification_gate`, or `qa_gate` field.

<a id="profile-gated-value-names"></a>

## Profile-Gated Value Names

These names may appear only when a promoted profile explicitly supports the corresponding guarantee. They are not default active MVP guarantees. `preventive` and `isolated` are profile-gated display values, not default active MVP guarantees.

| Field | Profile-gated value name | Requirement |
|---|---|---|
| `GuaranteeDisplay.level` | `preventive` | Requires explicit pre-tool blocking support for the covered operation, plus an owner-defined behavior, fallback, and proof path. |
| `GuaranteeDisplay.level` | `isolated` | Requires explicit isolation support for the covered boundary, plus a named boundary, owner-defined behavior, fallback, and proof path. |

Profile-gated display value names do not expand Write Authorization, validator, storage, or error behavior by themselves. Unsupported requests to use or display them remain capability or validation failures; they are not evidence that the stronger guarantee exists.

<a id="later-candidate-value-names"></a>

## Later Candidate Value Names

Later candidate value names stay catalog-only in [Later Candidate Index](../../later/index.md#later-schema-candidates) until a promoted owner adds exact active fields, value sets, validators, fallback behavior, and proof expectations here or in another active owner document.

This active API reference intentionally does not define later schema bodies. A later candidate name is not an active enum member, schema field, storage value, public method, validator requirement, close blocker, or guarantee value merely because it is cataloged for future work.

| Source | Active API boundary |
|---|---|
| Later schema extensions | Candidate names only; no active request, response, shared schema, or enum member. |
| Later design, verification, and QA gates | Candidate names only; no active `design_gate`, `verification_gate`, `qa_gate`, Manual QA gate, design-policy gate, verification response field, or QA response field. |
| Later design-policy categories and validators | Candidate names only; no active `CloseBlocker.category=design_policy`, design-policy waiver, design-policy validator family, or severity-based close blocker. |
| Later ref and artifact values | Candidate names only; no active `ArtifactRef`, `StateRecordRef`, storage, evidence, QA, export, or projection value. |
| Later template, fixture, conformance, operation, export, and diagnostic names | Candidate names only; no active API payload, runtime operation, error family, conformance-runner behavior, or close effect. |
