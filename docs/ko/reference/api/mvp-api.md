# MVP-1 API

## 이 문서로 할 수 있는 일

큰 future schema appendix를 읽지 않고 MVP-1에서 활성인 public API surface를 확인할 때 이 짧은 참조를 사용합니다.

이 문서는 향후 하네스 서버 동작을 계획하고 검토하기 위한 참조입니다. 현재 저장소에는 하네스 runtime이나 server 구현이 없습니다. 현재 저장소 단계와 구현 인계 상태는 [구현 개요](../../build/implementation-overview.md#문서-수락-상태)가 담당합니다.

## 핵심 생각

MVP-1은 작은 local MCP surface만 노출합니다. 평범한 작업 요청을 받아들이고, 현재 상태와 다음 안전한 행동을 보여 주고, 제안된 쓰기가 현재 범위에 맞는지 협력형으로 확인하고, 실행과 근거 ref를 기록하고, 사용자 소유 판단을 요청하고, 사용자의 답을 기록하고, 최소 계약이 허용할 때만 닫습니다.

MVP-1에서는 별도 `harness.next` method를 두지 않습니다. 다음 안전한 행동은 `harness.status.next_actions`에서 읽습니다. 별도 `harness.next`는 [Schema Later](schema-later.md#harnessnext)의 later/compatibility material입니다.

이 API는 OS-level blocking, arbitrary-tool sandboxing, tamper-proof file, pre-tool prevention을 주장하지 않습니다. 쓰기 전에 범위와 권한 상태를 확인하는 협력형 Core 확인이 `harness.prepare_write`입니다. 더 강한 preventive 또는 isolated 주장은 관련 보안/connector 문서에서 owner-promoted profile과 증명이 필요합니다.

## MVP-1 method set

| Method | MVP-1 역할 |
|---|---|
| [`harness.intake`](#harnessintake) | 평범한 말로 들어온 작업을 시작하거나 이어가고, advice/read-only, small direct work, tracked work로 분류합니다. |
| [`harness.status`](#harnessstatus) | 현재 범위, 막힘, 대기 중인 판단, 근거 요약, 다음 행동, 닫기 준비 상태를 반환합니다. |
| [`harness.prepare_write`](#harnessprepare_write) | 제안된 제품 파일 쓰기를 현재 Task, 범위, baseline, 민감 동작 permission, 사용자 판단 coverage와 비교합니다. |
| [`harness.record_run`](#harnessrecord_run) | shaping, implementation, direct run과 최소 artifact/evidence ref를 기록합니다. |
| [`harness.request_user_judgment`](#harnessrequest_user_judgment) | 집중된 사용자 판단 요청을 만듭니다. |
| [`harness.record_user_judgment`](#harnessrecord_user_judgment) | 대기 중인 사용자 판단에 대한 사용자의 답을 기록합니다. |
| [`harness.close_task`](#harnessclose_task) | 닫기 준비 상태를 확인하고, 막힘이 허용할 때만 complete, cancel, supersede합니다. |

## MVP-1이 아닌 것

다음 surface는 owner 문서가 승격하기 전까지 later/profile-gated입니다.

- 별도 `harness.next`
- `harness.launch_verify`
- `harness.record_eval`
- `harness.record_manual_qa`
- sensitive-action approval을 `user_judgment`로 다루는 범위를 넘어선 committed Approval record lifecycle
- full Evidence Manifest, detached verification, full Manual QA matrix, reconcile, export/recover, broad operations, detailed diagnostic projections

## 공통 request 규칙

모든 method는 [`ToolEnvelope`](schema-core.md#tool-envelope)와 [`ToolResponseBase`](schema-core.md#common-response)를 사용합니다. State-changing tool은 non-null `idempotency_key`와 current `expected_state_version`을 요구합니다. Read-only tool은 같은 envelope를 tracing에 사용할 수 있고 `expected_state_version`을 `null`로 둘 수 있습니다.

MVP-1 request validator는 [Schema Core](schema-core.md#stage-specific-active-value-sets)의 active value set을 사용합니다. [Schema Later](schema-later.md)에 존재하는 later enum value나 extension branch는 그 자체로 MVP-1에서 유효해지지 않습니다.

Error code, primary error precedence, idempotency replay, stale-state behavior는 [Errors](errors.md)가 담당합니다.

<a id="harnessintake"></a>

## `harness.intake`

작업을 시작하거나, 분류하거나, 이어갈 때 이 method를 사용합니다.

Stage meaning: 내부 엔지니어링 점검에서는 선택적인 minimal setup path입니다. MVP-1에서는 평범한 말로 시작/이어가기 behavior가 active입니다. Full discovery, design-support routing, broad planning workflow는 명시적으로 승격되기 전까지 later material입니다.

Allowed actors: `user`, `lead_agent`, `operator`.

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

IntakeResponse:
  base: ToolResponseBase
  task_id: string
  created: boolean
  resumed: boolean
  state: StateSummary
  next_action: string
  change_unit_id: string | null
```

Core는 Task를 만들거나 이어가고, work mode를 설정하며, write-capable direct 또는 tracked work에 initial scoped boundary를 만들 수 있습니다. Idempotent replay는 같은 Task/resume 결정을 반환하고, 같은 key에 다른 payload를 쓰면 `STATE_CONFLICT`를 반환합니다.

<a id="harnessstatus"></a>

## `harness.status`

현재 어디에 있고, 무엇이 막고 있고, 다음 안전한 행동이 무엇인지 답할 때 이 method를 사용합니다.

Stage meaning: 내부 엔지니어링 점검에서는 minimal status/blocker output이 active입니다. MVP-1에서는 현재 위치, 대기 중인 사용자 판단, 근거 요약, 닫기 준비 상태, `next_actions`가 active입니다.

Allowed actors: `user`, `lead_agent`, `evaluator`, `operator`.

```yaml
StatusRequest:
  envelope: ToolEnvelope
  include:
    task: boolean
    gates: boolean
    projections: boolean
    pending_user_judgments: boolean
    guarantees: boolean
    user_judgments: boolean
    autonomy_boundary: boolean
    write_authority: boolean
    residual_risk: boolean

StatusResponse:
  base: ToolResponseBase
  active_task: StateSummary | null
  status_card: string
  next_actions: NextActionSummary[]
  pending_user_judgments: StateRecordRef[]
  active_user_judgment_refs: StateRecordRef[]
  autonomy_boundary_summary: AutonomyBoundarySummary | null
  write_authority_summary: WriteAuthoritySummary | null
  residual_risk_summary: ResidualRiskSummary | null
  evidence_summary_refs: StateRecordRef[]
  close_readiness_refs: StateRecordRef[]
  projection_freshness:
    status: current | stale | failed | unknown
    stale_refs: StateRecordRef[]
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]
```

`next_actions`가 MVP-1의 다음 안전한 행동 surface입니다. 사용자에게는 가장 작은 useful next action이나 unblocker를 쉬운 말로 보여 주고, exact enum value는 secondary detail로 둡니다.

MVP-1 active `NextActionSummary.action_kind` values:

```text
ask_user | prepare_write | implement | request_acceptance | close_task | idle
```

Verification, Eval, Manual QA, reconcile, export/recover, operations next-action kind는 later/profile-gated입니다.

Status는 read-only입니다. State를 만들거나, write를 허가하거나, gate를 충족하거나, work acceptance를 기록하거나, residual risk를 받아들이거나, projection repair를 enqueue하거나, Task를 close하면 안 됩니다.

<a id="harnessprepare_write"></a>

## `harness.prepare_write`

에이전트가 제품 파일을 쓰기 전에, 그 정확한 쓰기가 현재 Core state에 맞는지 확인할 때 이 method를 사용합니다. 결과는 compatible single-use Write Authorization이거나 structured blocker입니다.

Stage meaning: 내부 엔지니어링 점검과 MVP-1에서 active입니다. MVP-1에서 sensitive-action permission은 `judgment_type=sensitive_action_approval`인 compatible `user_judgment`로 표현합니다. Committed Approval record는 later-profile material입니다.

Allowed actors: `lead_agent`, `operator`.

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

PrepareWriteResponse:
  base: ToolResponseBase
  decision: allowed | blocked | approval_required | decision_required | state_conflict
  state: StateSummary | null
  change_unit_id: string | null
  baseline_ref: string | null
  write_authorization_ref: StateRecordRef | null
  write_authorization: WriteAuthorizationSummary | null
  authorization_effect: none | would_create | created | returned
  active_user_judgment_refs: StateRecordRef[]
  blocked_reasons:
    - code: string
      message: string
      related_error: ErrorCode
  approval_request_candidate: ApprovalRequestCandidate | null
  user_judgment_candidate: UserJudgmentCandidate | null
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]
```

`decision=allowed`이고 `dry_run=false`이면 `write_authorization_ref`가 있어야 합니다. `dry_run=true`에서는 `authorization_effect=would_create`를 반환할 수 있지만 authorization을 만들지 않습니다. Blocked 또는 judgment-required response는 Write Authorization을 포함하면 안 됩니다.

`approval_request_candidate`와 `user_judgment_candidate`는 non-mutating candidate payload입니다. 이것만으로 user judgment, Approval record, Write Authorization, projection을 만들지 않습니다.

<a id="harnessrecord_run"></a>

## `harness.record_run`

Shaping update, direct result, implementation run 뒤에 이 method를 사용합니다. Implementation 또는 direct product-write Run은 `harness.prepare_write`가 반환한 compatible Write Authorization을 소비합니다.

Stage meaning: 내부 엔지니어링 점검에서는 compatible run 하나와 artifact/evidence ref 하나가 active입니다. MVP-1에서는 evidence summary에 active입니다. Verification input, Feedback Loop update, TDD Trace update, full Evidence Manifest behavior는 later/profile-gated입니다.

Allowed actors: `lead_agent`, `evaluator`, `operator`.

```yaml
RecordRunRequest:
  envelope: ToolEnvelope
  kind: shaping_update | implementation | direct
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

RecordRunResponse:
  base: ToolResponseBase
  run_id: string | null
  state: StateSummary
  write_authorization_ref: StateRecordRef | null
  evidence_summary_ref: StateRecordRef | null
  run_summary_ref: StateRecordRef | null
  direct_result_ref: StateRecordRef | null
  registered_artifacts: ArtifactRef[]
  next_action: string
```

`payload` branch는 `kind`와 일치해야 합니다. MVP-1은 `shaping_update`, `implementation`, `direct`를 허용합니다. `verification_input`은 later-profile only입니다.

Core가 write-capable run을 commit 전에 거절하면 `run_id`는 `null`이고 artifact는 등록되지 않으며 response는 Run이 존재한다고 암시하면 안 됩니다. Violation/audit Run은 제품 쓰기가 이미 관찰된 뒤 Core가 의도적으로 기록할 때만 생길 수 있습니다. 그런 Run은 evidence, QA, verification, work acceptance, close readiness를 충족하지 않습니다.

<a id="harnessrequest_user_judgment"></a>
<a id="harnessrequest_user_decision"></a>

## `harness.request_user_judgment`

Compatibility alias: `harness.request_user_decision`.

사용자 소유 판단, sensitive-action permission, work acceptance, residual-risk acceptance가 progress나 close를 막을 때 focused user judgment request를 만들기 위해 이 method를 사용합니다.

Stage meaning: 내부 엔지니어링 점검에서는 active가 아닙니다. MVP-1에서 active입니다. Full-format Decision Packet presentation, committed Approval record lifecycle, waiver, reconcile, full residual-risk profile은 명시적으로 active가 되기 전까지 later/profile-gated입니다.

Allowed actors: `lead_agent`, `evaluator`, `operator`.

```yaml
RequestUserJudgmentRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  judgment_type: product_choice | technical_choice | sensitive_action_approval | work_acceptance | residual_risk_acceptance
  presentation: short | full
  display_label: Product/UX judgment | Technical judgment | Sensitive action approval | Work acceptance | Residual risk acceptance
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary | null
  what_user_is_judging: string
  what_agent_may_decide_without_user: string[]
  affected_scope: UserJudgmentScope
  affected_gates: UserJudgmentGateRef[]
  affected_acceptance_criteria: UserJudgmentCriterionRef[]
  judgment_payload: UserJudgmentPayload
  expires_at: string | null

RequestUserJudgmentResponse:
  base: ToolResponseBase
  user_judgment_id: string
  user_judgment_ref: StateRecordRef
  user_judgment: UserJudgment
  approval_id: string | null
  reconcile_item_id: string | null
  state: StateSummary
  user_visible_summary: string
```

Minimum MVP-1에서는 `approval_id`가 `null`입니다. Sensitive-action approval judgment는 `harness.record_user_judgment`가 resolve한 뒤에만 scoped permission을 기록합니다. 이것은 Write Authorization이 아니며 product, technical, work-acceptance, residual-risk judgment를 대신하지 않습니다.

<a id="harnessrecord_user_judgment"></a>
<a id="harnessrecord_user_decision"></a>

## `harness.record_user_judgment`

Compatibility alias: `harness.record_user_decision`.

이미 존재하는 canonical `UserJudgment`에 대한 사용자의 답을 기록할 때 이 method를 사용합니다.

Stage meaning: 내부 엔지니어링 점검에서는 active가 아닙니다. MVP-1에서는 사용자 소유 판단, sensitive-action approval judgment resolution, required work acceptance에 active입니다. Committed Approval update, waiver, reconcile outcome, richer residual-risk metadata는 명시적으로 active가 되기 전까지 later/profile-gated입니다.

Allowed actors: `user`, `operator`.

```yaml
RecordUserJudgmentRequest:
  envelope: ToolEnvelope
  user_judgment_id: string
  judgment_type: product_choice | technical_choice | sensitive_action_approval | work_acceptance | residual_risk_acceptance
  selected_option_id: string | null
  judgment: RecordUserJudgmentPayload
  note: string
  waiver_reason: string | null
  accepted_risks: AcceptedRiskInput[]

RecordUserJudgmentPayload:
  value: selected | rejected | deferred | granted | denied | expired | accepted
  value_note: string | null

RecordUserJudgmentResponse:
  base: ToolResponseBase
  user_judgment_id: string
  user_judgment_ref: StateRecordRef
  user_judgment: UserJudgment
  state: StateSummary
  updated_records: StateRecordRef[]
  accepted_risk_refs: StateRecordRef[]
  next_action: string
```

`judgment_type`은 저장된 `UserJudgment`와 일치해야 합니다. `go ahead`, `looks good`, `진행해` 같은 free-form note는 pending judgment가 그 judgment type을 명시적으로 묻고 answer가 allowed value와 맞을 때만 approval, acceptance, risk acceptance, waiver, write authority와 연결될 수 있습니다.

`accepted_risk_refs`는 `record_kind=residual_risk`인 `StateRecordRef`만 포함합니다. 별도 accepted-risk record kind는 없습니다.

<a id="harnessclose_task"></a>

## `harness.close_task`

Task를 complete, cancel, supersede할 수 있는지 Core에 묻기 위해 이 method를 사용합니다.

Stage meaning: 내부 엔지니어링 점검에서는 optional narrow blocker/status smoke입니다. MVP-1에서는 close-readiness와 blocker response가 active입니다. Full assurance, QA, waiver, report freshness, export, operations blocker는 later/profile-gated입니다.

Allowed actors: `user`, `lead_agent`, `operator`.

```yaml
CloseTaskRequest:
  envelope: ToolEnvelope
  task_id: string
  intent: complete | cancel | supersede
  requested_close_reason: completed_verified | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  user_note: string | null
  superseded_by_task_id: string | null

CloseTaskResponse:
  base: ToolResponseBase
  close_state: open | blocked | closed | cancelled | superseded
  closed: boolean
  close_reason: none | completed_verified | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  assurance_level: none | self_checked | detached_verified
  residual_risk_state: ResidualRiskSummary
  acceptance_state:
    status: not_required | required | pending | accepted | rejected
    accepted_by_ref: StateRecordRef | null
    required_before_close: boolean
  profile_required_verification:
    active: boolean
    status: not_required | required | pending | passed | failed | waived_by_user | blocked
    required_profile: string | null
    related_refs: StateRecordRef[]
  state: StateSummary
  blockers:
    - code: ErrorCode
      category: open_run | scope | user_judgment | sensitive_action_approval | design_policy | evidence | verification | manual_qa | residual_risk_visibility | residual_risk_acceptance | work_acceptance | projection_freshness | artifact_availability
      message: string
      required_next_action: string
      related_refs: StateRecordRef[]
  final_report_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
```

MVP-1 close는 core close state, blockers, residual-risk visibility, required work-acceptance state, evidence/close-readiness refs를 사용합니다. Verification, Manual QA, projection/report, operations refs는 해당 profile이 enabled일 때만 active입니다.

`CloseTaskRequest`는 accepted-risk refs를 싣지 않습니다. `completed_with_risk_accepted`에서는 Core가 visible close-relevant Residual Risk record에 이미 기록된 accepted state를 읽고, 그 상태가 없으면 block합니다.

Successful close는 Task를 terminal state로 옮깁니다. Failed close는 Task를 open 상태로 남기고 structured blockers를 반환합니다. 같은 idempotency key의 repeated successful close는 같은 terminal response를 반환하고, conflicting close intent는 `STATE_CONFLICT`를 반환합니다.
