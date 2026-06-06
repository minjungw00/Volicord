# API Schema Core

## What this document helps you do

Use this reference for the shared API shapes that support the MVP-1 methods in [MVP API](mvp-api.md): request envelopes, common responses, read-only resource schemas, shared refs, artifact inputs, user-judgment payloads, next-action summaries, and active MVP-1 value sets.

This document describes future Harness Server behavior for planning and review. It does not mean the current documentation repository implements an MCP server.

## Contract map

| Need | Section |
|---|---|
| Active MVP-1 tools | [MVP API](mvp-api.md) |
| Error codes, MVP-1 status/error conditions, precedence, idempotency, stale-state behavior | [Errors](errors.md) |
| Later/profile-gated schemas and methods | [Schema Later](../../later/index.md#later-schema-candidates) |
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
- Later/profile-gated enum values and branches are not valid for MVP-1 and are not part of the active schema blocks below. They are defined in [Schema Later](../../later/index.md#later-schema-candidates).

Storage validation is a separate ownership boundary. API payloads and API-shaped stored JSON validate against this API reference first; storage-only JSON `TEXT`, DDL nullability, column defaults, and storage hardening validate against [Storage](../storage.md).

## Stage Profile Manifest

The schema blocks in this document define the active MVP-1 API shape. The Engineering Checkpoint may use narrower subsets of those values, but later/profile values are defined in [Schema Later](../../later/index.md#later-schema-candidates) instead of being listed here and gated only by prose.

| Stage/profile | Active API slice | Not active in that slice |
|---|---|---|
| Engineering Checkpoint | Minimal status/blocker read, one owner-valid setup path, one registered reference `capability_profile`, active Task, active Change Unit/scope boundary, `harness.prepare_write`, one compatible `harness.record_run`, one artifact/evidence ref, structured status/blocker output, and narrow close-blocker check. | Full natural-language intake, stored user judgment path, full Evidence Manifest, detached verification, Manual QA, final acceptance, residual-risk acceptance, rich projections, export/recover, broad connector APIs, hosted connector registry, cross-surface orchestration, and broad operations. |
| MVP-1 User Work Loop | Active method set owned by [MVP API](mvp-api.md#mvp-1-method-set), with next-safe-action output carried by `harness.status.next_actions`, and the same one reference `capability_profile` controlling guarantee display and capability blockers. The method set is exactly `harness.status`, `harness.intake`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.prepare_write`, `harness.record_run`, and `harness.close_task`. | Separate `harness.next`, detached verification launch/Eval, full Manual QA matrix, committed Approval hardening, export/recover, advanced connector APIs, hosted connector registry, cross-surface orchestration, broad operations, and detailed diagnostic projections. |
| Assurance Profile, Operations Profile, or later | Verification, Eval, Manual QA, waiver, full residual-risk acceptance, reconcile, validators, projection/report/export/recover, operations, and advanced connectors when owner docs promote them. | Not Engineering Checkpoint or minimum MVP-1 requirements. |

## Read-only resources

MCP resources are read-only views. They must not create Tasks, user judgments, projection jobs, reconciliations, evidence, QA, final acceptance, residual-risk acceptance, Write Authorizations, or close state.

Read-only resources use the three-part context model. `harness://status/card` is a user status card: a short readable view over current Core state and refs. Agent surfaces may use read-only resources to build an agent context packet: the minimal state, refs, freshness, and owner-section pointers needed for the next safe action. Core state remains the local authority record and only operational source of truth. Stale cards or projections are not authority, and rendered templates cannot create sensitive-action approval, final acceptance, residual-risk acceptance, evidence, or close readiness.

### Engineering Checkpoint resources

| Resource | Profile meaning |
|---|---|
| `harness://project/current` | Current registered project identity, reference `capability_profile` availability facts, and local MCP availability facts. |
| `harness://task/active` | Active Task pointer, or explicit `none` / `unknown`, without creating a Task. |
| `harness://task/{task_id}` | Current Task state for the narrow authority loop. |
| `harness://task/{task_id}/summary` | Optional compact Task status/blocker summary. |
| `harness://status/card` | Optional compact current-position user status card derived from current Core state and refs. |

### MVP-1 resources

| Resource | Profile meaning |
|---|---|
| `harness://task/{task_id}/user-judgments` | Active, resolved, deferred, and blocked `user_judgment` summaries. |
| `harness://task/{task_id}/judgment-context` | Minimum current context needed for a user judgment. |

The MVP-1 evidence and close-readiness path can also be displayed through the exact user-facing compact outputs `status-card`, `judgment-request`, `run-evidence-summary`, and `close-result`, or through `harness.status`, `harness://task/{task_id}/summary`, or `harness://status/card` when the output is derived from current Core state and refs. Agent surfaces may derive the separate agent-facing `agent-context-packet` from current Core state and refs. Exact compact-view behavior and template bodies stay with [Projection And Templates Reference](../projection-and-templates.md) and [Template Reference](../templates/README.md).

### Later resources

Assurance, operations, and diagnostic resources such as evidence-manifest reads, report reads, bundle reads, design maps, Journey views, and broad projection resources are later/profile-gated. See [Schema Later](../../later/index.md#later-schema-candidates).

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

Envelope fields are routing and audit claims. `surface_id` does not grant capability or write authority. Envelope fields do not authorize a surface to change state outside Core, and they do not prove user judgment, sensitive-action permission, final acceptance, Manual QA, or detached verification independence.

For any request that needs a primary Task, Core resolves it in this order: tool-specific `task_id`, `ToolEnvelope.task_id`, then active Task resolution. Task-scoped mutations compare `expected_state_version` against `tasks.state_version`. Project-scoped mutations with no resolved primary Task compare it against `project_state.state_version`.

## MCP boundary and caller trust

The Engineering Checkpoint/default posture is local-only exposure for one registered reference project surface. Local-only means a local process, local socket, or localhost-loopback connection for the expected local user/profile. It excludes unauthenticated shared endpoints, non-loopback binds, forwarded/tunneled endpoints, cloud/CI relays, cross-user sockets/directories, and remote callers unless a registered connector profile proves a stronger posture.

Public schemas may carry display-safe access material class, bind/reachability posture, freshness, profile refs, conformance/operator-check refs, or safe handles/fingerprints. They carry profile facts for display, validation, and diagnostics, not a hosted connector registry. They must not carry raw tokens, secrets, private configuration values, omitted secret values, or blocked payload bytes.

If Core cannot be reached, no authoritative Core response exists; report `MCP_UNAVAILABLE` or diagnostic `MCP_SERVER_UNAVAILABLE`. If Core or an operator can classify a reachable local caller/access path as outside the registered profile, use `LOCAL_ACCESS_MISMATCH` with display-safe details. If the recognized profile lacks a required capability, use `CAPABILITY_INSUFFICIENT` or an equivalent structured blocked reason and lower the guarantee display. User-facing behavior for Core unavailable, local access denied, unsupported surface, and stale state is owned by [Errors: MVP-1 guarantee and status taxonomy](errors.md#mvp-1-guarantee-and-status-taxonomy).

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
  lifecycle_phase: intake | shaping | ready | executing | waiting_user | blocked | completed | cancelled
  result: none | advice_only | passed | failed | cancelled
  close_reason: none | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  assurance_level: none | self_checked
  gates:
    scope_gate: not_required | required | pending | passed | failed | blocked
    decision_gate: not_required | required | pending | resolved | deferred | blocked
    approval_gate: not_required | required | pending | granted | denied | expired
    design_gate: not_required | required | pending | passed | partial | waived | stale | blocked
    evidence_gate: not_required | none | partial | sufficient | stale | blocked
    acceptance_gate: not_required | required | pending | accepted | rejected
```

MVP-1 `StateSummary` intentionally omits detached-verification and Manual QA lifecycle/gate values. Later/profile extensions such as `lifecycle_phase=verifying`, `lifecycle_phase=qa`, `close_reason=completed_verified`, `assurance_level=detached_verified`, `verification_gate`, and `qa_gate` are owned by [Schema Later: later close and assurance extensions](../../later/index.md#later-schema-candidates).

`EventRef.state_version` is the affected-scope resulting version after that event. It is not an event ordering key; event ordering uses `event_seq`.

`StateSummary.mode` values stay `advisor`, `direct`, and `work`. User-facing surfaces may render them as advice/read-only work, small direct work, and tracked work. Those labels are display text, not enum values.

### ProjectionKind support

`ProjectionKind` is extensible but profile-gated:

| Support class | Values | Requirement |
|---|---|---|
| Core status output | none required | Engineering Checkpoint can expose status/blocker output without persisted Markdown projection jobs. |
| MVP-1 compact outputs | No persisted `ProjectionKind` is required. Exact audience-split compact-output names and behavior are owned by [Projection And Templates Reference](../projection-and-templates.md#mvp-1-view-set) and [Template Reference](../templates/README.md#mvp-1-template-set). | The four user-facing outputs and one agent-facing packet may satisfy MVP-1 without full template rendering. `TASK` and `DIRECT-RESULT` are later/full-profile or compatibility projections. |
| Assurance reports | `APR`, `MANUAL-QA` | Only when the matching approval, Manual QA, waiver, verification, or assurance profile is active. |
| Operations/export reports | `EXPORT` | Only when export, release-handoff, or operations report profile is active. |
| Future/diagnostic projections | `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `TDD-TRACE`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT`, `DEC`, `DESIGN`, `JOURNEY-CARD` | Enable only when an owner-promoted later profile is in scope. |

Projection support never creates state, evidence, QA, verification, approval, final acceptance, residual-risk acceptance, close readiness, close authority, or Write Authorization.

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

A single intended write may carry more than one category. The category explains why sensitive-action permission may be needed; it does not resolve product, architecture, security, QA, verification, final acceptance, residual-risk acceptance, or policy judgment.

## ArtifactRef

An artifact ref points to a durable evidence file registered in the artifact store. Artifact registration is not a loose file dump: Core validates staging/capture source, stored-byte integrity, `redaction_state`, and Task-scoped owner relation before returning an `ArtifactRef`.

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

For the reference implementation, `uri` uses `harness-artifact://{project_id}/{artifact_id}`. The local file path is resolved through storage, not by trusting an absolute path in the API payload.

`ArtifactRef` carries the contract-level artifact identity, owner scope, kind, URI, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `produced_by`, relation owner, and `retention_class`. Storage may persist the relation through `artifact_links`, but API callers must be able to see which Core-owned record the artifact supports.

`redaction_state` meanings:

| State | Meaning |
|---|---|
| `none` | Stored bytes are allowed evidence under current policy. |
| `redacted` | Sensitive content was removed before storage. |
| `secret_omitted` | Secret values or PII are intentionally omitted or replaced by handles. |
| `blocked` | Raw-payload storage was blocked; only a metadata notice may be exposed. |

For `redacted`, `secret_omitted`, and `blocked`, `sha256` and `size_bytes` describe the committed safe stored bytes, not a hidden original.

Raw secrets, tokens, and full sensitive logs must not be stored as evidence artifacts. If secret-related evidence is needed, the registered ref must point to redacted bytes, an omission/blocked metadata notice, or another safe owner-approved representation.

## Stage-Specific Active Value Sets

These tables summarize the separated schemas. The active MVP-1 values in the left column are already present in the normative schema blocks in this document; they are not a prose-only filter over a broader enum. Later/profile values are owned by [Schema Later](../../later/index.md#later-schema-candidates) and must be rejected by an MVP-1 validator.

| Field | Active MVP-1 schema values | Later/profile extension values owned by Schema Later |
|---|---|---|
| `ArtifactRef.kind`, `ArtifactInput.kind` | `diff`, `log`, `screenshot`, `checkpoint`, `other` | `bundle`, `manifest`, `qa_capture`, `export_component`, `design_probe`, `prototype`, `architecture_scan`, `decision_context` |
| `ArtifactRef.retention_class`, `ArtifactInput.retention_class` | `task`, `project`, `temporary` | `export` |

| Field | Active MVP-1 schema values | Later/profile extension values owned by Schema Later |
|---|---|---|
| `ArtifactInput.relation.record_kind`, `ArtifactRef.relation_owner.record_kind` | `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` | `residual_risk`, `shared_design`, `evidence_manifest`, `eval`, `manual_qa_record`, `feedback_loop`, `tdd_trace`, `projection`, `journey_spine_entry` |
| `StateRecordRef.record_kind` | `task`, `change_unit`, `run`, `write_authorization`, `user_judgment`, `evidence_summary`, `blocker` | `approval`, `residual_risk`, `close_readiness`, `shared_design`, `domain_term`, `module_map_item`, `interface_contract`, `feedback_loop`, `evidence_manifest`, `eval`, `manual_qa_record`, `tdd_trace`, `change_unit_dependency`, `reconcile_item`, `projection` |
| `RecordRunRequest.kind`, `RecordRunPayload.kind` | `shaping_update`, `implementation`, `direct` | `verification_input` |

MVP-1 sensitive-action approval uses `record_kind=user_judgment`. Committed `approval` refs are later-profile unless the Approval owner profile is active.

The Engineering Checkpoint can further restrict these active MVP-1 lists, for example by omitting `user_judgment` where the stored judgment path is not active. It does not add values beyond the active schema.

## ArtifactInput

```yaml
ArtifactInput:
  input_id: string
  source_kind: staged_file | capture_adapter | existing_artifact
  existing_artifact_ref: ArtifactRef | null
  staged: StagedArtifactSource | null
  capture: CaptureAdapterArtifactSource | null
  kind: diff | log | screenshot | checkpoint | other
  redaction_state: none | redacted | secret_omitted | blocked
  produced_by: lead_agent | evaluator | operator | harness
  retention_class: task | project | temporary
  relation:
    task_id: string
    run_id: string | null
    record_kind: task | change_unit | run | user_judgment | evidence_summary | blocker
    record_id_hint: string | null
  description: string | null

StagedArtifactSource:
  staged_uri: string
  display_name: string | null
  content_type: string
  expected_sha256: string | null
  expected_size_bytes: integer | null

CaptureAdapterArtifactSource:
  adapter_id: string
  capture_ref: string
  display_name: string | null
  content_type: string
  expected_sha256: string | null
  expected_size_bytes: integer | null
```

`source_kind=staged_file` requires `staged` and `existing_artifact_ref=null` and `capture=null`. `source_kind=capture_adapter` requires `capture` and `staged=null` and `existing_artifact_ref=null`. `source_kind=existing_artifact` requires an existing committed `ArtifactRef` and `staged=null` and `capture=null`.

The only allowed artifact sources are a Harness staging location, an approved capture adapter output, or an existing committed artifact ref. `staged_uri` is a Harness staging locator, not permission to read arbitrary files. `capture_ref` is a capture-adapter handle, not a caller-supplied path. Tool responses return committed `ArtifactRef` values, never staged locators or capture handles as authority.

`record_run` must reject unsupported `ArtifactInput` shapes, caller-supplied arbitrary paths, forbidden raw secret payloads, tokens, and full sensitive logs before mutation. Such rejected input cannot be repaired by marking it redacted after commit. Public error mapping for these cases is owned by [Errors: Error taxonomy](errors.md#error-taxonomy).

Critical or close-relevant evidence cannot be treated as sufficient unless the supporting Core state and each required `ArtifactRef` have current owner relation, availability, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `produced_by`, and `retention_class` metadata. A missing artifact, unresolved relation owner, missing integrity metadata, or integrity failure such as diagnostic `hash_mismatch` makes the affected evidence `stale` or `blocked`; if required evidence is affected, close remains blocked.

## StateRecordRef

```yaml
StateRecordRef:
  record_kind: task | change_unit | run | write_authorization | user_judgment | evidence_summary | blocker
  record_id: string
```

`record_kind=user_judgment` is the canonical MVP-1 ref kind for user-owned judgments, including sensitive-action approval, final acceptance, and residual-risk acceptance judgments. MVP-1 evidence coverage and blockers use `record_kind=evidence_summary` and `record_kind=blocker`; durable evidence bytes use `ArtifactRef`. There is no standalone accepted-risk ref kind and no active MVP-1 `shared_design` ref kind. Discovery and requirements shaping refs point to `task`, `change_unit`, `user_judgment`, `evidence_summary`, or `blocker` owner paths as applicable.

Later/profile-only ref kinds such as `approval`, `residual_risk`, `close_readiness`, `shared_design`, `domain_term`, `module_map_item`, `interface_contract`, `feedback_loop`, `evidence_manifest`, `eval`, `manual_qa_record`, `tdd_trace`, `change_unit_dependency`, `reconcile_item`, and `projection` are defined in [Schema Later](../../later/index.md#later-schema-candidates), not accepted by this active schema. Projection-specific metadata such as `projection_path` is also later/profile material.

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
  allowed_command_classes: string[]
  allowed_network_targets: string[]
  secret_scope: string[]
  baseline_ref: string | null

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
  guarantee_level: cooperative | detective | preventive | isolated

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
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]
  note: "Autonomy Boundary is judgment latitude, not a pre-write scope check."
```

`AuthorizedAttemptScope` is the one active MVP shape for the authorized write attempt scope. Core builds it from the proposed write in `harness.prepare_write` plus resolved Core context: `task_id`, `change_unit_id`, `basis_state_version`, `surface_id`, related user judgment refs, and the displayed guarantee level. `write_authorizations.attempt_scope_json`, `WriteAuthorizationSummary.attempt_scope`, and `record_run` comparison use this same shape.

`AuthorizedAttemptScope.related_user_judgment_refs` includes compatible resolved `judgment_kind=sensitive_approval` user judgments when sensitive-action permission is required in minimum MVP-1. Committed Approval refs appear only when a later Approval owner profile is active.

`WriteAuthorizationSummary`, `WriteAuthoritySummary`, and `AuthorizedAttemptScope` are API/internal names. MVP-1 user-facing displays should call this a pre-write scope check first. Fields such as `intended_paths`, `intended_tools`, `decision=allowed`, `status=active`, `surface_id`, and `guarantee_display` describe Harness compatibility and display context for the cooperative record/check only; they do not mean OS permission, sandboxing, tamper-proof enforcement, preventive blocking, isolation, or surface-granted write authority. `allowed` belongs to `PrepareWriteResponse.decision`. `blocked` has no authorization row or lifecycle value.

`EvidenceSummary` is the active MVP-1 compact evidence contract. `status` uses exactly `not_required`, `none`, `partial`, `sufficient`, `stale`, and `blocked`; item coverage uses exactly `supported`, `unsupported`, `partial`, `not_applicable`, `stale`, and `blocked`. It is Core-owned state used by status and close checks. It is not a full Evidence Manifest, detached verification result, Manual QA record, final acceptance, residual-risk acceptance, or projection.

## UserJudgment

The MVP-1 judgment model is small but explicit. Users see one focused question; API payloads carry compact `judgment_kind` and `presentation`.

```yaml
UserJudgment:
  user_judgment_id: string
  task_id: string
  change_unit_id: string | null
  status: proposed | pending_user | resolved | deferred | rejected | blocked | superseded
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | qa_waiver | verification_risk_acceptance | final_acceptance | residual_risk_acceptance | cancellation
  presentation: short | full
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary
  question: string
  what_user_is_judging: string
  why_agent_cannot_decide: string
  no_decision_consequence: string
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
  task_ref: StateRecordRef
  change_unit_ref: StateRecordRef | null
  affected_object_refs: StateRecordRef[]
  write_refs: StateRecordRef[]
  close_refs: StateRecordRef[]
  scope_refs: StateRecordRef[]
  product_areas: string[]
  files_or_paths: string[]
  acceptance_criteria_refs: StateRecordRef[]
  note: string | null

UserJudgmentGateRef:
  gate: scope_gate | decision_gate | approval_gate | design_gate | evidence_gate | acceptance_gate
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

`judgment_kind` is the canonical decision-type field. User-facing labels such as Product decision, Technical decision, Scope decision, Sensitive action approval, QA waiver, Verification risk acceptance, Final acceptance, Residual risk acceptance, Cancellation, and their localized equivalents are renderer output derived from `judgment_kind` and locale. Active MVP-1 request, record, validator, storage, state-compatibility, and gate logic must not accept or compare `display_label` as authoritative input.

Legacy fields and methods map to the canonical names:

| Legacy | Canonical |
|---|---|
| `harness.request_user_decision` / `harness.record_user_decision` | `harness.request_user_judgment` / `harness.record_user_judgment` |
| `judgment_type` | `judgment_kind` |
| `judgment_domain` | `judgment_kind` plus a locale-derived rendered label |
| `decision_kind` | `judgment_kind` plus route-specific validation |
| `decision_profile` | `presentation` |
| `product_choice` / `technical_choice` / `sensitive_action_approval` / `work_acceptance` | `product_decision` / `technical_decision` / `sensitive_approval` / `final_acceptance` |

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
  rationale: string
  uncertainty: string | null
  deferral_consequence: string | null
  user_context: JudgmentUserContext | null
  approval_scope: ApprovalScope | null
  covers: string[]
  does_not_cover: string[]
  acceptance: AcceptanceJudgment | null
  qa_waiver: QAWaiverJudgment | null
  verification_risk_acceptance: VerificationRiskAcceptanceJudgment | null
  residual_risk_acceptance: ResidualRiskAcceptanceJudgment | null
  cancellation: CancellationJudgment | null
  separate_judgments_required: string[]

AcceptanceJudgment:
  result_ref: StateRecordRef | null
  result_summary: string
  evidence_status_refs: StateRecordRef[]
  residual_risk_visibility: ResidualRiskSummary
  does_not_replace: string[]

ResidualRiskAcceptanceJudgment:
  risk_refs: StateRecordRef[]
  accepted_scope: string[]
  acceptance_consequence: string
  follow_up_required: boolean
  follow_up: string | null
  evidence_refs: EvidenceRefs

QAWaiverJudgment:
  qa_requirement_ref: StateRecordRef | null
  waiver_allowed_by_ref: StateRecordRef | null
  skipped_qa: string
  risk_summary: string
  does_not_create_evidence: boolean

VerificationRiskAcceptanceJudgment:
  verification_requirement_ref: StateRecordRef | null
  missing_or_waived_verification: string
  risk_refs: StateRecordRef[]
  acceptance_consequence: string
  does_not_create_detached_verification: boolean

CancellationJudgment:
  cancellation_scope: string
  close_effect: string
  follow_up: string | null
```

For `judgment_kind=sensitive_approval`, `approval_scope` is required. For `judgment_kind=qa_waiver`, `qa_waiver` is required and policy must allow the waiver. For `judgment_kind=verification_risk_acceptance`, `verification_risk_acceptance` is required and must not upgrade MVP-1 assurance. For `judgment_kind=final_acceptance`, `acceptance` is required. For `judgment_kind=residual_risk_acceptance`, `residual_risk_acceptance` is required. For `judgment_kind=cancellation`, `cancellation` is required. Later reconcile branches and profile-specific final-acceptance verification/QA refs live in [Schema Later](../../later/index.md#later-schema-candidates).

<a id="userjudgmentcandidate"></a>

### UserJudgmentCandidate

`UserJudgmentCandidate` is a non-mutating candidate returned by read or validation paths when a user-owned judgment is needed before progress, write compatibility, or close can continue. It is not a committed `user_judgment` record, has no `StateRecordRef`, does not satisfy `decision_gate` or `approval_gate`, and does not create sensitive-action permission, final acceptance, residual-risk acceptance, evidence, Write Authorization, projection, or close state.

```yaml
UserJudgmentCandidate:
  candidate_id: string
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
```

The candidate body is the schema-normalized draft for a subsequent `harness.request_user_judgment` call after the caller supplies a fresh `ToolEnvelope` and Core revalidates current state. For `judgment_kind=sensitive_approval`, the candidate uses `judgment_payload.approval_scope`; there is no active MVP-1 `ApprovalRequestCandidate` or committed Approval request lifecycle.

<a id="acceptedriskinput"></a>

### AcceptedRiskInput

`AcceptedRiskInput` is accepted only by `harness.record_user_judgment` when `judgment_kind=residual_risk_acceptance`. It records the user's explicit acceptance of named close-relevant risk that was already visible in the pending `UserJudgment` context. It is not a standalone accepted-risk record kind and does not create rich Residual Risk lifecycle metadata in minimum MVP-1.

```yaml
AcceptedRiskInput:
  visible_risk_ref: StateRecordRef
  risk_summary: string
  accepted_scope: string[]
  acceptance_consequence: string
  evidence_refs: EvidenceRefs
  follow_up_required: boolean
  follow_up: string | null
```

For active MVP-1, `visible_risk_ref.record_kind` must be `blocker`. The blocker must be close-relevant, visible in the pending residual-risk acceptance `UserJudgment`, and scoped to the same Task. `accepted_risks` must be `[]` for every other `judgment_kind`. Rich residual-risk owner refs, lifecycle status, review metadata, and accepted-risk metadata are later/profile material in [Schema Later](../../later/index.md#later-schema-candidates).

<a id="record-run-payloads"></a>

## Record-run payloads

These are the active payload branches for `harness.record_run`. The top-level `RecordRunRequest.kind`, `RecordRunPayload.kind`, and the non-null payload branch must match one-to-one. Exactly one of `shaping_update`, `implementation`, or `direct` is non-null. The other branch fields must be `null`. No other branch kind is valid in active MVP-1.

```yaml
RecordRunPayload:
  kind: shaping_update | implementation | direct
  shaping_update: ShapingUpdatePayload | null
  implementation: ImplementationPayload | null
  direct: DirectPayload | null

ShapingUpdatePayload:
  shaping_kind: requirements | scope | acceptance_criteria | constraint | judgment_routing
  task_update: TaskShapingUpdate | null
  change_unit_update: ChangeUnitShapingUpdate | null
  user_judgment_candidates: UserJudgmentCandidate[]
  confirmed_facts: string[]
  remaining_uncertainties: string[]
  blocking_question: string | null
  useful_non_blocking_questions: string[]
  next_safe_action: string | null
  source_refs: StateRecordRef[]
  evidence_refs: EvidenceRefs

TaskShapingUpdate:
  title: string | null
  original_user_request: string | null
  current_goal_summary: string | null
  mode: advisor | direct | work | null
  success_criteria: string[]
  non_goals: string[]
  affected_areas: string[]
  affected_path_candidates: string[]
  constraints:
    allowed_paths: string[]
    sensitive_categories: string[]

ChangeUnitShapingUpdate:
  change_unit_id: string | null
  operation: propose | activate | update | supersede
  scope_summary: string
  affected_areas: string[]
  affected_path_candidates: string[]
  allowed_paths: string[]
  denied_paths: string[]
  non_goals: string[]
  success_criteria: string[]
  sensitive_categories: string[]
  baseline_ref: string | null
  autonomy_boundary: AutonomyBoundaryUpdate | null

AutonomyBoundaryUpdate:
  autonomy_profile: human_in_loop | afk_eligible | evaluator_only | read_only_advisor | null
  what_agent_may_do: string[]
  what_agent_may_decide_without_user: string[]
  what_requires_user_judgment: string[]
  stop_conditions: string[]

ImplementationPayload:
  outcome: completed | partial | blocked | failed
  product_write: boolean
  observed_changes: ObservedChanges
  command_results: CommandResult[]
  tool_invocations: ToolInvocationSummary[]
  network_accesses: NetworkAccessObservation[]
  secret_accesses: SecretAccessObservation[]
  evidence_updates: EvidenceUpdates
  implementation_notes: string[]
  follow_up_needed: string[]

DirectPayload:
  result_kind: answer | product_write | no_change | blocked
  product_write: boolean
  direct_summary: string
  observed_changes: ObservedChanges
  command_results: CommandResult[]
  tool_invocations: ToolInvocationSummary[]
  network_accesses: NetworkAccessObservation[]
  secret_accesses: SecretAccessObservation[]
  evidence_updates: EvidenceUpdates
  user_visible_result: string
  follow_up_needed: string[]

ObservedChanges:
  changed_paths: ChangedPath[]
  diff_artifact_input_ids: string[]
  no_product_changes: boolean

ChangedPath:
  path: string
  change_kind: added | modified | deleted | moved | copied | permission_changed | unknown
  product_file: boolean
  within_change_unit: boolean
  before_sha256: string | null
  after_sha256: string | null

CommandResult:
  command: string
  command_class: string
  exit_code: integer | null
  status: succeeded | failed | blocked | skipped | unknown
  writes_product_files: boolean
  started_at: string | null
  completed_at: string | null
  artifact_input_ids: string[]
  summary: string | null

ToolInvocationSummary:
  tool_name: string
  purpose: string
  status: succeeded | failed | blocked | skipped | unknown
  artifact_input_ids: string[]
  summary: string | null

NetworkAccessObservation:
  target: string
  direction: read | write
  observed: boolean
  note: string | null

SecretAccessObservation:
  secret_handle: string
  access_kind: read | write
  observed: boolean
  approved_by_ref: StateRecordRef | null
  note: string | null

EvidenceUpdates:
  coverage_updates: EvidenceCoverageUpdate[]
  gap_blocker_refs: StateRecordRef[]
  summary: string

EvidenceCoverageUpdate:
  claim_or_criterion: string
  coverage_state: supported | unsupported | partial | not_applicable | stale | blocked
  supporting_state_refs: StateRecordRef[]
  supporting_artifact_input_ids: string[]
  note: string | null
```

`ShapingUpdatePayload` is the active MVP payload for Discovery and requirements-shaping updates that persist into active Task, Change Unit, or User Judgment boundaries. It does not create Shared Design, Feedback Loop, TDD Trace, Evidence Manifest, Projection, Approval, Residual Risk, Discovery Brief, Question Queue, Assumption Register, First Safe Change Unit Candidate, full Decision Packet, full design artifact, or other later/profile records. At least one of `task_update`, `change_unit_update`, `user_judgment_candidates`, `confirmed_facts`, `remaining_uncertainties`, `blocking_question`, or `next_safe_action` must carry a non-empty update.

`TaskShapingUpdate.original_user_request` is the user's original wording stored on the Task; later shaping updates normally leave it unchanged. `current_goal_summary` is the clarified current goal. `confirmed_facts` are facts checked against available current sources; unavailable or stale sources belong in `remaining_uncertainties`, not in confirmed facts. `blocking_question` is the single question that changes the next safe action when one exists. If the question is user-owned, it must also appear as a `UserJudgmentCandidate` or later `user_judgment` record. `useful_non_blocking_questions` are parked context, not blockers.

`ChangeUnitShapingUpdate.operation=propose` carries a Change Unit candidate, including the first safe implementation slice when product writes are near. `operation=activate` or `update` makes or changes the active Change Unit according to Core state rules. There is no separate active First Safe Change Unit Candidate record.

`ImplementationPayload` records implementation work against a Task or Change Unit. When `product_write=true`, `RecordRunRequest.write_authorization_id` must name a compatible active Write Authorization and `observed_changes.changed_paths` must describe the observed product-file changes. The request body, including `observed_changes`, `command_results`, `tool_invocations`, `network_accesses`, `secret_accesses`, `artifact_inputs`, and `evidence_updates`, is part of request validation and the canonical idempotency hash. Storage maps the payload into the Run row's observed payload JSON fields, linked artifacts, and evidence-summary updates.

`DirectPayload` records a small direct result, including direct no-change or answer-only work. If `result_kind=product_write` or `product_write=true`, it follows the same Write Authorization, observed-change, artifact, and evidence validation rules as `ImplementationPayload`. If `product_write=false`, `write_authorization_id` must be `null` and `observed_changes.no_product_changes` must be `true`.

For product-write runs, Core compares the observed payload with the stored `AuthorizedAttemptScope`. A surface must not mark commands, command classes, network access, secret access, artifact capture, pre-tool blocking, isolation, or changed paths as verified when the relevant observation or capability is unsupported or absent; the result must instead be narrowed, blocked, or marked as `CAPABILITY_INSUFFICIENT` / insufficient surface capability according to the API and error owners.

## NextActionSummary

```yaml
NextActionSummary:
  action_kind: ask_user | prepare_write | implement | request_acceptance | close_task | idle
  summary: string
  required_tool: string | null
  related_refs: StateRecordRef[]
  blocker_code: ErrorCode | null
```

MVP-1 uses `harness.status.next_actions`, not a separate `harness.next` method. The schema block above is the complete active MVP-1 enum. Later/profile action kinds such as `launch_verify`, `record_eval`, `record_manual_qa`, and `reconcile` are defined in [Schema Later](../../later/index.md#later-schema-candidates) and must be rejected by an MVP-1 validator.

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
  acceptance_status: not_required | required | pending | accepted | rejected
  what_acceptance_does_not_replace: string[]
```

`ResidualRiskSummary.status=none` means Core has no known close-relevant residual risk for the current Task/requested action. It is different from `not_visible`, which means known close-relevant risk exists but has not been shown with enough context.

In MVP-1, residual-risk summary refs usually point to `blocker` and `user_judgment` records. Rich `residual_risk` records are later/profile-promoted storage.

Autonomy Boundary summaries describe judgment latitude, not pre-write scope-check compatibility. They do not create Write Authorization records, make paths/tools/commands/network targets/secret access/sensitive categories compatible, or expand active scope and required sensitive-action permission.

## ValidatorResult

`ValidatorResult` is included here because common responses can carry validator results. Active MVP-1 keeps this schema narrow: the only active validator kind is `capability`, and the active stable validator ID is `surface_capability_check`. It affects blocked reasons, fallback behavior, and guarantee display, not Core write authority by itself. Broader validator kinds and IDs are later/profile material.

```yaml
ValidatorResult:
  validator_id: string
  validator_kind: capability
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

For active MVP-1, `validator_id` is `surface_capability_check`. Additional validator kinds and stable IDs are listed in [Schema Later](../../later/index.md#later-schema-candidates).
