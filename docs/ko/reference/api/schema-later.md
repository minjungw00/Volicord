# API Schema Later

## 이 문서로 할 수 있는 일

MVP-1 path 밖에 두어야 하는 later/profile-gated API material을 확인할 때 이 appendix를 사용합니다. 별도 next-action read, verification/Eval/Manual QA method, richer artifact/ref value, validator ID, waiver/reconcile branch, future diagnostic resource를 다룹니다.

이 문서는 향후 하네스 동작의 reference shape를 보존합니다. 이 method나 field가 내부 엔지니어링 점검 또는 MVP-1에서 active라는 뜻이 아니며, 현재 저장소에 runtime/server 구현이 있다는 뜻도 아닙니다.

## Later material map

| Later/profile-gated material | Section |
|---|---|
| 별도 next-action method | [`harness.next`](#harnessnext) |
| Detached verification launch | [`harness.launch_verify`](#harnesslaunch_verify) |
| Eval recording | [`harness.record_eval`](#harnessrecord_eval) |
| Manual QA recording | [`harness.record_manual_qa`](#harnessrecord_manual_qa) |
| Later read-only resources | [Later read-only resources](#later-read-only-resources) |
| Later Record Run branches | [Later `harness.record_run` branches](#later-harnessrecord_run-branches) |
| Later user-judgment branches | [Later user judgment branches](#later-user-judgment-branches) |
| Later close and assurance fields | [Later close and assurance extensions](#later-close-and-assurance-extensions) |
| Validator IDs | [ValidatorResult stable IDs](#validatorresult-stable-ids) |

## Profile rule

아래 schema block은 owner profile이 active일 때만 exact합니다. Public validator는 matching profile을 owner 문서가 승격하지 않는 한 Engineering Checkpoint와 minimum MVP-1에서 이 method, enum value, extension branch를 reject해야 합니다. [Schema Core](schema-core.md)의 활성 MVP-1 schema block은 이 later value를 의도적으로 포함하지 않습니다. 그래야 generated MVP-1 validator와 client가 해당 값을 실수로 accept하지 않습니다.

<a id="later-close-and-assurance-extensions"></a>

## Later close and assurance extensions

이 field들은 보증 프로필이나 다른 owner profile이 detached verification, Manual QA, projection/report freshness를 close-relevant behavior로 명시적으로 켰을 때만 [MVP `harness.close_task`](mvp-api.md#harnessclose_task)와 [Schema Core `StateSummary`](schema-core.md#shared-schemas)를 확장합니다. Minimum MVP-1 validator는 이 값과 field를 reject해야 합니다.

```yaml
StateSummary later-profile extensions:
  lifecycle_phase: verifying | qa
  close_reason: completed_verified
  assurance_level: detached_verified
  gates:
    verification_gate: not_required | required | pending | passed | failed | waived_by_user | blocked
    qa_gate: not_required | required | pending | passed | failed | waived

CloseTaskRequest later-profile extension:
  requested_close_reason: completed_verified

CloseTaskResponse later-profile extensions:
  close_reason: completed_verified
  assurance_level: detached_verified
  profile_required_verification:
    active: boolean
    status: not_required | required | pending | passed | failed | waived_by_user | blocked
    required_profile: string | null
    related_refs: StateRecordRef[]
  blockers[].category:
    verification | manual_qa | projection_freshness
  blockers[].required_judgment_kind:
    qa_waiver | verification_risk_acceptance
```

`completed_verified`와 `assurance_level=detached_verified`는 active profile 아래에서 qualifying Eval이 valid independence와 current input을 가질 때만 valid합니다. `profile_required_verification`은 later/profile response field이며 MVP-1 close field가 아닙니다. Manual QA blocker는 active Manual QA owner profile이 필요합니다. Projection freshness는 display/readiness material이며 그 자체로 canonical close state가 되면 안 됩니다.

## Later read-only resources

이 resources는 profile-gated read입니다. 읽는 것만으로 projection을 repair하거나 owner record를 만들거나 evidence, verification, Manual QA, result, residual risk를 accept하거나 Write Authorization record를 만들거나 product write를 compatible하게 하거나 Task를 close하면 안 됩니다.

| Resource | Profile meaning |
|---|---|
| `harness://policy/sensitive-categories` | Read-only sensitive-action category policy. 민감 동작 승인을 grant하지 않습니다. |
| `harness://task/{task_id}/evidence-manifest` | Evidence/assurance profile이 enabled일 때 evidence coverage와 manifest-oriented read. |
| `harness://project/surfaces` | Operations profile을 위한 surface/profile inventory와 connector-operational status. |
| `harness://task/{task_id}/reports/latest` | Operations/readiness를 위한 latest report refs와 freshness. |
| `harness://task/{task_id}/bundle/current` | Handoff 또는 recovery profile을 위한 bundle/export-oriented refs. |
| `harness://task/{task_id}/spine` | Journey Spine-style diagnostic reconstruction. |
| `harness://task/{task_id}/journey` | Journey/current-position diagnostic read. |
| `harness://task/{task_id}/change-unit-dag` | Diagnostic dependency view를 위한 Change Unit dependency summaries. |
| `harness://design/domain-language` | Design owner record에서 읽는 domain-language read. |
| `harness://design/module-map` | Design owner record에서 읽는 module-map read. |
| `harness://design/interface-contracts` | Design owner record에서 읽는 interface-contract read. |

<a id="harnessnext"></a>

## `harness.next`

이 method는 minimum MVP-1 API에 속하지 않습니다. MVP-1은 [`harness.status.next_actions`](mvp-api.md#harnessstatus)를 사용합니다. 별도 next-action payload가 필요한 profile이나 client를 위해 expanded/compatibility read로만 유지합니다.

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

`harness.next`는 read-only입니다. State를 변경하거나, Write Authorization record를 만들거나, product write를 compatible하게 하거나, user judgment를 기록하거나, gate를 충족하거나, work를 accept하거나, residual risk를 accept하거나, Task를 close하지 않습니다.

<a id="later-next-action-values"></a>

### Later next-action values

Later/profile-gated `NextActionSummary.action_kind` extension values는 다음과 같습니다.

```yaml
NextActionSummary.action_kind later-profile extension:
  launch_verify | record_eval | record_manual_qa | reconcile
```

이 값은 matching owner profile이 active일 때만 valid합니다. Minimum MVP-1 validator는 reject해야 합니다.

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

Recommended playbook과 route field는 display/routing metadata입니다. 이것만으로 state transition, event, projection, gate, write, evidence, verification, QA, risk, acceptance, close effect가 생기지 않습니다.

<a id="later-harnessrecord_run-branches"></a>

## Later `harness.record_run` branches

이 branch는 owner profile이 active일 때만 [MVP `harness.record_run`](mvp-api.md#harnessrecord_run)을 확장합니다.

```yaml
RecordRunRequest later-profile extension:
  kind: verification_input

RecordRunPayload later-profile extensions:
  kind: verification_input
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

나중 프로필이 `verification_input`을 활성화해도 [Schema Core: Record-run payloads](schema-core.md#record-run-payloads)의 일대일 branch rule은 그대로 적용됩니다. `RecordRunRequest.kind`, `RecordRunPayload.kind`, non-null branch가 서로 맞아야 합니다.

## Later user judgment branches

이 branch는 waiver, reconcile, residual-risk, 더 풍부한 assurance profile이 active일 때만 `UserJudgmentPayload`와 활성 residual-risk acceptance input을 확장합니다.

```yaml
UserJudgmentGateRef.gate later-profile extension:
  verification_gate | qa_gate

AcceptanceJudgment later-profile extensions:
  verification_status_refs: StateRecordRef[]
  qa_status_refs: StateRecordRef[]

AcceptanceVisibilityContext later-profile extensions:
  verification_status: not_required | required | pending | passed | failed | waived_by_user | blocked
  qa_status: not_required | required | pending | passed | failed | waived

UserJudgmentPayload later-profile extensions:
  waiver: WaiverJudgment | null
  reconcile: ReconcileJudgment | null

AcceptedRiskInput later-profile extensions:
  residual_risk_ref: StateRecordRef | null
  residual_risk_status: visible | accepted | blocked | superseded | stale | null
  owner_review_refs: StateRecordRef[]
  expires_at: string | null

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

Waiver는 skipped check를 수행하지 않고, detached verification, Manual QA pass, evidence satisfaction, final acceptance, unrelated residual risk acceptance를 만들지 않습니다. Reconcile display는 owner path가 compatible outcome을 commit하기 전까지 state가 되지 않습니다.

<a id="later-profile-ref-and-artifact-values"></a>

## Later-profile ref and artifact values

아래 enum extension은 reference stability를 보존하되 활성 MVP-1 schema를 closed enum으로 유지하기 위한 것입니다. Matching owner profile이 active schema를 명시적으로 확장하지 않는 한 [Schema Core](schema-core.md)는 이 값을 accept하지 않습니다.

```yaml
ArtifactRef.kind / ArtifactInput.kind later-profile extension:
  bundle | manifest | qa_capture | export_component | design_probe | prototype | architecture_scan | decision_context

ArtifactRef.retention_class / ArtifactInput.retention_class later-profile extension:
  export

ArtifactRef.relation_owner.record_kind / ArtifactInput.relation.record_kind later-profile extension:
  residual_risk | shared_design | evidence_manifest | eval | manual_qa_record | feedback_loop | tdd_trace | projection | journey_spine_entry

StateRecordRef.record_kind later-profile extension:
  approval | residual_risk | close_readiness | shared_design | domain_term | module_map_item | interface_contract | feedback_loop | evidence_manifest | eval | manual_qa_record | tdd_trace | change_unit_dependency | reconcile_item | projection

StateRecordRef projection-profile extension:
  projection_path: string | null
```

`record_kind=projection`에서 `record_id`는 운영/projection profile이 active일 때 projection job identity입니다. `projection_path`는 optional display/recovery metadata이지 alternate key가 아닙니다. `projection` 또는 `close_readiness` 같은 derived-view ref는 읽기용 보기 또는 later/profile-promoted display record를 가리킵니다. 그 보기 뒤의 owner record를 대체하지 않습니다.

`decision_packet`은 user-judgment/full-format presentation material을 위한 legacy compatibility alias입니다. 새 payload는 `user_judgment`를 사용해야 합니다.

## ValidatorResult stable IDs

추가 validator kind는 owner가 특정 check를 승격하지 않는 한 later/profile-gated입니다.

```yaml
ValidatorResult.validator_kind later-profile extension:
  state | scope | decision | approval | evidence | verification | qa | acceptance | design | autonomy_boundary | residual_risk | artifact | projection | connector
```

활성 MVP-1의 `capability` validator kind와 `surface_capability_check` ID는 [Schema Core: ValidatorResult](schema-core.md#validatorresult)가 소유합니다. Later/profile stable IDs:

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

Core check는 transition을 block하거나, gate를 update하거나, blocked reason을 채우거나, fixture assertion에 나타날 수 있습니다. 여기에 listed되었거나 validator owner가 승격하지 않는 한 validator ID가 아닙니다.

<a id="harnesslaunch_verify"></a>

## `harness.launch_verify`

Detached verification run 또는 manual evaluator bundle을 만들 때 이 method를 사용합니다.

Stage/profile: Assurance Profile only입니다. Detached candidate 또는 bundle을 만들 뿐, detached assurance를 만들지는 않습니다.

`verification_mode=sandbox`는 later-profile value입니다. 어떤 profile이 `isolated` guarantee를 표시하려면 실제 격리 경계를 이름 붙이고 증명해야 합니다. 이 value는 MVP-1에서 active가 아니며, fresh session, fresh worktree, manual bundle이 OS sandboxing, 권한 격리, 변조 방지 storage를 뜻하지 않습니다.

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

`bundle_ref`는 보통 `kind=bundle` 또는 `kind=manifest`인 `ArtifactRef`입니다. `verification_bundle` state record를 만들지 않습니다.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `EVIDENCE_INSUFFICIENT`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `VALIDATOR_FAILED`.

<a id="harnessrecord_eval"></a>

## `harness.record_eval`

Verification result를 기록하고 independence가 valid할 때만 verification gate/assurance를 update하기 위해 이 method를 사용합니다.

Stage/profile: Assurance Profile only입니다. Same-session check, self-check summary, passed command는 이 method가 qualifying Eval을 기록하고 Core가 gate/assurance state를 update하기 전까지 detached verification이 아닙니다.

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

`verdict=passed`는 assurance upgrade의 필요조건이지만 충분조건은 아닙니다. Eval이 passed이고, independence가 valid이며, input이 current 또는 reverified이고, artifact/baseline check가 통과해야 Core가 `assurance_updated=true`로 둘 수 있습니다.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `VERIFY_NOT_DETACHED`, `EVIDENCE_INSUFFICIENT`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `VALIDATOR_FAILED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`.

<a id="harnessrecord_manual_qa"></a>

## `harness.record_manual_qa`

Human Manual QA outcome을 기록하고 required QA가 satisfied, failed, waived인지 `qa_gate`를 update할 때 이 method를 사용합니다.

Stage/profile: Manual QA policy/profile이 enabled된 Assurance Profile only입니다. Browser capture나 note는 human QA record 또는 valid waiver path를 대신하지 않습니다.

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

`result=waived`에서 product/user risk 또는 policy-required judgment가 있으면 `waiver_user_judgment_ref`가 가리키는 compatible technical user judgment가 필요합니다. `waiver_reason`만으로 허용되는 것은 policy가 low-risk waiver를 허용할 때뿐입니다.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `QA_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `ARTIFACT_MISSING`, `EVIDENCE_INSUFFICIENT`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.
