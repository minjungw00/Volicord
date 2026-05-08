# MCP API And Schemas

## Document Role

This document owns the public MCP resources, public tools, common envelope, request and response schemas, error taxonomy, idempotency behavior, state conflict behavior, validator result schema, and artifact ref schema.

It does not own SQLite DDL, the full kernel transition table, projection template text, CLI command semantics, or connector cookbook details.

## API Scope

MCP resources are read-only. All state changes go through public tools and Core. A tool response may include projection paths and artifact refs, but those are references to state records or raw evidence files, not a replacement for canonical state.

Capability is not a first-class kernel gate. Surface capability appears through:

- the `surface_capability_check` validator
- `harness.prepare_write.response.blocked_reasons`
- guarantee display in status and write decisions

## MCP Resources

Resources expose current state and projection-oriented summaries without mutating state:

```text
harness://project/current
harness://project/surfaces
harness://task/active
harness://task/{task_id}
harness://task/{task_id}/summary
harness://task/{task_id}/spine
harness://task/{task_id}/journey
harness://task/{task_id}/decision-packets
harness://task/{task_id}/change-unit-dag
harness://task/{task_id}/judgment-context
harness://task/{task_id}/reports/latest
harness://task/{task_id}/evidence-manifest
harness://task/{task_id}/bundle/current
harness://design/domain-language
harness://design/module-map
harness://design/interface-contracts
harness://policy/sensitive-categories
harness://status/card
```

Resource reads must not create Task records, decisions, projection jobs, or reconcile items. If a resource detects stale projection, it reports freshness; it does not repair it.

The Journey resources are projection-oriented reads over canonical state:

- `harness://task/{task_id}/journey` returns the current Journey Card and Journey Spine-oriented refs.
- `harness://task/{task_id}/decision-packets` returns active, resolved, deferred, and blocked Decision Packet summaries for the Task.
- `harness://task/{task_id}/change-unit-dag` returns Change Unit dependency refs and ordering summaries.
- `harness://task/{task_id}/judgment-context` returns the minimum current context needed for a user judgment, with optional pull refs separated from required context.

## Common Tool Envelope

Every public tool request carries an envelope. State-changing tools require a non-null `idempotency_key` and `expected_state_version`. Read-only tools accept the same envelope for tracing; they may set `expected_state_version` to `null`.

State version scope is resolved by Core from the operation's primary addressed Task. The resolved primary Task may come from `ToolEnvelope.task_id`, a tool-specific `task_id`, or active Task resolution. Task-scoped mutations compare `expected_state_version` with that Task's `tasks.state_version`. If Core resolves no primary Task and the operation is project-scoped, it compares `expected_state_version` with `project_state.state_version`.

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

Common response fields:

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
```

`dry_run=true` validates and returns the transition plan but does not update current records, append to `state.sqlite.task_events`, register artifacts, create consumable Write Authorization records, or enqueue projection jobs.

`ToolResponseBase.state_version` returns the resulting version for the primary affected scope. For state-changing operations this is the Task State Version when Core resolves a primary Task, otherwise the Project State Version. Read-only responses return the current `state_version` for the primary read scope and do not increment it.

## Shared Schemas

```yaml
EventRef:
  event_id: string
  event_type: string
  task_id: string | null
  state_version: integer

ProjectionJobRef:
  projection_job_id: string
  projection_kind: TASK | APR | DEC | RUN-SUMMARY | EVIDENCE-MANIFEST | EVAL | DIRECT-RESULT | MANUAL-QA | TDD-TRACE | DOMAIN-LANGUAGE | MODULE-MAP | INTERFACE-CONTRACT
  target_ref: string
  projection_version: integer
```

`EventRef.state_version` is the resulting version for the event's affected scope. Task events use `tasks.state_version`; project-level events with `task_id=null` use `project_state.state_version`.

ProjectionJobRef note: DEC is a valid projection_kind only for standalone Decision Packet Markdown when that feature is enabled. `DEC` is not an MVP-required projection job. Absence of a standalone `DEC` job must not reduce MVP Decision Packet visibility, which is required through `TASK` projections, status/next responses, judgment-context resources, and decision-packet resources.

```yaml
ToolError:
  code: ErrorCode
  message: string
  retryable: boolean
  details: object

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

Sensitive categories:

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

## Artifact Ref Schema

An artifact ref points to a durable evidence file registered in the artifact store. Report projections and record projections use artifact refs when they need evidence-file references; the projection itself is not the evidence file.

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

For the reference MVP, `uri` uses `harness-artifact://{project_id}/{artifact_id}`. The local file path is resolved through the per-project `artifacts` registry row in `state.sqlite`, not by trusting an absolute path in the API payload.

Requests that create or attach evidence use `ArtifactInput`. A request may either reference an existing committed artifact or provide a staged file for Core to validate, register, and return as an `ArtifactRef`.

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
    record_kind: task | change_unit | run | decision_packet | shared_design | residual_risk | evidence_manifest | eval | manual_qa_record | tdd_trace | journey_spine_entry | verification_bundle | export | other
    record_id_hint: string | null
  description: string | null

StagedArtifactSource:
    staged_uri: string
    display_name: string | null
    content_type: string
    expected_sha256: string | null
    expected_size_bytes: integer | null
```

Rules:

- `source_kind=existing_artifact` requires `existing_artifact_ref` and must set `staged` to `null`.
- `source_kind=staged_file` requires `staged` and must set `existing_artifact_ref` to `null`.
- When an existing artifact is attached to a new record, Core verifies the artifact's task relation and rejects incompatible reuse.
- `staged_uri` must point to a harness staging location or an approved capture adapter, not an arbitrary absolute path supplied for trust.
- If `expected_sha256` or `expected_size_bytes` is present, Core verifies the stored bytes before commit.
- Core applies redaction rules before final storage and records the committed artifact as an `ArtifactRef`.
- Tool responses return committed `ArtifactRef` values in `registered_artifacts`, `bundle_ref`, or other response fields.

Record or projection references use `StateRecordRef`, not `ArtifactRef`:

```yaml
StateRecordRef:
  record_kind: task | change_unit | change_unit_dependency | run | approval | write_authorization | decision_packet | journey_spine_entry | shared_design | residual_risk | evidence_manifest | eval | manual_qa_record | tdd_trace | reconcile_item | projection
  record_id: string
  projection_path: string | null
```

Evidence references, approval scope, write authorization, Write Authority Summary display, and end-to-end paths use these shared shapes:

```yaml
EvidenceRefs:
  state_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]

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
  approval_refs: StateRecordRef[]
  decision_packet_refs: StateRecordRef[]
  guarantee_level: cooperative | detective | preventive | isolated
  status: allowed | consumed | expired | stale | revoked
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
  note: "Autonomy Boundary is judgment latitude, not write authority."

EndToEndPath:
  trigger_or_input: string | null
  domain_logic: string | null
  persistence_or_state: string | null
  api_or_caller_boundary: string | null
  ui_or_observable_output: string | null
```

`WriteAuthorizationSummary` and `WriteAuthoritySummary` are API payload shapes only. This document does not define SQLite DDL for Write Authorization records. `WriteAuthoritySummary` is the display/read shape clients use to show the Write Authority Summary beside Autonomy Boundary judgment latitude.

`DEC` remains a valid projection job kind for standalone Decision Packet Markdown when that projection feature is enabled. MVP-required Decision Packet visibility is provided through `TASK` projections, status/next responses, judgment-context resources, and decision-packet resources. Full DEC and Decision Packet template text is owned by Appendix A, not this API schema file.

Decision Packet, Write Authorization, Write Authority Summary, Journey Card, Judgment Context, Autonomy Boundary, acceptance visibility, and residual-risk summaries are public MCP schemas. They describe API payloads only; owner docs define the canonical kernel records.

```yaml
DecisionPacket:
  decision_packet_id: string
  task_id: string
  change_unit_id: string | null
  status: proposed | pending_user | resolved | deferred | rejected | blocked | superseded
  decision_kind: approval | scope_confirmation | design_choice | architecture_choice | product_tradeoff | autonomy_boundary | verification_waiver | qa_waiver | acceptance | residual_risk_acceptance | reconcile
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary
  what_user_is_deciding: string
  what_agent_may_decide_without_user: string[]
  affected_gates:
    - scope_gate | decision_gate | approval_gate | design_gate | evidence_gate | verification_gate | qa_gate | acceptance_gate
  affected_acceptance_criteria:
    - criteria_id: string
      statement: string
  options: DecisionPacketOption[]
  recommendation: DecisionPacketRecommendation | null
  deferral_consequence: string
  user_context: DecisionPacketUserContext
  approval_scope: ApprovalScope | null
  reconcile_item_id: string | null
  created_at: string
  resolved_at: string | null

DecisionPacketOption:
  option_id: string
  label: string
  benefits: string[]
  costs: string[]
  risks: string[]
  reversibility: reversible | partially_reversible | irreversible | unknown
  confidence: low | medium | high
  suitable_when: string[]
  evidence_refs: EvidenceRefs

DecisionPacketRecommendation:
  option_id: string | null
  reason: string
  uncertainty: string | null
  when_to_revisit: string | null

DecisionPacketUserContext:
  minimum_context: string[]
  optional_pull_refs: StateRecordRef[]

DecisionPacketCandidate:
  task_id: string
  change_unit_id: string | null
  decision_kind: approval | scope_confirmation | design_choice | architecture_choice | product_tradeoff | autonomy_boundary | verification_waiver | qa_waiver | acceptance | residual_risk_acceptance | reconcile
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary
  what_user_is_deciding: string
  what_agent_may_decide_without_user: string[]
  affected_gates:
    - scope_gate | decision_gate | approval_gate | design_gate | evidence_gate | verification_gate | qa_gate | acceptance_gate
  affected_acceptance_criteria:
    - criteria_id: string
      statement: string
  options: DecisionPacketOption[]
  recommendation: DecisionPacketRecommendation | null
  deferral_consequence: string
  user_context: DecisionPacketUserContext
  expires_at: string | null
  approval_scope: ApprovalScope | null
  reconcile_item_id: string | null

JourneyCardSummary:
  task_id: string
  state: StateSummary
  current_position: string
  next_action: string
  active_change_unit_ref: StateRecordRef | null
  write_authority_summary: WriteAuthoritySummary | null
  active_decision_packet_refs: StateRecordRef[]
  blocker_refs: StateRecordRef[]
  residual_risk_summary: ResidualRiskSummary | null
  projection_freshness:
    status: current | stale | failed | unknown
    stale_refs: StateRecordRef[]

JudgmentContext:
  task_ref: StateRecordRef
  journey_card: JourneyCardSummary | null
  current_state_summary: StateSummary
  minimum_context: string[]
  relevant_refs: StateRecordRef[]
  evidence_refs: EvidenceRefs
  active_decision_packet_refs: StateRecordRef[]
  optional_pull_refs: StateRecordRef[]
  stale_or_missing_refs: StateRecordRef[]
  acceptance_visibility: AcceptanceVisibilityContext | null

AutonomyBoundarySummary:
  change_unit_id: string | null
  status: absent | proposed | active | exceeded | stale
  autonomy_profile: human_in_loop | afk_eligible | evaluator_only | read_only_advisor | null
  what_agent_may_do: string[]
  what_agent_may_decide_without_user: string[]
  what_requires_user_judgment: string[]
  stop_conditions: string[]
  triggered_stop_conditions: string[]
  related_decision_packet_refs: StateRecordRef[]

ResidualRiskSummary:
  status: none | visible | not_visible | accepted | blocked
  close_relevant_count: integer
  not_visible_refs: StateRecordRef[]
  unaccepted_refs: StateRecordRef[]
  accepted_refs: StateRecordRef[]
  summary: string

AcceptanceVisibilityContext:
  residual_risk_summary: ResidualRiskSummary | null
  unaccepted_close_relevant_risk_refs: StateRecordRef[]
  evidence_summary_refs: StateRecordRef[]
  verification_status: not_required | required | pending | passed | failed | waived_by_user | blocked
  qa_status: not_required | required | pending | passed | failed | waived
  acceptance_status: not_required | required | pending | accepted | rejected
  what_acceptance_does_not_replace: string[]
```

Autonomy Boundary summaries describe judgment latitude, not scope authority. They do not authorize paths, tools, commands, network targets, secret access, or sensitive categories outside the active Change Unit scope and any required approval.

`decision_kind=approval` is retained as a stable public enum value. In both `DecisionPacket` and `DecisionPacketCandidate`, it means an approval-shaped judgment context for sensitive-change approval only. It cannot resolve product trade-offs, design direction, QA waiver, verification risk, final acceptance, or residual-risk acceptance unless those decisions are separately represented by compatible Decision Packets and gate updates.

## Validator Result Schema

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

The `surface_capability_check` validator uses this schema with `validator_kind=capability`.

Stable design and agency validator ids used by this API are:

- `decision_quality_check`
- `autonomy_boundary_check`
- `feedback_loop_check`
- `tdd_trace_required`
- `codebase_stewardship_check`
- `residual_risk_visibility_check`
- `context_hygiene_check`

## Error Taxonomy

| Code | Meaning |
|---|---|
| `STATE_CONFLICT` | `expected_state_version` is stale for the relevant state version scope, lock ownership changed, or the same idempotency key was reused with a different payload |
| `NO_ACTIVE_TASK` | a Task is required but none is active or addressed |
| `NO_ACTIVE_CHANGE_UNIT` | a write-capable operation has no active scoped Change Unit |
| `SCOPE_REQUIRED` | scope confirmation is required before the requested write can proceed |
| `SCOPE_VIOLATION` | intended paths, tools, commands, network, secrets, or categories exceed scope |
| `WRITE_AUTHORIZATION_REQUIRED` | a write-capable run is missing a required Write Authorization from `prepare_write` |
| `WRITE_AUTHORIZATION_INVALID` | the supplied Write Authorization is absent, expired, stale, revoked, already consumed outside idempotent replay, or incompatible with the Task, Change Unit, baseline, intended operation, approval refs, or Decision Packet refs |
| `DECISION_REQUIRED` | blocking product judgment requires a Decision Packet before the requested action can proceed |
| `DECISION_UNRESOLVED` | a relevant Decision Packet is pending, deferred without coverage, rejected, blocked, stale, or incompatible with the requested action |
| `AUTONOMY_BOUNDARY_EXCEEDED` | the intended operation exceeds the active Change Unit Autonomy Boundary |
| `APPROVAL_REQUIRED` | sensitive change requires approval before proceeding |
| `APPROVAL_DENIED` | the relevant approval was denied |
| `APPROVAL_EXPIRED` | approval expired or drifted from baseline/scope |
| `CAPABILITY_INSUFFICIENT` | the connected surface cannot satisfy a required validator or enforcement condition |
| `MCP_UNAVAILABLE` | required MCP access is unavailable or stale |
| `EVIDENCE_INSUFFICIENT` | required evidence coverage is absent, partial, stale, or blocked |
| `VERIFY_NOT_DETACHED` | verification cannot count as detached verification |
| `QA_REQUIRED` | required Manual QA is pending, failed, or missing |
| `ACCEPTANCE_REQUIRED` | required user acceptance is pending or rejected |
| `PROJECTION_STALE` | projection freshness is stale or failed for the requested action |
| `RECONCILE_REQUIRED` | human-editable or managed-block drift requires reconcile |
| `RESIDUAL_RISK_NOT_VISIBLE` | known close-relevant residual risk has not been made visible before a successful close |
| `ARTIFACT_MISSING` | a referenced artifact file is missing or integrity check failed |
| `BASELINE_STALE` | baseline no longer matches the repository state required by the operation |
| `VALIDATOR_FAILED` | one or more required validators failed |

`WRITE_AUTHORIZATION_REQUIRED` and `WRITE_AUTHORIZATION_INVALID` are used only for missing or invalid Write Authorization. Scope violations still use `SCOPE_VIOLATION` when observed paths, tools, commands, network targets, secrets, or sensitive categories exceed authorized or active scope.

`DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID`, `AUTONOMY_BOUNDARY_EXCEEDED`, and `RESIDUAL_RISK_NOT_VISIBLE` are stable public `ErrorCode` values. Validator-specific detail still belongs in `ValidatorResult.findings`.

## Idempotency And State Conflict Behavior

Idempotency keys are scoped to `(project_id, tool_name, idempotency_key)`. Repeating the same payload with the same key returns the original committed response. Reusing a key with a different payload returns `STATE_CONFLICT`.

For state-changing tools, Core compares `expected_state_version` with current project/task state. A mismatch returns `STATE_CONFLICT` and includes the current state version and a status summary in `details`. The caller must refresh state and either retry with a new idempotency key or replay the exact previous request.

State conflict comparison is scope-specific. Core first resolves the primary addressed Task from `ToolEnvelope.task_id`, any tool-specific `task_id`, or active Task resolution. Task-scoped tools compare against that Task's `tasks.state_version`; project-scoped tools with no resolved primary Task compare against `project_state.state_version`. `STATE_CONFLICT.details` should include `scope` (`task` or `project`), `current_state_version`, `expected_state_version`, and the relevant `project_id` plus `task_id` when `scope=task`; it may also include a compact status summary for refresh guidance.

## Public Tools

### `harness.status`

Purpose: return project, surface, active Task, Journey Card, gate, guarantee, projection, active Decision Packet, Autonomy Boundary, Write Authority Summary, residual-risk, and pending-decision status.

Allowed actor: `user`, `lead_agent`, `evaluator`, `operator`.

Request schema:

```yaml
StatusRequest:
  envelope: ToolEnvelope
  include:
    task: boolean
    gates: boolean
    projections: boolean
    pending_decisions: boolean
    guarantees: boolean
    journey_card: boolean
    decision_packets: boolean
    autonomy_boundary: boolean
    write_authority: boolean
    residual_risk: boolean
```

Response schema:

```yaml
StatusResponse:
  base: ToolResponseBase
  active_task: StateSummary | null
  status_card: string
  journey_card: JourneyCardSummary | null
  pending_decisions: StateRecordRef[]
  active_decision_packet_refs: StateRecordRef[]
  autonomy_boundary_summary: AutonomyBoundarySummary | null
  write_authority_summary: WriteAuthoritySummary | null
  residual_risk_summary: ResidualRiskSummary | null
  projection_freshness:
    status: current | stale | failed | unknown
    stale_refs: StateRecordRef[]
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]
```

State transition summary: no state transition.

Events emitted: none.

Projection jobs enqueued: none.

`pending_decisions` contains unresolved user-action Decision Packets. `active_decision_packet_refs` contains all Decision Packets relevant to the current phase or requested action, including pending, deferred, blocked, or recently resolved packets. Both fields use `StateRecordRef` entries with `record_kind=decision_packet`.

`write_authority_summary` is returned when `include.write_authority=true`. When `include.journey_card=true`, the same current Write Authority Summary display may also appear in `journey_card.write_authority_summary`.

Validators run: optional `surface_capability_check`, optional `decision_gate_check`, optional `autonomy_boundary_check`, optional residual-risk visibility read, optional projection freshness read.

Possible errors: `MCP_UNAVAILABLE`, `PROJECTION_STALE`.

Idempotency behavior: read-only; repeated requests do not mutate state.

### `harness.intake`

Purpose: create or resume a Task from user intent and classify it as advisor, direct, or work.

Allowed actor: `user`, `lead_agent`, `operator`.

Request schema:

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

Response schema:

```yaml
IntakeResponse:
  base: ToolResponseBase
  task_id: string
  created: boolean
  resumed: boolean
  state: StateSummary
  next_action: string
  change_unit_id: string | null
```

State transition summary: creates or resumes a Task; sets `mode` and initial `lifecycle_phase`; may create an initial Change Unit for write-capable direct/work.

Events emitted: `task_intake_recorded`, `task_created`, `task_resumed`, `task_superseded`, `change_unit_created`.

Projection jobs enqueued: `TASK`; optionally `DOMAIN-LANGUAGE`, `MODULE-MAP`, or `INTERFACE-CONTRACT` if intake accepted design support records.

Validators run: `state_envelope`, `active_task_policy`, `surface_capability_check`.

Possible errors: `STATE_CONFLICT`, `MCP_UNAVAILABLE`, `VALIDATOR_FAILED`, `CAPABILITY_INSUFFICIENT`.

Idempotency behavior: same key returns the same Task/resume decision; different payload with same key returns `STATE_CONFLICT`.

### `harness.next`

Purpose: return the next safe action, instruction bundle, and pending decisions for the current Task.

Allowed actor: `user`, `lead_agent`, `evaluator`, `operator`.

Request schema:

```yaml
NextRequest:
  envelope: ToolEnvelope
  task_id: string | null
  focus: status | shaping | decision | implementation | verification | qa | acceptance | reconcile
  include_instruction_bundle: boolean
```

Response schema:

```yaml
NextResponse:
  base: ToolResponseBase
  state: StateSummary | null
  next_action:
    action_kind: ask_user | prepare_write | implement | launch_verify | record_eval | record_manual_qa | request_acceptance | close_task | reconcile | idle
    summary: string
    required_tool: string | null
  instruction_bundle:
    summary: string
    constraints: string[]
    relevant_refs: StateRecordRef[]
    artifact_refs: ArtifactRef[]
  pending_decisions: StateRecordRef[]
  judgment_context: JudgmentContext | null
  autonomy_boundary: AutonomyBoundarySummary | null
```

State transition summary: no state transition.

Events emitted: none.

Projection jobs enqueued: none.

`pending_decisions` contains unresolved user-action Decision Packets. Deferred, blocked, or recently resolved packets that still affect the current phase or requested action appear through `judgment_context.active_decision_packet_refs`.

When `focus=acceptance`, `judgment_context.acceptance_visibility` must be non-null. It must include the residual-risk summary, unaccepted close-relevant risk refs, evidence summary refs, verification status, QA status, acceptance status, and what acceptance does not replace. The context must make clear before any acceptance request that acceptance does not replace evidence sufficiency, verification, Manual QA, approval, scope, or residual-risk visibility.

Validators run: optional `surface_capability_check`, optional `decision_gate_check`, optional `autonomy_boundary_check`, optional `context_hygiene_check`.

Possible errors: `NO_ACTIVE_TASK`, `MCP_UNAVAILABLE`, `PROJECTION_STALE`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `RECONCILE_REQUIRED`.

Idempotency behavior: read-only; repeated requests do not mutate state.

### `harness.prepare_write`

Purpose: decide whether an intended product write is allowed before the agent writes.

Allowed actor: `lead_agent`, `operator`.

Request schema:

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
```

Response schema:

```yaml
PrepareWriteResponse:
  base: ToolResponseBase
  decision: allowed | blocked | approval_required | decision_required | state_conflict
  state: StateSummary | null
  change_unit_id: string | null
  baseline_ref: string | null
  write_authorization_ref: StateRecordRef | null
  write_authorization: WriteAuthorizationSummary | null
  authorization_effect: none | would_create | created | returned
  active_decision_packet_refs: StateRecordRef[]
  blocked_reasons:
    - code: string
      message: string
      related_error: ErrorCode
  approval_request_candidate: ApprovalRequestCandidate | null
  decision_packet_candidate: DecisionPacketCandidate | null
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]

ApprovalRequestCandidate:
  sensitive_categories: string[]
  allowed_paths: string[]
  allowed_tools: string[]
  allowed_commands: string[]
  allowed_network_targets: string[]
  secret_scope: string[]
  baseline_ref: string | null
```

`approval_request_candidate` is present only when `decision=approval_required` or when Core can suggest a new approval request. Otherwise it is `null`.

When `dry_run=false` and `decision=allowed`, the response must include a non-null `write_authorization_ref`; the `write_authorization` summary may also be returned when the caller requests expanded payloads or the implementation supports it. `authorization_effect` is `created` when Core creates a new authorization.

`authorization_effect=returned` is reserved for idempotent replay of the same committed `prepare_write` request and response with the same idempotency key, request hash, and state basis. A distinct compatible request creates a distinct Write Authorization; compatibility does not make authorizations reusable. Core may stale, expire, or revoke older unconsumed authorizations if their compatibility basis changes.

When `dry_run=true` and the write would otherwise be allowed, Core returns `decision=allowed` with `authorization_effect=would_create`, but `write_authorization_ref` and `write_authorization` must be `null`, and no Write Authorization record, event, artifact, or projection job is created.

For `decision=blocked`, `decision=approval_required`, `decision=decision_required`, and `decision=state_conflict`, both authorization fields must be `null` and `authorization_effect=none`.

A Write Authorization is specific to the intended operation and the current state, baseline, active Change Unit scope, approval refs, Decision Packet refs, sensitive categories, and guarantee level. It is consumed by `harness.record_run` through `write_authorization_id`; it is not a reusable grant.

`active_decision_packet_refs` contains all Decision Packets relevant to the intended write, including pending, deferred, blocked, or recently resolved packets.

`decision_packet_candidate` is present when `decision=decision_required` and no compatible Decision Packet already exists. Its fields match `RequestUserDecisionRequest` after the envelope. It is a non-mutating candidate payload for a later `harness.request_user_decision` call; returning it from `prepare_write` does not create or update a Decision Packet.

State transition summary: may move Task to `executing`, `waiting_user`, or `blocked`; may create a Write Authorization when allowed or return the already committed response for idempotent replay; may set `scope_gate=pending/blocked`, `decision_gate=required/pending/blocked`, `approval_gate=pending/expired`, or stale evidence/approval markers.

Events emitted: `prepare_write_allowed`, `write_authorization_created`, `write_authorization_returned`, `prepare_write_blocked`, `scope_required`, `decision_required`, `autonomy_boundary_exceeded`, `approval_required`, `baseline_stale_detected`, `capability_insufficient_detected`.

Projection jobs enqueued: `TASK`; `APR` when approval is required.

Validators run: `state_envelope`, `active_task`, `active_change_unit`, `scope_coverage`, `changed_paths_intent`, `autonomy_boundary_check`, `baseline_freshness`, `approval_scope`, `decision_gate_check`, `decision_quality_check`, `feedback_loop_check`, `tdd_trace_required`, `codebase_stewardship_check`, `surface_capability_check`, design precondition validators that apply before write.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `BASELINE_STALE`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `VALIDATOR_FAILED`.

Idempotency behavior: repeated allowed/blocked decision with same payload returns the original decision and event refs; changed payload with same key returns `STATE_CONFLICT`.

### `harness.record_run`

Purpose: record shaping, implementation, direct-result, or verification-input run data, including artifacts and evidence updates.

Allowed actor: `lead_agent`, `evaluator`, `operator`.

Request schema:

```yaml
RecordRunRequest:
  envelope: ToolEnvelope
  kind: shaping_update | implementation | direct | verification_input
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
  verification_input: VerificationInputPayload | null

ShapingUpdatePayload:
  task_summary_update: string | null
  acceptance_criteria_updates:
    - criteria_id: string | null
      operation: add | update | remove
      statement: string
  change_unit_updates:
    - operation: create | update | select_active | complete | defer | supersede
      change_unit_id: string | null
      title: string | null
      purpose: string | null
      non_goals: string[]
      slice_type: vertical | enabling | cleanup | horizontal-exception | null
      horizontal_exception_reason: string | null
      follow_up_vertical_change_unit_id: string | null
      allowed_paths: string[]
      allowed_tools: string[]
      allowed_commands: string[]
      allowed_network_targets: string[]
      secret_scope: string[]
      sensitive_categories: string[]
      autonomy_profile: human_in_loop | afk_eligible | evaluator_only | read_only_advisor | null
      agent_may_do: string[]
      user_judgment_required: string[]
      afk_stop_conditions: string[]
      end_to_end_path: EndToEndPath | null
      validator_profile: string[]
      completion_conditions: string[]
      evaluator_focus: string[]
  design_record_refs: StateRecordRef[]
  pending_decision_refs: StateRecordRef[]

ImplementationPayload:
  observed_changes: ObservedChanges
  command_results: CommandResult[]
  evidence_updates: EvidenceUpdates
  tdd_trace_update: TddTraceUpdate | null

DirectPayload:
  observed_changes: ObservedChanges
  command_results: CommandResult[]
  evidence_updates: EvidenceUpdates
  self_check_summary: string
  escalation:
    value: none | escalate_to_work
    reason: string | null

VerificationInputPayload:
  evaluator_bundle_input: ArtifactInput | null
  evaluator_focus: string[]
  observed_changes: ObservedChanges
  command_results: CommandResult[]

ObservedChanges:
  changed_paths: string[]
  created_paths: string[]
  deleted_paths: string[]

CommandResult:
  command: string
  exit_code: integer
  artifact_inputs: ArtifactInput[]
  summary: string

EvidenceUpdates:
  acceptance_criteria:
    - criteria_id: string
      status: supported | unsupported | not_applicable
      supporting_refs: StateRecordRef[]
      artifact_inputs: ArtifactInput[]

TddTraceUpdate:
  tdd_trace_id: string | null
  status: required | recorded | waived | not_required
  red_inputs: ArtifactInput[]
  green_inputs: ArtifactInput[]
  refactor_inputs: ArtifactInput[]
  non_tdd_justification: string | null
```

The `payload` branch must match `kind`; all other branches must be `null` or absent. `ArtifactInput` values are resolved during the same Core transaction; response fields contain the committed `ArtifactRef` values. Change Unit creation and update for MVP happens through `kind=shaping_update` with `change_unit_updates`; `operation=create` creates a `change_units` record, and `operation=select_active` updates the Task's `active_change_unit_id`. `allowed_paths`, `allowed_tools`, `allowed_commands`, `allowed_network_targets`, `secret_scope`, and `sensitive_categories` are scope fields. `autonomy_profile`, `agent_may_do`, `user_judgment_required`, and `afk_stop_conditions` describe Autonomy Boundary judgment latitude only.

`write_authorization_id` references the compatible Write Authorization returned by `harness.prepare_write`. For `kind=implementation` and `kind=direct`, `write_authorization_id` is required unless the Run records no product write and Core classifies it as read-only evidence or shaping. For `kind=shaping_update`, `write_authorization_id` must be `null`; MVP does not support shaping updates that also record observed product writes, so those writes must be recorded as `kind=implementation` or `kind=direct` with a compatible authorization. For `kind=verification_input`, keep `write_authorization_id` `null`; verification input that creates product writes should normally be disallowed in MVP.

`runs.write_authorization_id` is populated only when a Run successfully consumes a compatible Write Authorization. A violation or audit Run that attempted to use an invalid, stale, missing, consumed, or scope-exceeded authorization must not populate `runs.write_authorization_id` as a consumed authorization. The attempted authorization ref, when useful for audit, should be recorded in validator findings, run violation payload, or `task_events.payload_json`. Such a violation Run may be recorded for audit or recovery if an observed product write already happened, but it must not satisfy evidence sufficiency, detached verification, QA, acceptance, or close readiness. The corresponding Write Authorization should remain unconsumed and may be marked stale, revoked, or expired according to the violation and compatibility basis.

Response schema:

```yaml
RecordRunResponse:
  base: ToolResponseBase
  run_id: string
  state: StateSummary
  write_authorization_ref: StateRecordRef | null
  evidence_manifest_ref: StateRecordRef | null
  run_summary_ref: StateRecordRef | null
  direct_result_ref: StateRecordRef | null
  registered_artifacts: ArtifactRef[]
  next_action: string
```

`write_authorization_ref` is non-null only when the committed Run successfully consumes a compatible Write Authorization.

State transition summary: shaping updates can keep `shaping`, move to `ready`, or move to `waiting_user`; implementation moves toward `verifying`; direct can become close-eligible or escalate to work; verification input records evaluator bundle context without proving detached verification.

Events emitted: `run_recorded`, `write_authorization_consumed`, `shaping_updated`, `implementation_recorded`, `direct_result_recorded`, `verification_input_recorded`, `evidence_manifest_updated`, `artifact_registered`, `tdd_trace_updated`.

Projection jobs enqueued: `TASK`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`; `DIRECT-RESULT` for `kind=direct`; `TDD-TRACE` when updated.

Validators run: `state_envelope`, `changed_paths`, `scope_coverage`, `approval_scope`, `baseline_freshness`, `artifact_integrity`, `evidence_sufficiency`, `decision_quality_check`, `autonomy_boundary_check`, `feedback_loop_check`, `tdd_trace_required`, `codebase_stewardship_check`, applicable design-quality validators, `surface_capability_check`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID`, `SCOPE_VIOLATION`, `APPROVAL_REQUIRED`, `APPROVAL_EXPIRED`, `ARTIFACT_MISSING`, `BASELINE_STALE`, `EVIDENCE_INSUFFICIENT`, `VALIDATOR_FAILED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated request returns the same run, artifact records, evidence updates, events, and projection jobs; artifact inputs and resolved artifact refs must match the original payload.

### `harness.request_user_decision`

Purpose: create a structured Decision Packet for a user judgment that blocks progress, write, close, risk acceptance, waiver, or reconcile.

Allowed actor: `lead_agent`, `evaluator`, `operator`.

Request schema:

```yaml
RequestUserDecisionRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  decision_kind: approval | scope_confirmation | design_choice | architecture_choice | product_tradeoff | autonomy_boundary | verification_waiver | qa_waiver | acceptance | residual_risk_acceptance | reconcile
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary | null
  what_user_is_deciding: string
  what_agent_may_decide_without_user: string[]
  affected_gates:
    - scope_gate | decision_gate | approval_gate | design_gate | evidence_gate | verification_gate | qa_gate | acceptance_gate
  affected_acceptance_criteria:
    - criteria_id: string
      statement: string
  options: DecisionPacketOption[]
  recommendation: DecisionPacketRecommendation | null
  deferral_consequence: string
  user_context: DecisionPacketUserContext
  expires_at: string | null
  approval_scope: ApprovalScope | null
  reconcile_item_id: string | null
```

Core stores a canonical `DecisionPacket`. If `state_summary_at_request` is `null`, Core derives it from current state during the same transaction. The stored `state_summary_at_request` is a request-time snapshot and is not updated by later Task transitions. `approval_scope` is required when `decision_kind=approval`; for all other `decision_kind` values it must be `null` or omitted. `decision_kind=approval` is only the approval-shaped sensitive-change context and cannot resolve product trade-offs, design direction, QA waiver, verification risk, final acceptance, or residual-risk acceptance without separate compatible Decision Packets and gate updates. A `residual_risk_acceptance` packet must include the risk visibility context in `user_context.minimum_context` and relevant risk refs in `context.source_refs`.

Response schema:

```yaml
RequestUserDecisionResponse:
  base: ToolResponseBase
  decision_packet_id: string
  decision_packet_ref: StateRecordRef
  decision_packet: DecisionPacket
  approval_id: string | null
  reconcile_item_id: string | null
  state: StateSummary
  user_visible_summary: string
```

`pending_decisions` returned by status and next-action responses contain unresolved user-action `StateRecordRef` entries with `record_kind=decision_packet`. `active_decision_packet_refs` fields include all Decision Packets relevant to the current phase or requested action, including pending, deferred, blocked, or recently resolved packets.

State transition summary: records a pending Decision Packet and usually moves Task to `waiting_user`; product judgment sets `decision_gate=pending`; approval requests set `approval_gate=pending`; scope confirmation sets `scope_gate=pending`; acceptance and residual-risk acceptance set or keep `acceptance_gate=pending` when acceptance is required.

Events emitted: `decision_packet_created`, `user_decision_requested`, `approval_requested`, `scope_confirmation_requested`, `design_choice_requested`, `architecture_choice_requested`, `autonomy_boundary_decision_requested`, `verification_waiver_requested`, `qa_waiver_requested`, `acceptance_requested`, `residual_risk_acceptance_requested`, `reconcile_decision_requested`.

Projection jobs enqueued: `TASK`; `DEC` when standalone Decision Packet projection is enabled; `APR` for approval where applicable; affected projection for reconcile.

Validators run: `state_envelope`, `decision_packet_validity`, `decision_quality_check`, `autonomy_boundary_check` when the packet affects the active Change Unit boundary, `approval_scope` for approval decisions, `reconcile_required` for reconcile decisions, `residual_risk_visibility_check` for risk-acceptance decisions.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `DECISION_REQUIRED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `RECONCILE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `PROJECTION_STALE`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated request returns the same Decision Packet, related records, events, and projection jobs; a different packet payload with the same key returns `STATE_CONFLICT`.

### `harness.record_user_decision`

Purpose: record the user's answer to a pending Decision Packet and optionally record accepted residual risk.

Allowed actor: `user`, `operator`.

Request schema:

```yaml
RecordUserDecisionRequest:
  envelope: ToolEnvelope
  decision_packet_id: string
  decision_kind: approval | scope_confirmation | design_choice | architecture_choice | product_tradeoff | autonomy_boundary | verification_waiver | qa_waiver | acceptance | residual_risk_acceptance | reconcile
  selected_option_id: string
  decision: RecordUserDecisionPayload
  note: string
  waiver_reason: string | null
  accepted_risks: AcceptedRiskInput[]

RecordUserDecisionPayload:
  approval:
    value: granted | denied | expired
  scope_confirmation:
    value: confirmed | rejected | revise_scope
  design_choice:
    value: selected | rejected | defer
  architecture_choice:
    value: selected | rejected | defer
  product_tradeoff:
    value: selected | rejected | defer
  autonomy_boundary:
    value: accepted | rejected | revise_boundary | defer
  verification_waiver:
    value: waived | rejected
  qa_waiver:
    value: waived | rejected
  acceptance:
    value: accepted | rejected
  residual_risk_acceptance:
    value: accepted | rejected | defer
  reconcile:
    value: merge | reject | convert_to_note | create_decision | defer

AcceptedRiskInput:
  residual_risk_ref: StateRecordRef | null
  risk_summary: string
  accepted_scope: string[]
  acceptance_consequence: string
  follow_up_required: boolean
  follow_up: string | null
  evidence_refs: EvidenceRefs
```

The payload branch must match `decision_kind`; other branches must be absent. `accepted_risks` is allowed only when the Decision Packet and current Judgment Context made the close-relevant residual risk visible before the user decision. Core records accepted risk as residual-risk state refs; it does not treat risk acceptance as detached verification.

Response schema:

```yaml
RecordUserDecisionResponse:
  base: ToolResponseBase
  decision_packet_id: string
  decision_packet_ref: StateRecordRef
  state: StateSummary
  updated_records: StateRecordRef[]
  accepted_risk_refs: StateRecordRef[]
  next_action: string
```

State transition summary: resolves, defers, rejects, or blocks the targeted Decision Packet; updates affected gates or reconcile item; approval grant/deny updates `approval_gate`; accepted scope updates `scope_gate`; user-resolved product judgment updates `decision_gate`; accepted Autonomy Boundary decisions may update the active Change Unit boundary; verification waiver updates `verification_gate=waived_by_user`; QA waiver updates `qa_gate`; acceptance updates `acceptance_gate`; accepted residual risk records accepted-risk refs without upgrading assurance; reconcile may create accepted state records.

Events emitted: `user_decision_recorded`, `decision_packet_resolved`, `decision_packet_deferred`, `decision_packet_rejected`, `approval_granted`, `approval_denied`, `scope_confirmed`, `scope_rejected`, `design_choice_recorded`, `architecture_choice_recorded`, `autonomy_boundary_decision_recorded`, `verification_waiver_recorded`, `qa_waiver_recorded`, `acceptance_recorded`, `residual_risk_accepted`, `reconcile_resolved`.

Projection jobs enqueued: `TASK`; `DEC` when standalone Decision Packet projection is enabled; `APR` for approval where applicable; `MANUAL-QA` for QA waiver when represented as a QA record; affected design/task projections for reconcile. Decision Packet visibility still appears through `TASK` projections, status/next responses, judgment-context resources, and decision-packet resources.

Validators run: `state_envelope`, `pending_decision_packet_exists`, `decision_quality_check`, `autonomy_boundary_check`, `approval_scope`, `qa_waiver_reason`, `residual_risk_visibility_check`, `reconcile_target_validity`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_UNRESOLVED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `SCOPE_VIOLATION`, `QA_REQUIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `RECONCILE_REQUIRED`, `PROJECTION_STALE`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated decision returns the same Decision Packet resolution, accepted-risk refs, updated records, and events; attempting to change an already-recorded decision with the same key returns `STATE_CONFLICT`.

### `harness.launch_verify`

Purpose: create a detached verification run or manual evaluator bundle.

Allowed actor: `lead_agent`, `operator`.

Request schema:

```yaml
LaunchVerifyRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  verification_mode: fresh_session | fresh_worktree | sandbox | manual_bundle
  evaluator_surface_id: string | null
  baseline_ref: string
  include_artifacts: ArtifactRef[]
  bundle_artifact_input: ArtifactInput | null
  evaluator_focus: string[]
```

`include_artifacts` references already registered evidence to include in or link from the bundle. `bundle_artifact_input` is optional; when it is `null`, Core assembles and registers the verification bundle. When it is present, Core validates and registers the supplied staged bundle instead.

Response schema:

```yaml
LaunchVerifyResponse:
  base: ToolResponseBase
  evaluator_run_id: string | null
  bundle_ref: ArtifactRef
  state: StateSummary
  evaluator_instructions: string
  independence_expected:
    context: fresh_session | fresh_worktree | sandbox | manual_bundle
    write_capable: boolean
```

State transition summary: records verification launch, sets or keeps `verification_gate=pending`, and creates evaluator run/bundle references.

Events emitted: `verification_launched`, `verification_bundle_created`, `evaluator_run_created`.

Projection jobs enqueued: `TASK`; optionally `EVIDENCE-MANIFEST`.

Validators run: `state_envelope`, `evidence_sufficiency`, `baseline_freshness`, `artifact_integrity`, `surface_capability_check`, `same_session_verify_guard`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `EVIDENCE_INSUFFICIENT`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `VALIDATOR_FAILED`.

Idempotency behavior: repeated request returns the same evaluator run and bundle ref; included artifact refs and bundle artifact input must match the original payload, and staged bundle contents must be byte-identical for the same key.

### `harness.record_eval`

Purpose: record a verification result and update verification gate/assurance when independence is valid.

Allowed actor: `evaluator`, `operator`.

Request schema:

```yaml
RecordEvalRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  evaluator_run_id: string | null
  target_run_id: string | null
  verdict: passed | failed | blocked | inconclusive
  checks_performed:
    - check_id: string
      result: passed | failed | skipped | blocked
      summary: string
  evidence_reviewed:
    state_refs: StateRecordRef[]
    artifact_refs: ArtifactRef[]
  independence:
    context: same_session | subagent_context | fresh_session | fresh_worktree | sandbox | manual_bundle
    write_capable: boolean
    baseline_reverified: boolean
    evaluator_surface_id: string
    parent_run_id: string | null
  blockers: string[]
  artifact_inputs: ArtifactInput[]
```

Core may derive `change_unit_id` from `target_run_id` or the evidence bundle when it is omitted, but supplying it explicitly improves projection and template alignment when the Eval applies to a Change Unit.

Response schema:

```yaml
RecordEvalResponse:
  base: ToolResponseBase
  eval_id: string
  state: StateSummary
  assurance_updated: boolean
  eval_ref: StateRecordRef
  registered_artifacts: ArtifactRef[]
  next_action: string
```

State transition summary: records Eval; passed detached verification can set `verification_gate=passed` and `assurance_level=detached_verified`; failed or blocked Eval moves gate to failed/blocked; same-session or invalid independence cannot upgrade assurance.

Events emitted: `eval_recorded`, `verification_passed`, `verification_failed`, `verification_blocked`, `assurance_updated`, `verify_not_detached_detected`.

Projection jobs enqueued: `TASK`, `EVAL`; optionally `EVIDENCE-MANIFEST`.

Validators run: `state_envelope`, `same_session_verify_guard`, `baseline_freshness`, `artifact_integrity`, `evidence_sufficiency`, `approval_scope`, `surface_capability_check`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `VERIFY_NOT_DETACHED`, `EVIDENCE_INSUFFICIENT`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `VALIDATOR_FAILED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated request returns the same Eval and assurance decision; a changed verdict, independence payload, or artifact input with the same key returns `STATE_CONFLICT`.

### `harness.record_manual_qa`

Purpose: record human QA result and update `qa_gate` when required QA is satisfied, failed, or waived.

Allowed actor: `user`, `operator`, `evaluator`.

Request schema:

```yaml
RecordManualQaRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  qa_profile: ui_quality | workflow | copy | accessibility | browser_smoke | performance_smoke | other
  performed_by: string
  result: passed | failed | waived
  findings:
    - severity: info | warning | error | blocker
      summary: string
      path: string | null
  artifact_inputs: ArtifactInput[]
  waiver_reason: string | null
  waiver_decision_packet_ref: StateRecordRef | null
  next_action: rework | accept | waive | block | none
```

`change_unit_id` should be supplied when Manual QA applies to a Change Unit. It may be `null` for Task-level QA that is not scoped to a single Change Unit.

For `result=waived`, product/user risk or policy-required judgment requires a `qa_waiver` Decision Packet referenced by `waiver_decision_packet_ref`. `waiver_reason` alone is allowed only for a low-risk waiver when policy permits it.

Response schema:

```yaml
RecordManualQaResponse:
  base: ToolResponseBase
  manual_qa_record_id: string
  state: StateSummary
  manual_qa_ref: StateRecordRef
  registered_artifacts: ArtifactRef[]
  next_action: string
```

State transition summary: records Manual QA; `passed` can set `qa_gate=passed`; `failed` sets `qa_gate=failed` and routes to rework/blocked; `waived` requires either a compatible `qa_waiver` Decision Packet or a policy-permitted low-risk waiver reason and sets `qa_gate=waived`.

Events emitted: `manual_qa_recorded`, `qa_passed`, `qa_failed`, `qa_waived`, `artifact_registered`.

Projection jobs enqueued: `TASK`, `MANUAL-QA`; `DEC` when standalone Decision Packet projection is enabled and a waiver Decision Packet affects visibility; optionally `EVIDENCE-MANIFEST`. Waiver Decision Packet visibility still appears through `TASK` projections, status/next responses, judgment-context resources, and decision-packet resources.

Validators run: `state_envelope`, `manual_qa_required`, `decision_quality_check`, `residual_risk_visibility_check`, `qa_waiver_reason`, `artifact_integrity`, `evidence_sufficiency`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `QA_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `ARTIFACT_MISSING`, `EVIDENCE_INSUFFICIENT`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated request returns the same Manual QA record and gate update; waiver reason and artifact inputs must match.

### `harness.close_task`

Purpose: close, cancel, or supersede a Task after Core checks all close-relevant gates.

Allowed actor: `user`, `lead_agent`, `operator`.

Request schema:

```yaml
CloseTaskRequest:
  envelope: ToolEnvelope
  task_id: string
  intent: complete | cancel | supersede
  requested_close_reason: completed_verified | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  user_note: string | null
  superseded_by_task_id: string | null
```

Response schema:

```yaml
CloseTaskResponse:
  base: ToolResponseBase
  closed: boolean
  state: StateSummary
  blockers:
    - code: ErrorCode
      message: string
      required_next_action: string
      related_refs: StateRecordRef[]
  final_report_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
```

Close blockers include unresolved, missing, deferred-without-coverage, blocked, rejected, stale, or incompatible blocking Decision Packets, and known close-relevant residual risk that is not visible before any successful close. A risk-accepted close additionally requires visible and accepted Residual Risk refs. Acceptance, when required, can be recorded only after close-relevant residual risk is visible.

State transition summary: successful completion moves Task to `completed` with result and close reason; cancellation/supersession moves Task to `cancelled`; failed close leaves Task non-terminal and reports blockers.

Events emitted: `close_requested`, `task_closed`, `task_cancelled`, `task_superseded`, `close_blocked`.

Projection jobs enqueued: `TASK`; latest required reports as needed for final freshness.

Validators run: `state_envelope`, `active_run_absent`, `active_change_unit_complete`, `scope_coverage`, `decision_gate_check`, `decision_quality_check`, `autonomy_boundary_check`, `approval_scope`, `design_gate_close`, `feedback_loop_check`, `tdd_trace_required`, `codebase_stewardship_check`, `evidence_sufficiency`, `same_session_verify_guard`, `manual_qa_required`, `residual_risk_visibility_check`, `acceptance_required`, `projection_freshness`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `EVIDENCE_INSUFFICIENT`, `VERIFY_NOT_DETACHED`, `QA_REQUIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `PROJECTION_STALE`, `RECONCILE_REQUIRED`, `ARTIFACT_MISSING`, `BASELINE_STALE`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated successful close returns the same terminal state and report refs; a second close with a different intent or close reason returns `STATE_CONFLICT`.
