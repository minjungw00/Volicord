# API Schema Later

## What this document helps you do

Use this appendix for later/profile-gated API material that must stay out of the MVP-1 path: separate next-action reads, verification/Eval/Manual QA methods, richer artifact/ref values, validator IDs, waiver/reconcile branches, and future diagnostic resources.

This document preserves reference shapes for future Harness behavior. It does not make these methods or fields active for Engineering Checkpoint or MVP-1, and it does not imply that a runtime/server implementation exists in this repository today.

## Later material map

| Later/profile-gated material | Section |
|---|---|
| Separate next-action method | [`harness.next`](#harnessnext) |
| Detached verification launch | [`harness.launch_verify`](#harnesslaunch_verify) |
| Eval recording | [`harness.record_eval`](#harnessrecord_eval) |
| Manual QA recording | [`harness.record_manual_qa`](#harnessrecord_manual_qa) |
| Later read-only resources | [Later read-only resources](#later-read-only-resources) |
| Later Record Run branches | [Later `harness.record_run` branches](#later-harnessrecord_run-branches) |
| Later user-judgment branches | [Later user judgment branches](#later-user-judgment-branches) |
| Validator IDs | [ValidatorResult stable IDs](#validatorresult-stable-ids) |

## Profile rule

The schema blocks below are exact only when their owner profile is active. Public validators must reject these methods, enum values, and extension branches in Engineering Checkpoint and minimum MVP-1 unless an owner document promotes the matching profile.

## Later read-only resources

These resources are profile-gated reads. Reading them must not repair projections, create owner records, accept evidence, perform verification, record Manual QA, accept results, accept residual risk, create Write Authorization records, make product writes compatible, or close a Task.

| Resource | Profile meaning |
|---|---|
| `harness://policy/sensitive-categories` | Read-only sensitive-action category policy. It does not grant sensitive-action permission. |
| `harness://task/{task_id}/evidence-manifest` | Evidence coverage and manifest-oriented read when the evidence/assurance profile is enabled. |
| `harness://project/surfaces` | Surface/profile inventory and connector-operational status for operations profiles. |
| `harness://task/{task_id}/reports/latest` | Latest report refs and freshness for operations/readiness. |
| `harness://task/{task_id}/bundle/current` | Bundle/export-oriented refs for handoff or recovery profiles. |
| `harness://task/{task_id}/spine` | Journey Spine-style diagnostic reconstruction. |
| `harness://task/{task_id}/journey` | Journey/current-position diagnostic read. |
| `harness://task/{task_id}/change-unit-dag` | Change Unit dependency summaries for diagnostic dependency views. |
| `harness://design/domain-language` | Domain-language read from design owner records. |
| `harness://design/module-map` | Module-map read from design owner records. |
| `harness://design/interface-contracts` | Interface-contract read from design owner records. |

<a id="harnessnext"></a>

## `harness.next`

This method is not part of the minimum MVP-1 API. MVP-1 uses [`harness.status.next_actions`](mvp-api.md#harnessstatus). Keep `harness.next` as an expanded/compatibility read only when a profile or client needs a separate next-action payload.

Allowed actors: `user`, `lead_agent`, `evaluator`, `operator`.

```yaml
NextRequest:
  envelope: ToolEnvelope
  task_id: string | null
  focus: status | shaping | judgment | implementation | verification | qa | acceptance | reconcile
  include_instruction_bundle: boolean

NextResponse:
  base: ToolResponseBase
  state: StateSummary | null
  next_action: NextActionSummary
  recommended_playbooks: RecommendedPlaybook[]
  instruction_bundle:
    summary: string
    constraints: string[]
    relevant_refs: StateRecordRef[]
    artifact_refs: ArtifactRef[]
  pending_user_judgments: StateRecordRef[]
  judgment_context: JudgmentContext | null
  autonomy_boundary: AutonomyBoundarySummary | null
```

`harness.next` is read-only. It does not mutate state, create Write Authorization records, make product writes compatible, record user judgment, satisfy gates, accept work, accept residual risk, or close a Task.

Later/profile-gated `NextActionSummary.action_kind` values include `launch_verify`, `record_eval`, `record_manual_qa`, and `reconcile` only when the matching owner profile is active.

## Recommended playbooks and judgment context

```yaml
RecommendedPlaybook:
  playbook_id: string
  label: string
  reason: string
  applies_to:
    focus: status | shaping | judgment | implementation | verification | qa | acceptance | reconcile
    state_refs: StateRecordRef[]
  route:
    display_route: continue_guidance | show_existing_user_judgment | propose_user_judgment_request | write_readiness_guidance | evidence_guidance | verification_guidance | manual_qa_guidance | close_readiness_guidance | reconcile_guidance
    user_judgment_ref: StateRecordRef | null
    judgment_path: none | existing_user_judgment | user_judgment_candidate_or_request_path
  guidance_refs: StateRecordRef[]
  authority_note: string

JudgmentContext:
  task_ref: StateRecordRef
  journey_card: JourneyCardSummary | null
  current_state_summary: StateSummary
  minimum_context: string[]
  relevant_refs: StateRecordRef[]
  evidence_refs: EvidenceRefs
  active_user_judgment_refs: StateRecordRef[]
  optional_pull_refs: StateRecordRef[]
  stale_or_missing_refs: StateRecordRef[]
  acceptance_visibility: AcceptanceVisibilityContext | null

JourneyCardSummary:
  task_id: string
  state: StateSummary
  current_position: string
  next_action: string
  recommended_playbooks: RecommendedPlaybook[]
  active_change_unit_ref: StateRecordRef | null
  write_authority_summary: WriteAuthoritySummary | null
  active_user_judgment_refs: StateRecordRef[]
  blocker_refs: StateRecordRef[]
  residual_risk_summary: ResidualRiskSummary | null
  projection_freshness:
    status: current | stale | failed | unknown
    stale_refs: StateRecordRef[]
```

Recommended playbooks and route fields are display/routing metadata. They have no state transition, event, projection, gate, write, evidence, verification, QA, risk, acceptance, or close effect by themselves.

## Later `harness.record_run` branches

These branches extend [MVP `harness.record_run`](mvp-api.md#harnessrecord_run) only when their owner profile is active.

```yaml
RecordRunRequest later-profile extension:
  kind: verification_input

RecordRunPayload later-profile extensions:
  verification_input: VerificationInputPayload | null

ShapingUpdatePayload later-profile extensions:
  feedback_loop_updates: FeedbackLoopUpdate[]

ImplementationPayload later-profile extensions:
  tdd_trace_update: TddTraceUpdate | null

EvidenceUpdates later-profile extensions:
  feedback_loop_updates: FeedbackLoopUpdate[]

VerificationInputPayload:
  evaluator_bundle_input: ArtifactInput | null
  evaluator_focus: string[]
  observed_changes: ObservedChanges
  command_results: CommandResult[]

FeedbackLoopUpdate:
  feedback_loop_id: string | null
  operation: create | update
  change_unit_id: string | null
  loop_kind: test | typecheck | lint | build | browser_smoke | manual_qa | tdd | eval | operational | alternate | null
  loop_profile: string | null
  planned_loop: string | null
  selected_loop_refs: StateRecordRef[]
  execution_refs: StateRecordRef[]
  artifact_inputs: ArtifactInput[]
  tdd_trace_refs: StateRecordRef[]
  manual_qa_record_refs: StateRecordRef[]
  evidence_manifest_refs: StateRecordRef[]
  status: defined | executed | waived | blocked | stale | null
  waiver_reason: string | null
  alternate_loop: string | null

TddTraceUpdate:
  tdd_trace_id: string | null
  status: required | recorded | waived | not_required
  red_inputs: ArtifactInput[]
  green_inputs: ArtifactInput[]
  refactor_inputs: ArtifactInput[]
  non_tdd_justification: string | null

RecordRunResponse later-profile extensions:
  evidence_manifest_ref: StateRecordRef | null
  updated_feedback_loop_refs: StateRecordRef[]
```

## Later user judgment branches

These branches extend `UserJudgmentPayload` only when waiver, reconcile, or richer assurance profiles are active.

```yaml
UserJudgmentPayload later-profile extensions:
  waiver: WaiverJudgment | null
  reconcile: ReconcileJudgment | null

WaiverJudgment:
  skipped_check: string
  waiver_reason: string
  gate_or_close_impact: string
  residual_risk_refs: StateRecordRef[]
  follow_up: string | null

ReconcileJudgment:
  reconcile_item_id: string
  target_refs: StateRecordRef[]
  options: JudgmentOption[]
```

Waivers do not perform the skipped check, create detached verification, create Manual QA pass, satisfy evidence, accept work, or accept unrelated residual risk. Reconcile display does not become state until the owner path commits a compatible outcome.

## Full profile-gated ref values

These full enums preserve reference stability. Active values are filtered by [Schema Core: Stage-Specific Active Value Sets](schema-core.md#stage-specific-active-value-sets).

```yaml
ArtifactRef.kind:
  diff | log | screenshot | checkpoint | bundle | manifest | qa_capture | export_component | design_probe | prototype | architecture_scan | decision_context | other

ArtifactInput.relation.record_kind:
  task | change_unit | run | user_judgment | residual_risk | shared_design | evidence_manifest | eval | manual_qa_record | feedback_loop | tdd_trace | journey_spine_entry | projection

StateRecordRef.record_kind:
  task | change_unit | run | approval | write_authorization | user_judgment | residual_risk | evidence_summary | close_readiness | shared_design | domain_term | module_map_item | interface_contract | feedback_loop | evidence_manifest | eval | manual_qa_record | tdd_trace | change_unit_dependency | reconcile_item | projection
```

`decision_packet` is a legacy compatibility alias for user-judgment/full-format presentation material. New payloads should use `user_judgment`.

## ValidatorResult stable IDs

Validator emission is later/profile-gated unless an owner promotes a specific check. Stable IDs are:

- `decision_gate_check`
- `decision_quality_check`
- `autonomy_boundary_check`
- `feedback_loop_check`
- `tdd_trace_required`
- `codebase_stewardship_check`
- `residual_risk_visibility_check`
- `shared_design_alignment`
- `vertical_slice_shape`
- `domain_language_consistency`
- `module_interface_review`
- `manual_qa_required`
- `context_hygiene_check`
- `surface_capability_check`

Core checks may still block transitions, update gates, populate blocked reasons, or appear in fixture assertions. They are not validator IDs unless listed here or promoted by the validator owner.

<a id="harnesslaunch_verify"></a>

## `harness.launch_verify`

Purpose: create a detached verification run or manual evaluator bundle.

Stage/profile: Assurance Profile only. It creates a detached candidate or bundle; it does not by itself create detached assurance.

`verification_mode=sandbox` is a later-profile value. A profile must name and prove the actual isolation boundary before any `isolated` guarantee is shown. This value is not active for MVP-1, and a fresh session, fresh worktree, or manual bundle does not imply OS sandboxing, permission isolation, or tamper-proof storage.

Allowed actors: `lead_agent`, `operator`.

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

`bundle_ref` is an `ArtifactRef`, usually with `kind=bundle` or `kind=manifest`. It does not create a `verification_bundle` state record.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `EVIDENCE_INSUFFICIENT`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `VALIDATOR_FAILED`.

<a id="harnessrecord_eval"></a>

## `harness.record_eval`

Purpose: record a verification result and update verification gate/assurance only when independence is valid.

Stage/profile: Assurance Profile only. Same-session checks, self-check summaries, or passed commands are not detached verification unless this method records a qualifying Eval and Core updates gate/assurance state.

Allowed actors: `evaluator`, `operator`.

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

RecordEvalResponse:
  base: ToolResponseBase
  eval_id: string
  state: StateSummary
  assurance_updated: boolean
  eval_ref: StateRecordRef
  registered_artifacts: ArtifactRef[]
  next_action: string
```

`verdict=passed` is necessary but not sufficient for an assurance upgrade. Core may set `assurance_updated=true` only when the Eval passed, independence is valid, inputs are current or reverified, and artifact/baseline checks pass.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `VERIFY_NOT_DETACHED`, `EVIDENCE_INSUFFICIENT`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `VALIDATOR_FAILED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`.

<a id="harnessrecord_manual_qa"></a>

## `harness.record_manual_qa`

Purpose: record a human Manual QA outcome and update `qa_gate` when required QA is satisfied, failed, or waived.

Stage/profile: Assurance Profile only when Manual QA policy/profile is enabled. Browser captures or notes do not replace the human QA record or valid waiver path.

Allowed actors: `user`, `operator`, `evaluator`.

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
  waiver_user_judgment_ref: StateRecordRef | null
  feedback_loop_ref: StateRecordRef | null
  next_action: rework | accept | waive | block | none

RecordManualQaResponse:
  base: ToolResponseBase
  manual_qa_record_id: string
  state: StateSummary
  manual_qa_ref: StateRecordRef
  updated_feedback_loop_refs: StateRecordRef[]
  registered_artifacts: ArtifactRef[]
  next_action: string
```

For `result=waived`, product/user risk or policy-required judgment requires a compatible technical user judgment referenced by `waiver_user_judgment_ref`. `waiver_reason` alone is allowed only for a low-risk waiver when policy permits it.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `QA_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `ARTIFACT_MISSING`, `EVIDENCE_INSUFFICIENT`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.
