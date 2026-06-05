# API Schema Core

## What this document helps you do

Use this reference for the shared API shapes that support the MVP-1 methods in [MVP API](mvp-api.md): request envelopes, common responses, read-only resource schemas, shared refs, artifact inputs, user-judgment payloads, next-action summaries, and API staged value sets.

This document describes future Harness Server behavior for planning and review. It does not mean the current documentation repository implements an MCP server.

## Contract map

| Need | Section |
|---|---|
| Active MVP-1 tools | [MVP API](mvp-api.md) |
| Error codes, MVP-1 status/error conditions, precedence, idempotency, stale-state behavior | [Errors](errors.md) |
| Later/profile-gated schemas and methods | [Schema Later](schema-later.md) |
| Core Model state semantics | [Core Model Reference](../core-model.md) |
| Storage and DDL | [Storage](../storage.md) |
| Compact view behavior and template bodies | [Projection And Templates Reference](../projection-and-templates.md) and [Template Reference](../templates/README.md) |

## Schema notation convention

The YAML-like blocks in these API docs are normative schema notation, not examples unless called examples.

- `field: Type` means the field is required and non-null.
- `field: Type | null` means the field is required and may be JSON `null`.
- Optional fields must be named as optional in prose or profile-extension text.
- `Type[]` means an array whose items match `Type`; `[]` is present and empty.
- `one_of:` means exactly one listed branch is present.
- `a | b | c` is a closed enum unless the section explicitly labels it extensible.
- Unlisted fields are rejected outside an explicit extension container.
- Later/profile-gated enum values and branches are not valid for MVP-1 unless the owning profile is active.

Storage validation is a separate ownership boundary. API payloads and API-shaped stored JSON validate against this API reference first; storage-only JSON `TEXT`, DDL nullability, column defaults, and storage hardening validate against [Storage](../storage.md).

## Stage Profile Manifest

This manifest filters the API schemas by stage/profile. A field or enum existing in this reference does not make it active in an earlier stage.

| Stage/profile | Active API slice | Not active in that slice |
|---|---|---|
| Engineering Checkpoint | Minimal status/blocker read, one owner-valid setup path, active Task, active Change Unit/scope boundary, `harness.prepare_write`, one compatible `harness.record_run`, one artifact/evidence ref, structured status/blocker output, and narrow close-blocker check. | Full natural-language intake, stored user judgment path, full Evidence Manifest, detached verification, Manual QA, work acceptance, residual-risk acceptance, rich projections, export/recover, broad operations. |
| MVP-1 User Work Loop | Active method set owned by [MVP API](mvp-api.md#mvp-1-method-set), with next-safe-action output carried by `harness.status.next_actions`. The method set is exactly `harness.status`, `harness.intake`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.prepare_write`, `harness.record_run`, and `harness.close_task`. | Separate `harness.next`, detached verification launch/Eval, full Manual QA matrix, committed Approval hardening, export/recover, advanced connector APIs, broad operations, and detailed diagnostic projections. |
| Assurance Profile, Operations Profile, or later | Verification, Eval, Manual QA, waiver, full residual-risk acceptance, reconcile, validators, projection/report/export/recover, operations, and advanced connectors when owner docs promote them. | Not Engineering Checkpoint or minimum MVP-1 requirements. |

## Read-only resources

MCP resources are read-only views. They must not create Tasks, user judgments, projection jobs, reconciliations, evidence, QA, work acceptance, residual-risk acceptance, Write Authorizations, or close state.

Read-only resources use the three-part context model. `harness://status/card` is a user status card: a short readable view over current Core state and refs. Agent surfaces may use read-only resources to build an agent context packet: the minimal state, refs, freshness, and owner-section pointers needed for the next safe action. Core state remains the local authority record and only operational source of truth. Stale cards or projections are not authority, and rendered templates cannot create approval, acceptance, residual-risk acceptance, evidence, or close readiness.

### Engineering Checkpoint resources

| Resource | Profile meaning |
|---|---|
| `harness://project/current` | Current registered project identity and local MCP availability facts. |
| `harness://task/active` | Active Task pointer, or explicit `none` / `unknown`, without creating a Task. |
| `harness://task/{task_id}` | Current Task state for the narrow authority loop. |
| `harness://task/{task_id}/summary` | Optional compact Task status/blocker summary. |
| `harness://status/card` | Optional compact current-position user status card derived from current Core state and refs. |

### MVP-1 resources

| Resource | Profile meaning |
|---|---|
| `harness://task/{task_id}/user-judgments` | Active, resolved, deferred, and blocked `user_judgment` summaries. |
| `harness://task/{task_id}/judgment-context` | Minimum current context needed for a user judgment. |

The MVP-1 evidence and close-readiness path can also be displayed through the exact compact view set `status-card`, `agent-context-packet`, `judgment-request`, `run-evidence-summary`, and `close-result`, or through `harness.status`, `harness://task/{task_id}/summary`, or `harness://status/card` when the output is derived from current Core state and refs. Exact compact-view behavior and template bodies stay with [Projection And Templates Reference](../projection-and-templates.md) and [Template Reference](../templates/README.md).

### Later resources

Assurance, operations, and diagnostic resources such as evidence-manifest reads, report reads, bundle reads, design maps, Journey views, and broad projection resources are later/profile-gated. See [Schema Later](schema-later.md#later-read-only-resources).

## Tool envelope

Every public tool request carries an envelope. State-changing tools require a non-null `idempotency_key` and `expected_state_version`. Read-only tools accept the same envelope for tracing; they may set `expected_state_version` to `null`.

```yaml
ToolEnvelope:
  request_id: string
  idempotency_key: string | null
  expected_state_version: integer | null
  project_id: string
  task_id: string | null
  surface_id: string
  run_id: string | null
  actor_kind: user | lead_agent | evaluator | operator
  dry_run: boolean
```

Envelope fields are routing and audit claims. They do not authorize a surface to change state outside Core, and they do not prove user judgment, sensitive-action permission, work acceptance, Manual QA, or detached verification independence.

For any request that needs a primary Task, Core resolves it in this order: tool-specific `task_id`, `ToolEnvelope.task_id`, then active Task resolution. Task-scoped mutations compare `expected_state_version` against `tasks.state_version`. Project-scoped mutations with no resolved primary Task compare it against `project_state.state_version`.

## MCP boundary and caller trust

The Engineering Checkpoint/default posture is local-only exposure for a registered project surface. Local-only means a local process, local socket, or localhost-loopback connection for the expected local user/profile. It excludes unauthenticated shared endpoints, non-loopback binds, forwarded/tunneled endpoints, cloud/CI relays, cross-user sockets/directories, and remote callers unless a registered connector profile proves a stronger posture.

Public schemas may carry display-safe access material class, bind/reachability posture, freshness, profile refs, conformance/operator-check refs, or safe handles/fingerprints. They must not carry raw tokens, secrets, private configuration values, omitted secret values, or blocked payload bytes.

If Core cannot be reached, no authoritative Core response exists; report `MCP_UNAVAILABLE` or diagnostic `MCP_SERVER_UNAVAILABLE`. If Core or an operator can classify a reachable local caller/access path as outside the registered profile, use `LOCAL_ACCESS_MISMATCH` with display-safe details. User-facing behavior for Core unavailable, local access denied, unsupported surface, and stale state is owned by [Errors: MVP-1 guarantee and status taxonomy](errors.md#mvp-1-guarantee-and-status-taxonomy).

## Common response

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
  projection_jobs: ProjectionJobRef[]

ToolError:
  code: ErrorCode
  message: string
  retryable: boolean
  details: object
```

`ToolError.message` should follow the honest user-facing message patterns in [Errors](errors.md#mvp-1-guarantee-and-status-taxonomy) when one of the MVP-1 status/error conditions applies.

In Engineering Checkpoint and MVP-1, `projection_jobs` is present for envelope compatibility and is normally `[]`. It does not require a `projection_jobs` storage table. Durable projection jobs are Operations Profile or profile-promoted storage.

`dry_run=true` validates and returns diagnostics or a transition plan but does not mutate current records, append events, register artifacts, create consumable Write Authorizations, enqueue projection jobs, create/update idempotency replay rows, or reserve an `idempotency_key`.

For state-changing operations, `ToolResponseBase.state_version` is the primary affected scope's resulting version: `tasks.state_version` after a task-scoped mutation, or `project_state.state_version` after a project-scoped mutation with no resolved primary Task. Read-only and dry-run responses return the current version for the primary read scope or would-be affected scope.

## Shared schemas

```yaml
EventRef:
  event_id: string
  event_seq: integer
  event_type: string
  task_id: string | null
  state_version: integer

ProjectionJobRef:
  projection_job_id: string
  projection_kind: ProjectionKind
  target_ref: string
  projection_version: integer

StateSummary:
  mode: advisor | direct | work
  lifecycle_phase: intake | shaping | ready | executing | verifying | qa | waiting_user | blocked | completed | cancelled
  result: none | advice_only | passed | failed | cancelled
  close_reason: none | completed_verified | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  assurance_level: none | self_checked | detached_verified
  gates:
    scope_gate: not_required | required | pending | passed | failed | blocked
    decision_gate: not_required | required | pending | resolved | deferred | blocked
    approval_gate: not_required | required | pending | granted | denied | expired
    design_gate: not_required | required | pending | passed | partial | waived | stale | blocked
    evidence_gate: not_required | none | partial | sufficient | stale | blocked
    verification_gate: not_required | required | pending | passed | failed | waived_by_user | blocked
    qa_gate: not_required | required | pending | passed | failed | waived
    acceptance_gate: not_required | required | pending | accepted | rejected
```

`EventRef.state_version` is the affected-scope resulting version after that event. It is not an event ordering key; event ordering uses `event_seq`.

`StateSummary.mode` values stay `advisor`, `direct`, and `work`. User-facing surfaces may render them as advice/read-only work, small direct work, and tracked work. Those labels are display text, not enum values.

### ProjectionKind support

`ProjectionKind` is extensible but profile-gated:

| Support class | Values | Requirement |
|---|---|---|
| Core status output | none required | Engineering Checkpoint can expose status/blocker output without persisted Markdown projection jobs. |
| MVP-1 compact views | No persisted `ProjectionKind` is required. Exact compact-view names and behavior are owned by [Projection And Templates Reference](../projection-and-templates.md#mvp-1-view-set) and [Template Reference](../templates/README.md#mvp-1-template-set). | These views may satisfy MVP-1 without full template rendering. `TASK` and `DIRECT-RESULT` are later/full-profile or compatibility projections. |
| Assurance reports | `APR`, `MANUAL-QA` | Only when the matching approval, Manual QA, waiver, verification, or assurance profile is active. |
| Operations/export reports | `EXPORT` | Only when export, release-handoff, or operations report profile is active. |
| Future/diagnostic projections | `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `TDD-TRACE`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT`, `DEC`, `DESIGN`, `JOURNEY-CARD` | Enable only when an owner-promoted later profile is in scope. |

Projection support never creates state, evidence, QA, verification, approval, work acceptance, residual-risk acceptance, close readiness, close authority, or Write Authorization.

## Sensitive Categories

Sensitive categories are approval-risk labels, not a command language:

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

A single intended write may carry more than one category. The category explains why sensitive-action permission may be needed; it does not resolve product, architecture, security, QA, verification, work acceptance, residual-risk acceptance, or policy judgment.

## ArtifactRef

An artifact ref points to a durable evidence file registered in the artifact store. Artifact registration is not a loose file dump: Core validates staging/capture source, stored-byte integrity, `redaction_state`, and Task-scoped owner relation before returning an `ArtifactRef`.

```yaml
ArtifactRef:
  artifact_id: string
  kind: diff | log | screenshot | checkpoint | bundle | manifest | qa_capture | export_component | design_probe | prototype | architecture_scan | decision_context | other
  uri: string
  sha256: string
  size_bytes: integer
  content_type: string
  redaction_state: none | redacted | secret_omitted | blocked
  task_id: string
  run_id: string | null
  created_at: string
  produced_by: lead_agent | evaluator | operator | harness
  retention_class: task | project | export | temporary
```

For the reference implementation, `uri` uses `harness-artifact://{project_id}/{artifact_id}`. The local file path is resolved through storage, not by trusting an absolute path in the API payload.

`redaction_state` meanings:

| State | Meaning |
|---|---|
| `none` | Stored bytes are allowed evidence under current policy. |
| `redacted` | Sensitive content was removed before storage. |
| `secret_omitted` | Secret values or PII are intentionally omitted or replaced by handles. |
| `blocked` | Raw-payload storage was blocked; only a metadata notice may be exposed. |

For `redacted`, `secret_omitted`, and `blocked`, hashes and sizes describe the committed safe stored bytes, not a hidden original.

## Stage-Specific Active Value Sets

These tables are the active validator sets for staged implementations. Full later values stay exact in [Schema Later](schema-later.md), but callers and validators accept only values enabled by the active stage/profile.

| Field | Engineering Checkpoint / MVP-1 active values | Later-profile values | Future candidates |
|---|---|---|---|
| `ArtifactRef.kind`, `ArtifactInput.kind` | `diff`, `log`, `screenshot`, `checkpoint`, `other` | `bundle`, `manifest`, `qa_capture`, `export_component` | `design_probe`, `prototype`, `architecture_scan`, `decision_context` |

| Field | Engineering Checkpoint active owner kinds | MVP-1 active owner kinds | Later-profile owner kinds | Future candidates |
|---|---|---|---|---|
| `ArtifactInput.relation.record_kind` | `task`, `change_unit`, `run`, `evidence_summary`, `blocker` | `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` | `residual_risk`, `shared_design`, `evidence_manifest`, `eval`, `manual_qa_record`, `feedback_loop`, `tdd_trace`, `projection` | `journey_spine_entry` |
| `StateRecordRef.record_kind` | `task`, `change_unit`, `run`, `write_authorization`, `evidence_summary`, `blocker` | `task`, `change_unit`, `run`, `write_authorization`, `user_judgment`, `evidence_summary`, `blocker` | `approval`, `residual_risk`, `close_readiness`, `shared_design`, `feedback_loop`, `evidence_manifest`, `eval`, `manual_qa_record`, `tdd_trace`, `reconcile_item`, `projection` | `change_unit_dependency`, `journey_spine_entry`, `domain_term`, `module_map_item`, `interface_contract` |

MVP-1 sensitive-action approval uses `record_kind=user_judgment`. Committed `approval` refs are later-profile unless the Approval owner profile is active.

## ArtifactInput

```yaml
ArtifactInput:
  input_id: string
  source_kind: staged_file | existing_artifact
  existing_artifact_ref: ArtifactRef | null
  staged: StagedArtifactSource | null
  kind: diff | log | screenshot | checkpoint | bundle | manifest | qa_capture | export_component | design_probe | prototype | architecture_scan | decision_context | other
  redaction_state: none | redacted | secret_omitted | blocked
  produced_by: lead_agent | evaluator | operator | harness
  retention_class: task | project | export | temporary
  relation:
    task_id: string
    run_id: string | null
    record_kind: task | change_unit | run | user_judgment | evidence_summary | blocker | residual_risk | shared_design | evidence_manifest | eval | manual_qa_record | feedback_loop | tdd_trace | projection | journey_spine_entry
    record_id_hint: string | null
  description: string | null

StagedArtifactSource:
  staged_uri: string
  display_name: string | null
  content_type: string
  expected_sha256: string | null
  expected_size_bytes: integer | null
```

`source_kind=existing_artifact` requires `existing_artifact_ref` and `staged=null`. `source_kind=staged_file` requires `staged` and `existing_artifact_ref=null`.

`staged_uri` is a Harness staging locator or registered capture-adapter output, not permission to read arbitrary files. Tool responses return committed `ArtifactRef` values, never staged locators as authority.

## StateRecordRef

```yaml
StateRecordRef:
  record_kind: task | change_unit | run | approval | write_authorization | user_judgment | evidence_summary | blocker | residual_risk | close_readiness | shared_design | domain_term | module_map_item | interface_contract | feedback_loop | evidence_manifest | eval | manual_qa_record | tdd_trace | change_unit_dependency | reconcile_item | projection
  record_id: string
  projection_path: string | null
```

`record_kind=user_judgment` is the canonical MVP-1 ref kind for user-owned judgments, including sensitive-action approval, work acceptance, and residual-risk acceptance judgments. MVP-1 evidence coverage and blockers use `record_kind=evidence_summary` and `record_kind=blocker`; durable evidence bytes use `ArtifactRef`. `record_kind=approval`, `record_kind=residual_risk`, `record_kind=close_readiness`, and `record_kind=projection` are later/profile-promoted or derived-view refs unless their owner profile is active. There is no standalone accepted-risk ref kind.

For `record_kind=projection`, `record_id` is the projection job identity when the Operations/projection profile is active. `projection_path` is optional display/recovery metadata, not an alternate key.

Derived-view refs such as `projection` or `close_readiness` identify a readable view or later/profile-promoted display record. They do not replace the owner records behind the view. A stale derived-view ref must be refreshed or reconciled before a caller relies on it for a state-dependent action.

## Evidence and pre-write scope schemas

```yaml
EvidenceRefs:
  state_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]

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

ApprovalScope:
  sensitive_categories: string[]
  allowed_paths: string[]
  allowed_tools: string[]
  allowed_commands: string[]
  allowed_network_targets: string[]
  secret_scope: string[]
  baseline_ref: string | null

WriteAuthorizationSummary:
  write_authorization_id: string
  task_id: string
  change_unit_id: string
  basis_state_version: integer
  intended_operation: string
  intended_paths: string[]
  intended_tools: string[]
  sensitive_categories: string[]
  baseline_ref: string | null
  approval_refs: StateRecordRef[]
  user_judgment_refs: StateRecordRef[]
  guarantee_level: cooperative | detective | preventive | isolated
  status: active | consumed | expired | stale | revoked
  consumed_by_run_id: string | null
  created_at: string
  consumed_at: string | null

WriteAuthoritySummary:
  active_change_unit_ref: StateRecordRef | null
  write_authorization_ref: StateRecordRef | null
  allowed_paths: string[]
  allowed_tools: string[]
  allowed_commands: string[]
  allowed_command_classes: string[]
  allowed_network_targets: string[]
  secret_scope: string[]
  sensitive_categories: string[]
  approval_status: not_required | required | pending | granted | denied | expired | unknown
  baseline_ref: string | null
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]
  note: "Autonomy Boundary is judgment latitude, not a pre-write scope check."
```

`WriteAuthorizationSummary.approval_refs` is empty in minimum MVP-1. Resolved sensitive-action approval user judgments appear in `user_judgment_refs`; committed Approval refs appear only when the Approval owner profile is active.

`WriteAuthorizationSummary` and `WriteAuthoritySummary` are API/internal names. MVP-1 user-facing displays should call this a pre-write scope check first. Fields such as `allowed_paths`, `allowed_tools`, `decision=allowed`, and `status=active` describe Harness compatibility for the cooperative record/check only; they do not mean OS permission, sandboxing, tamper-proof enforcement, preventive blocking, or isolation. `allowed` belongs to `PrepareWriteResponse.decision`. `blocked` has no authorization row or lifecycle value.

`EvidenceSummary` is the active MVP-1 compact evidence contract. `status` uses exactly `not_required`, `none`, `partial`, `sufficient`, `stale`, and `blocked`; item coverage uses exactly `supported`, `unsupported`, `partial`, `not_applicable`, `stale`, and `blocked`. It is Core-owned state used by status and close checks. It is not a full Evidence Manifest, detached verification result, Manual QA record, work acceptance, residual-risk acceptance, or projection.

## UserJudgment

The MVP-1 judgment model is small. Users see one of five display labels; API payloads carry compact `judgment_type` and `presentation`.

```yaml
UserJudgment:
  user_judgment_id: string
  task_id: string
  change_unit_id: string | null
  status: proposed | pending_user | resolved | deferred | rejected | blocked | superseded
  judgment_type: product_choice | technical_choice | sensitive_action_approval | work_acceptance | residual_risk_acceptance
  presentation: short | full
  display_label: Product/UX judgment | Technical judgment | Sensitive action approval | Work acceptance | Residual risk acceptance
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary
  what_user_is_judging: string
  what_agent_may_decide_without_user: string[]
  affected_scope: UserJudgmentScope
  affected_gates: UserJudgmentGateRef[]
  affected_acceptance_criteria: UserJudgmentCriterionRef[]
  judgment_payload: UserJudgmentPayload
  resolution: UserJudgmentResolution | null
  expires_at: string | null
  created_at: string
  updated_at: string
  resolved_at: string | null

UserJudgmentScope:
  scope_refs: StateRecordRef[]
  product_areas: string[]
  files_or_paths: string[]
  acceptance_criteria_refs: StateRecordRef[]
  note: string | null

UserJudgmentGateRef:
  gate: scope_gate | decision_gate | approval_gate | design_gate | evidence_gate | verification_gate | qa_gate | acceptance_gate
  blocked_action: string | null

UserJudgmentCriterionRef:
  criteria_id: string
  statement: string

UserJudgmentResolution:
  selected_option_id: string | null
  judgment: RecordUserJudgmentPayload | null
  note: string | null
```

`presentation=short` is the default for small one-screen prompts. `presentation=full` is full-format Decision Packet-style presentation for complex or high-risk judgments. Presentation changes how much context is rendered; it does not change authority.

Legacy fields and methods map to the canonical names:

| Legacy | Canonical |
|---|---|
| `harness.request_user_decision` / `harness.record_user_decision` | `harness.request_user_judgment` / `harness.record_user_judgment` |
| `judgment_domain` | `judgment_type` plus display label |
| `decision_kind` | `judgment_type` plus route-specific validation |
| `decision_profile` | `presentation` |

### UserJudgment payload

```yaml
JudgmentOption:
  option_id: string
  label: string
  details: JudgmentOptionDetails | null

JudgmentOptionDetails:
  benefits: string[]
  costs: string[]
  risks: string[]
  reversibility: reversible | partially_reversible | irreversible | unknown
  confidence: low | medium | high
  suitable_when: string[]
  evidence_refs: EvidenceRefs

JudgmentRecommendation:
  option_id: string | null
  reason: string
  uncertainty: string | null
  when_to_revisit: string | null

JudgmentUserContext:
  minimum_context: string[]
  optional_pull_refs: StateRecordRef[]

UserJudgmentPayload:
  options: JudgmentOption[]
  recommendation: JudgmentRecommendation | null
  uncertainty: string | null
  deferral_consequence: string | null
  user_context: JudgmentUserContext | null
  approval_scope: ApprovalScope | null
  covers: string[]
  does_not_cover: string[]
  acceptance: AcceptanceJudgment | null
  residual_risk_acceptance: ResidualRiskAcceptanceJudgment | null
  separate_judgments_required: string[]

AcceptanceJudgment:
  result_ref: StateRecordRef | null
  result_summary: string
  evidence_status_refs: StateRecordRef[]
  verification_status_refs: StateRecordRef[]
  qa_status_refs: StateRecordRef[]
  residual_risk_visibility: ResidualRiskSummary
  does_not_replace: string[]

ResidualRiskAcceptanceJudgment:
  risk_refs: StateRecordRef[]
  accepted_scope: string[]
  acceptance_consequence: string
  follow_up_required: boolean
  follow_up: string | null
  evidence_refs: EvidenceRefs
```

For `judgment_type=sensitive_action_approval`, `approval_scope` is required. For `judgment_type=work_acceptance`, `acceptance` is required. For `judgment_type=residual_risk_acceptance`, `residual_risk_acceptance` is required. Later waiver and reconcile branches live in [Schema Later](schema-later.md#later-user-judgment-branches).

## NextActionSummary

```yaml
NextActionSummary:
  action_kind: ask_user | prepare_write | implement | launch_verify | record_eval | record_manual_qa | request_acceptance | close_task | reconcile | idle
  summary: string
  required_tool: string | null
  related_refs: StateRecordRef[]
  blocker_code: ErrorCode | null
```

MVP-1 uses `harness.status.next_actions`, not a separate `harness.next` method. Active MVP-1 values are:

```text
ask_user | prepare_write | implement | request_acceptance | close_task | idle
```

Later values `launch_verify`, `record_eval`, `record_manual_qa`, and `reconcile` are valid only when their owner profiles are active.

## Current-position display schemas

```yaml
AutonomyBoundarySummary:
  change_unit_id: string | null
  status: absent | proposed | active | exceeded | stale
  autonomy_profile: human_in_loop | afk_eligible | evaluator_only | read_only_advisor | null
  what_agent_may_do: string[]
  what_agent_may_decide_without_user: string[]
  what_requires_user_judgment: string[]
  stop_conditions: string[]
  triggered_stop_conditions: string[]
  related_user_judgment_refs: StateRecordRef[]

ResidualRiskSummary:
  status: none | visible | not_visible | accepted | blocked
  close_relevant_count: integer
  visible_refs: StateRecordRef[]
  not_visible_refs: StateRecordRef[]
  unaccepted_refs: StateRecordRef[]
  accepted_refs: StateRecordRef[]
  summary: string

AcceptanceVisibilityContext:
  residual_risk_summary: ResidualRiskSummary | null
  unaccepted_close_relevant_risk_refs: StateRecordRef[]
  evidence_summary: EvidenceSummary | null
  evidence_refs: StateRecordRef[]
  verification_status: not_required | required | pending | passed | failed | waived_by_user | blocked
  qa_status: not_required | required | pending | passed | failed | waived
  acceptance_status: not_required | required | pending | accepted | rejected
  what_acceptance_does_not_replace: string[]
```

`ResidualRiskSummary.status=none` means Core has no known close-relevant residual risk for the current Task/requested action. It is different from `not_visible`, which means known close-relevant risk exists but has not been shown with enough context.

In MVP-1, residual-risk summary refs usually point to `blocker` and `user_judgment` records. Rich `residual_risk` records are later/profile-promoted storage.

Autonomy Boundary summaries describe judgment latitude, not pre-write scope-check compatibility. They do not create Write Authorization records, make paths/tools/commands/network targets/secret access/sensitive categories compatible, or expand active scope and required sensitive-action permission.

## ValidatorResult

`ValidatorResult` is profile-gated. It is included here because common responses can carry validator results, but MVP-1 does not require broad validator emission unless an owner profile promotes a specific check.

```yaml
ValidatorResult:
  validator_id: string
  validator_kind: state | scope | decision | approval | evidence | verification | qa | acceptance | design | autonomy_boundary | residual_risk | artifact | projection | connector | capability
  status: passed | warning | failed | blocked | skipped
  guarantee_level: cooperative | detective | preventive | isolated
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

Stable later-profile validator IDs are listed in [Schema Later](schema-later.md#validatorresult-stable-ids).
