# MVP-1 API

## 이 문서로 할 수 있는 일

큰 future schema appendix를 읽지 않고 MVP-1에서 활성인 public API surface를 확인할 때 이 짧은 참조를 사용합니다.

이 문서는 향후 하네스 서버 동작을 계획하고 검토하기 위한 참조입니다. 현재 저장소에는 하네스 runtime이나 server 구현이 없습니다. 현재 저장소 단계와 구현 인계 상태는 [구현 개요](../../build/implementation-overview.md#문서-수락-상태)가 담당합니다.

## 핵심 생각

MVP-1은 작은 local MCP surface만 노출합니다. 평소 작업 요청을 받아들이고, 현재 상태와 다음 안전한 행동을 보여 주고, 제안된 쓰기가 현재 범위에 맞는지 협력형으로 확인하고, 실행과 근거 ref를 기록하고, 사용자 소유 판단을 요청하고, 사용자의 답을 기록하고, 최소 계약이 허용할 때만 닫습니다.

MVP-1에서는 별도 `harness.next` method를 두지 않습니다. 다음 안전한 행동은 `harness.status.next_actions`에서 읽습니다. 별도 `harness.next`는 [Schema Later](schema-later.md#harnessnext)의 later/compatibility material입니다.

이 API는 OS-level blocking, arbitrary-tool sandboxing, tamper-proof file, pre-tool prevention을 주장하지 않습니다. `harness.prepare_write`는 Core state를 기준으로 하는 협력형 쓰기 전 범위 확인입니다. 반환되는 Write Authorization은 하네스 수준의 기록/확인이지 OS 권한, sandboxing, 변조 방지 enforcement, 사전 차단이 아닙니다. 더 강한 preventive 또는 isolated 주장은 관련 보안/connector 문서에서 owner-promoted profile과 증명이 필요합니다.

Status output은 세 부분 모델을 따릅니다. `harness.status.status_card`는 사용자 상태 카드입니다. Agent 접점은 current status와 ref에서 에이전트 맥락 패킷을 만들 수 있습니다. Core 상태가 유일한 운영 기준입니다. 상태 카드, next-action text, 렌더링된 template, Projection은 read-only view이며 오래된 view는 권한 근거가 아닙니다. 활성 compact view set은 정확히 `status-card`, `agent-context-packet`, `judgment-request`, `run-evidence-summary`, `close-result`입니다. 상세 report surface는 후속/profile 범위에 남습니다.

## MVP-1 method set

| Method | MVP-1 역할 |
|---|---|
| [`harness.status`](#harnessstatus) | 현재 범위, 막힘, 대기 중인 판단, 근거 요약, 다음 행동, 닫기 준비 상태를 반환합니다. |
| [`harness.intake`](#harnessintake) | 평소 말로 들어온 작업을 시작하거나 이어가고, advice/read-only, small direct work, tracked work로 분류합니다. |
| [`harness.request_user_judgment`](#harnessrequest_user_judgment) | 집중된 사용자 판단 요청을 만듭니다. |
| [`harness.record_user_judgment`](#harnessrecord_user_judgment) | 대기 중인 사용자 판단에 대한 사용자의 답을 기록합니다. |
| [`harness.prepare_write`](#harnessprepare_write) | 제안된 제품 파일 쓰기를 현재 Task, 범위, baseline, 민감 동작 permission, 사용자 판단 coverage와 비교하는 쓰기 전 범위 확인을 실행합니다. |
| [`harness.record_run`](#harnessrecord_run) | shaping, implementation, direct run과 최소 artifact/evidence ref를 기록합니다. |
| [`harness.close_task`](#harnessclose_task) | 닫기 준비 상태를 확인하고, 막힘이 허용할 때만 complete, cancel, supersede합니다. |

## MVP-1이 아닌 것

다음 surface는 owner 문서가 승격하기 전까지 later/profile-gated입니다.

- 별도 `harness.next`
- `harness.launch_verify`
- `harness.record_eval`
- `harness.record_manual_qa`
- sensitive-action approval을 `user_judgment`로 다루는 범위를 넘어선 committed Approval record lifecycle
- full Evidence Manifest, detached verification 또는 detached Eval system, full Manual QA matrix, reconcile, export/recover suite, broad operations, detailed diagnostic projections

## 공통 request 규칙

모든 method는 [`ToolEnvelope`](schema-core.md#tool-envelope)와 [`ToolResponseBase`](schema-core.md#common-response)를 사용합니다. State-changing tool은 non-null `idempotency_key`와 current `expected_state_version`을 요구합니다. Read-only tool은 같은 envelope를 tracing에 사용할 수 있고 `expected_state_version`을 `null`로 둘 수 있습니다.

Method가 tool-specific `task_id`와 `ToolEnvelope.task_id`를 모두 가지면 tool-specific `task_id`가 첫 primary Task 후보입니다. Core는 tool-specific `task_id`, envelope `task_id`, active Task resolution 순서로 primary Task를 찾습니다. Primary Task가 없으면 그 mutation은 `expected_state_version`과 `ToolResponseBase.state_version`에 대해 project-scoped mutation입니다.

MVP-1 request validator는 [Schema Core](schema-core.md#stage-specific-active-value-sets)의 active value set을 사용합니다. [Schema Later](schema-later.md)에 존재하는 later enum value나 extension branch는 그 자체로 MVP-1에서 유효해지지 않습니다.

Error code, MVP-1 status/error condition name, 사용자 표시 문구 pattern, primary error precedence, idempotency replay, stale-state behavior는 [Errors](errors.md)가 담당합니다. Guarantee level의 보안 의미는 [보안 참조: 정직한 guarantee display](../security.md#정직한-guarantee-display)가 담당합니다. 모든 state-changing tool에서 `dry_run=true`는 기준 권한이 아닙니다. Validation diagnostic 또는 would-change summary를 반환할 수 있지만 current record, `task_events` row, artifact, consumable Write Authorization, projection job, idempotency replay row를 만들지 않습니다.

<a id="harnessintake"></a>

## `harness.intake`

작업을 시작하거나, 분류하거나, 이어갈 때 이 method를 사용합니다.

Stage meaning: 내부 엔지니어링 점검에는 필요하지 않습니다. 내부 점검은 owner-valid setup path를 사용할 수 있습니다. MVP-1에서는 평소 말로 시작/이어가기 behavior가 active입니다. Full discovery, design-support routing, broad planning workflow는 명시적으로 승격되기 전까지 later material입니다.

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
  evidence_summary: EvidenceSummary | null
  evidence_refs: StateRecordRef[]
  blocker_refs: StateRecordRef[]
  projection_freshness:
    status: current | stale | failed | unknown
    stale_refs: StateRecordRef[]
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]
```

`status_card`는 current Core state와 ref에서 만든 짧은 읽기용 보기입니다. Compact하게 유지하고 source/freshness 정보를 보여줘야 합니다. 전체 schema, DDL, history, template, projection body, artifact body, log, future catalog를 넣으면 안 됩니다. Core 상태가 아니며 민감 동작 승인, 작업 수락, 잔여 위험 수용, 근거, 닫기 준비 상태, Write Authorization, close를 만들 수 없습니다.

`next_actions`가 MVP-1의 다음 안전한 행동 surface입니다. 사용자에게는 가장 작은 useful next action이나 unblocker를 쉬운 말로 보여 주고, exact enum value는 secondary detail로 둡니다.

`evidence_summary`는 Core가 소유한 compact MVP-1 evidence summary입니다. `evidence_refs`는 active minimal evidence coverage ref를 담습니다. 보통 `StateRecordRef.record_kind=evidence_summary`를 사용하며, nested schema가 허용하는 곳에서는 artifact ref도 함께 둡니다. 이 field들은 full Evidence Manifest table이나 report가 아니며, verification, 수동 QA, 작업 수락, 잔여 위험 수용, close를 대신하지 않습니다.

Status가 Core에 닿지 못하거나, stale state를 보고하거나, unsupported surface를 이름 붙이거나, 범위 밖 작업, 필요한 사용자 판단, 부족한 근거, 닫기 막힘, 남은 잔여 위험 같은 blocker를 보여줄 때는 [Errors: MVP-1 guarantee와 상태/error taxonomy](errors.md#mvp-1-guarantee-and-status-taxonomy)의 canonical condition 동작을 사용합니다.

MVP-1 active `NextActionSummary.action_kind` values:

```text
ask_user | prepare_write | implement | request_acceptance | close_task | idle
```

Verification, Eval, Manual QA, reconcile, export/recover, operations next-action kind는 later/profile-gated입니다.

Status는 read-only입니다. State를 만들거나, 제품 파일 쓰기를 compatible하게 만들거나, Write Authorization을 만들거나, gate를 충족하거나, 근거를 만들거나, 민감 동작 승인을 만들거나, work acceptance를 기록하거나, residual risk를 받아들이거나, 닫기 준비 상태를 만들거나, projection repair를 enqueue하거나, Task를 close하면 안 됩니다.

<a id="harnessprepare_write"></a>

## `harness.prepare_write`

에이전트가 제품 파일을 쓰기 전에, 그 정확한 쓰기가 현재 Core state에 맞는지 확인할 때 이 method를 사용합니다. 결과는 compatible internal single-use Write Authorization record이거나 structured blocker입니다. 이것은 하네스 수준의 협력형 확인이지 OS 권한, sandboxing, 사전 차단이 아닙니다.

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

`decision=allowed`이고 `dry_run=false`이면 `write_authorization_ref`와 active `write_authorization`이 있어야 합니다. `dry_run=true`에서는 `authorization_effect=would_create`를 반환할 수 있지만 authorization을 만들지 않습니다. 여기서 `allowed`는 이 API path에서 현재 하네스 기록과 맞는다는 뜻이지 OS 권한이나 실행 전 차단이 아니며, durable Write Authorization lifecycle status도 아닙니다. `decision`이 `allowed`가 아닌 response는 Write Authorization을 포함하면 안 됩니다.

`approval_request_candidate`와 `user_judgment_candidate`는 non-mutating candidate payload입니다. 이것만으로 user judgment, Approval record, Write Authorization, projection을 만들지 않습니다.

Committed `dry_run=false` `decision=allowed` response를 exact idempotent replay하면 original response와 original `write_authorization_ref`를 `authorization_effect=returned`로 반환합니다. 두 번째 Write Authorization을 만들거나 event를 다시 append하면 안 됩니다. 같은 key를 다른 canonical request hash로 replay하면 `STATE_CONFLICT`를 반환합니다.

Public transition summary: `harness.prepare_write`는 envelope를 검증하고, idempotency를 검증하며 exact committed replay가 있으면 새 side effect 전에 반환합니다. Shared request rule에 따라 primary Task를 resolve합니다. Primary Task가 있으면 `tasks.state_version`, 없으면 `project_state.state_version`에 대해 `expected_state_version`을 확인한 뒤 active Change Unit을 resolve합니다. 그다음 intended operation/path/tool/command/network/secret/sensitive-category compatibility, baseline freshness, sensitive-action permission, user judgment와 decision-gate coverage, Autonomy Boundary, surface capability, active design-policy precondition을 확인한 뒤 `decision`을 계산합니다. `dry_run=false`이고 `decision=allowed`일 때만 `write_authorizations.status=active`를 만들며, committed `dry_run=false` result는 반환 전에 task event를 append합니다.

<a id="harnessrecord_run"></a>

## `harness.record_run`

Shaping update, direct result, implementation run 뒤에 이 method를 사용합니다. Implementation 또는 direct product-write Run은 `harness.prepare_write`가 반환한 compatible internal Write Authorization record를 소비합니다.

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
  evidence_ref: StateRecordRef | null
  evidence_summary: EvidenceSummary | null
  run_summary_ref: StateRecordRef | null
  direct_result_ref: StateRecordRef | null
  registered_artifacts: ArtifactRef[]
  next_action: string
```

`payload` branch는 `kind`와 일치해야 합니다. MVP-1은 `shaping_update`, `implementation`, `direct`를 허용합니다. `verification_input`은 later-profile only입니다.

`evidence_ref`는 active minimal evidence coverage record를 가리킵니다. 보통 `StateRecordRef.record_kind=evidence_summary`를 사용합니다. `evidence_summary`는 Run이 기록된 뒤의 current Core-owned compact summary를 반환합니다. 같은 operation이 반환하는 durable byte는 `registered_artifacts`에 나타납니다.

Committed `record_run` response를 exact idempotent replay하면 current freshness check, authorization consumption, Run creation, artifact registration, blocker/gate update, projection enqueue, event append 전에 original response를 반환합니다. Write Authorization을 두 번 소비하면 안 됩니다.

Public transition summary: `harness.record_run`은 envelope를 검증하고, idempotency replay를 확인하며 exact committed replay가 있으면 새 side effect 전에 반환합니다. Shared request rule에 따라 primary Task를 resolve합니다. Primary Task가 있으면 `tasks.state_version`, 없으면 `project_state.state_version`에 대해 `expected_state_version`을 확인합니다. 그다음 `kind`를 확인하고 product write를 감지합니다. Product write에는 compatible active Write Authorization을 요구하고, observed changed paths, commands, tools, secret access를 검증합니다. Compatible하면 authorization을 소비하고, Run record를 만들고, `ArtifactRef`를 등록하거나 연결하고, evidence summary와 blockers/gates를 업데이트하고, task event를 append한 뒤 response를 반환합니다.

Core가 write-capable run을 commit 전에 거절하면 `run_id`는 `null`이고 artifact는 등록되지 않으며 response는 Run이 존재한다고 암시하면 안 됩니다. Core는 invalid authorization을 consumed로 표시하면 안 됩니다. Violation/audit Run은 제품 쓰기가 이미 관찰된 뒤 Core가 의도적으로 기록할 때만 생길 수 있습니다. Attempted authorization ref는 validator finding, violation payload, event payload에만 나타날 수 있으며 evidence, QA, verification, work acceptance, close readiness를 충족하지 않습니다.

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

`judgment_type`은 저장된 `UserJudgment`와 일치해야 합니다. `go ahead`, `looks good`, `진행해` 같은 free-form note는 pending judgment가 그 judgment type을 명시적으로 묻고 answer가 allowed value와 맞을 때만 approval, acceptance, risk acceptance, waiver, 쓰기 전 범위 확인 호환성과 연결될 수 있습니다.

MVP-1에서 `accepted_risk_refs`는 해당 close path에서 risk가 보였고 받아들여졌음을 보여주는 `user_judgment`와 `blocker` ref를 포함합니다. Rich `residual_risk` ref는 later/profile-promoted입니다. 별도 accepted-risk record kind는 없습니다.

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
  evidence_summary: EvidenceSummary | null
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

MVP-1 close는 core close state, blocker, residual-risk visibility, required work-acceptance state, artifact availability, Core가 소유한 `evidence_summary`를 사용합니다. Close readiness는 current record에서 파생됩니다. Verification, Manual QA, projection/report, operations ref는 해당 profile이 enabled일 때만 active입니다.

`intent=complete`에서 closed response가 되려면 Task state가 close intent와 호환되고, close와 관련해 unresolved active Run이 없고, required user judgment가 unresolved 또는 blocked 상태가 아니며, evidence가 required이면 `evidence_summary.status=sufficient`여야 합니다. Acceptance가 required이면 작업 수락이 기록되어야 합니다. Close-relevant residual risk는 visible해야 하며, `completed_with_risk_accepted`에는 명시적인 residual-risk acceptance가 필요합니다. Stale 또는 blocked Write Authorization fact는 그 영향이 닿는 current Run, scope, artifact, evidence, blocker record를 통해서만 close에 영향을 줍니다. Projection freshness는 display freshness이지 canonical close state가 아닙니다. Caller는 stale projection prose에서 close하면 안 됩니다.

`CloseTaskRequest`는 accepted-risk refs를 싣지 않습니다. `completed_with_risk_accepted`에서는 Core가 close-relevant risk를 보여 주는 blocker와 residual-risk acceptance `user_judgment`의 accepted state를 읽고, 그 상태가 없으면 block합니다. Rich Residual Risk record는 해당 later profile이 active일 때만 필요합니다.

Successful close는 Task를 terminal state로 옮깁니다. Failed close는 Task를 open 상태로 남기고 structured blockers를 반환합니다. 같은 idempotency key의 repeated successful close는 같은 terminal response를 반환하고, conflicting close intent는 `STATE_CONFLICT`를 반환합니다.
