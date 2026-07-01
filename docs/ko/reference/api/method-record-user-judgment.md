<a id="volicordrecord_user_judgment"></a>

# `volicord.record_user_judgment` 참조

## 담당하는 것

이 문서는 기준 범위의 `volicord.record_user_judgment` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 기존 대기 중인 `UserJudgment` 하나에 대한 사용자의 답을 기록하는 동작
- 그 대기 중인 사용자 소유 판단을 해결하고 유효하지 않은 답변 시도를 거절하는 메서드별 경계
- 수락된 결정과 수락된 잔여 위험에 대해 메서드가 선택하는 프로젝트 연속성 기록 생성
- 사용자 소유 판단 기록 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, `dry_run`, 거절 응답 스키마 본문
- `UserJudgment`, `JudgmentRationale`, `RecordUserJudgmentPayload`, `SensitiveActionScope`, `AcceptedRiskInput`, 값 집합, 상태 필드 정의
- `ProjectContinuityRecord` 또는 `ProjectContinuitySummary` 필드 정의
- Core의 사용자 소유 판단 의미, 최종 수락 의미, 잔여 위험 의미, 민감 동작 승인 의미, `Write Check` 의미
- 저장 기록 레이아웃, 정확한 저장 효과, 공개 오류 코드 의미, 공개 오류 우선순위, 공통 응답 분기 처리 경로

## 목적

`volicord.record_user_judgment`는 기존 대기 중인 `UserJudgment` 하나에 대한 사용자의 답을 기록합니다.

이 메서드는 사용자의 답에 따라 지정된 대기 판단을 갱신합니다. 답변을 관련 없는 승인, 현재 적용 범위 확장, 최종 수락, 잔여 위험 수락, 민감 동작 승인, `Write Check`으로 넓히지 않습니다.

답변을 기록하기 전에 Core는 대기 판단의 `JudgmentBasis`를 현재 상태와 비교합니다. 오래됨, 대체됨, 비호환, 저장 근거가 유효하지 않은 판단에는 성공적으로 답할 수 없습니다.

수락된 제품 결정, 기술 결정, 범위 결정은 `kind=decision`인 오래 유지되는 프로젝트 연속성 기록을 만들 수 있습니다. 수락된 잔여 위험 판단은 정확히 수락된 현재 잔여 위험에 대해 `kind=accepted_risk` 프로젝트 연속성 기록을 만들 수 있습니다. 이런 연속성 기록은 맥락을 보존하지만 관련 없는 승인이나 미래 닫기 권한을 만들지 않습니다.

## 필수 입력

- 유효한 `ToolEnvelope`. 커밋되는 `dry_run`이 아닌 요청에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- 기존 대기 판단을 가리키는 `user_judgment_id`.
- 일치하는 `judgment_kind`.
- `selected_option_id`, `answer`, `rationale`, `note`, `accepted_risks`.
- 대기 중인 `judgment_kind`에 맞는 판단별 요청 본문 분기만 담은 `answer`.

`selected_option_id`, `rationale`, `note`는 요청 수준에 남습니다. `RecordUserJudgmentPayload`는 판단별 답변 분기 안에서 이 필드를 반복하면 안 됩니다.

선택된 선택지의 저장된 `machine_action`과 `resolution_outcome`이 기준입니다. 답변 본문에 결과, 결정, 수락 필드가 있으면 선택된 선택지와 일치해야 합니다. 자유 형식 답변 텍스트, 라벨, 메모는 권한을 부여할 수 없습니다.

수락된 제품 판단, 기술 판단, 범위 판단, 최종 수락, 취소, 민감 승인, 잔여 위험 수락에는 요약보다 구조화된 근거가 더 필요합니다. 거절되거나 연기된 답변은 요약으로 충분할 때 간결한 근거를 사용할 수 있습니다. 근거는 설명 메타데이터이며, 비호환, 오래됨, 에이전트가 기록한 답변, 잘못된 선택지의 답변을 유효하게 만들 수 없습니다.

## 요청 스키마

이 메서드는 아래 최상위 `params` 요청 형태를 담당합니다. `envelope`는 [API 코어 스키마](schema-core.md#tool-envelope)의 공통 `ToolEnvelope`이며, 이 블록은 `ToolEnvelope` 필드를 다시 정의하지 않습니다.

이 메서드 소유 요청 블록에 표시된 모든 필드는 필드 참고가 명시적으로 선택 필드라고 표시하지 않는 한 `params`의 필수 멤버입니다. `T | null`은 멤버가 반드시 있어야 하며 JSON `null`을 담을 수 있다는 뜻입니다.

```yaml
RecordUserJudgmentRequest:
  envelope: ToolEnvelope
  user_judgment_id: string
  judgment_kind: string
  selected_option_id: string
  answer: RecordUserJudgmentPayload
  rationale: JudgmentRationale
  note: string | null
  accepted_risks: AcceptedRiskInput[]
```

중첩 형태 담당 문서:
- `answer`는 `RecordUserJudgmentPayload`를 사용합니다. `SensitiveActionScope`는 그 요청 본문 분기 안에서만 나타날 수 있으며 [API 판단 스키마](schema-judgment.md#resolution-and-answer-payload)가 담당합니다.
- `rationale`은 `JudgmentRationale`을 사용합니다. 중첩 형태는 [API 판단 스키마](schema-judgment.md#resolution-and-answer-payload)가 담당합니다.
- `accepted_risks`는 `AcceptedRiskInput[]`을 사용합니다. 중첩 형태는 [API 판단 스키마](schema-judgment.md#acceptedriskinput)가 담당합니다.
- `judgment_kind` 값은 [API 값 집합의 판단 값](schema-value-sets.md#judgment-values)이 담당합니다.

## 접근 요구사항

이 메서드에는 아래 조건이 필요합니다.

- `actor_source=local_user`인 서버 파생 호출 맥락
- `operation_category=user_only`인 서버 파생 호출 맥락
- 요청이 선택한 같은 프로젝트와 호환되는 `Task`에 속한, 지정된 대기 판단

로컬 User Channel 실패, 읽을 수 없는 판단 식별자, 부족한 호출 출처는 커밋 전에 거절됩니다.

권한을 지니는 해결에는 `cli_direct_user_channel`, `mcp_elicitation_user_channel`,
`user_prompt_submit_hook`, `local_user_local_web` 같은 호환 User Channel
`verification_basis`도 필요합니다. Agent Connection 호출, 생성된 지침 파일, 생성된
Markdown, 상태 요약, projection은 사용자 답변을 주장하거나 전달하더라도 사용자 권한을
만족할 수 없습니다.

## 상태 버전 동작

커밋된 `dry_run`이 아닌 결과:

- `project_state.state_version`을 정확히 한 번 올립니다.
- 지정된 `user_judgments` 행을 갱신합니다.
- `scope_revision`이나 `close_basis_revision`을 증가시키지 않습니다.
- 이 메서드가 선택할 때 수락된 결정이나 수락된 잔여 위험에 대한 `project_continuity_records`를 만들 수 있습니다.
- 저장 효과 담당 문서가 허용하는 경우에만 종속 차단 사유 또는 요약 상태를 갱신할 수 있습니다.

비주장:

- `dry_run`과 거절은 판단 해결, 프로젝트 연속성 기록, 차단 사유 갱신, 이벤트, 재실행 행, 상태 버전 증가를 만들지 않습니다.
- 기록된 `scope_decision`은 현재 적용 범위나 현재 적용 Change Unit 기록을 조용히 바꾸지 않습니다. 그 기록은 여전히 `volicord.update_scope` 같은 범위 담당 문서가 정의한 전이가 필요합니다.

호환성 요구사항:

- 최종 수락은 판단 근거에 캡처된 현재 `Task`, Change Unit, `scope_revision`, `close_basis_revision`, 기준선, 결과 참조와 일치해야 합니다.
- 잔여 위험 수락은 `AcceptedRiskInput`에 정확한 현재 `risk_id` 값을 포함해야 하며 현재 `close_basis_revision`과 일치해야 합니다.
- 민감 승인은 현재 `scope_revision`, Change Unit, 동작, 정규화된 경로, 민감 범주, 기준선과 일치해야 합니다.
- 나중의 범위 갱신에 쓰이는 범위 결정 권한은 `judgment_kind=scope_decision`, `status=resolved`, `machine_action=accept`, `resolution_outcome=accepted`, 현재 근거, scope update를 포함하는 `required_for`, `resolved_by_actor_source=local_user`, 호환되는 User Channel 출처, 호환되는 Task, Change Unit, `scope_revision`, 영향받는 참조가 필요합니다.
- 권한을 지니는 판단은 권한 요구사항을 만족하려면 `resolved_by_actor_source=local_user`, 호환되는 User Channel 출처, `machine_action=accept`, `resolution_outcome=accepted`가 필요합니다.
- 거절되거나 연기된 권한 판단은 결정 기록으로 남지만 현재 전이를 허가할 수 없습니다. 오래됨, 대체됨, 만료됨, 유효하지 않은 근거, 출처 누락, 해결 정보 누락, 에이전트가 기록한 권한 판단은 현재 전이를 허가할 수 없습니다.
- 범위 변경이나 실행 기록 변경은 이력 판단을 삭제하지 않습니다. 다만 호환되지 않는 판단은 현재 닫기, 쓰기, 범위 결정, 민감 승인 요구사항에 사용할 수 없게 됩니다.

## 성공 결과

아래 값을 담은 `RecordUserJudgmentResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `user_judgment_ref`
- 갱신된 `user_judgment`
- `updated_refs`
- 현재 `state`
- `next_actions`

답변이 성공적으로 기록되면 이 메서드는 지정된 판단을 `status=resolved`로 커밋합니다. 기록된 `machine_action`과 `resolution_outcome`은 선택된 선택지에서 복사되며 선택지의 동작/결과 매핑과 일치해야 합니다. 기록된 `rationale`은 설명 메타데이터로 `user_judgment.resolution` 안에 반환됩니다.

결과는 포함된 차단 사유, 판단에 의존하는 요약, 메서드가 선택한 프로젝트 연속성 기록만 갱신합니다. `accepted`이고 호환되는 권한 판단 자체를 넘어 관련 없는 승인, 증거, 범위 갱신, `Write Check`, 닫기 상태, 최종 수락, 잔여 위험 수락, 민감 승인, 취소 권한을 만들지 않습니다.

## 메서드 결과 필드

`RecordUserJudgmentResult`는 커밋된 사용자 판단 답변을 위한 메서드별 결과 분기입니다. 이 결과는 `base: ToolResultBase`와 아래 메서드 담당 최상위 필드를 담습니다.

| 필드 | 결과 필드 의미 |
|---|---|
| `base` | 공통 결과 메타데이터입니다. `events`를 포함한 `ToolResultBase` 형태는 [API 코어 스키마](schema-core.md#common-response)가 담당합니다. 커밋된 `RecordUserJudgmentResult` 분기는 `base.response_kind=result`와 `base.effect_kind=core_committed`를 사용합니다. `base.events[].event_kind`가 있으면 불투명한 예시 분류 문자열입니다. |
| `user_judgment_ref` | 답변이 기록된 뒤 지정된 `UserJudgment`의 `StateRecordRef`입니다. |
| `user_judgment` | 기록된 답변이 초점이 맞는 판단을 해결할 때 `resolution`이 채워진 갱신된 `UserJudgment`입니다. 중첩 형태는 [API 판단 스키마](schema-judgment.md#userjudgment)가 담당합니다. |
| `updated_refs` | 이 판단 답변 기록으로 갱신되거나 생성된 기록의 `StateRecordRef[]`입니다. 이 메서드가 프로젝트 연속성 기록을 만들면 그 참조도 포함합니다. |
| `state` | 판단 답변이 기록된 뒤의 현재 `StateSummary`입니다. 중첩 상태 필드는 [API 상태 스키마](schema-state.md)가 담당합니다. |
| `next_actions` | 다음에 안전하게 수행할 API 단계를 설명하는 `NextActionSummary[]`입니다. 기준 형태는 [API 상태 스키마](schema-state.md#current-position-display-shapes)가 담당합니다. |

`RecordUserJudgmentPayload`는 `user_judgment.resolution.answer` 안에 남으며, [API 판단 스키마](schema-judgment.md#resolution-and-answer-payload)가 담당하는 형태를 사용합니다. `JudgmentRationale`은 `user_judgment.resolution.rationale` 안에 남으며 권한을 바꾸지 않습니다. `next_actions` 항목은 `action_kind`, `owner_method`, `label`, `blocking_question`, `required_refs`를 사용합니다. 오래된 `action` 또는 `reason` 필드는 `NextActionSummary`의 일부가 아닙니다.

## 차단 결과

이 메서드에는 별도의 커밋된 차단 응답 분기가 없습니다.

`blocked`는 커밋되는 `JudgmentResolutionOutcome`이 아닙니다. 답변 본문이 차단된 판단 결과를 명시적으로 주장하면 커밋 전에 거절됩니다.

## 거절 결과

아래와 같은 커밋 전 실패에는 `ToolRejectedResponse`를 반환합니다.

- 오래된 `expected_state_version`
- 알 수 없거나 `pending`이 아닌 판단
- `judgment_kind` 불일치
- 유효하지 않은 선택지
- 유효하지 않은 답변 요청 본문
- 만료된 대기 판단
- 오래됨, 대체됨, 비호환, 유효하지 않은 저장 판단 근거
- 대기 판단과 호환되지 않는 답변
- 누락되었거나 현재와 일치하지 않는 잔여 위험 `risk_id`
- 행위자 출처 또는 작업 범주 불일치
- 검증기 실패

공개 오류 코드 의미, 우선순위, 거절 응답 처리 경로는 아래 오류 담당 문서가 담당합니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 미리보기:

- `ToolDryRunResponse`를 반환합니다.
- 판단을 해결하지 않습니다.
- 프로젝트 연속성 기록을 만들지 않습니다.
- 차단 사유, 이벤트, 재실행 행, 상태 버전을 갱신하지 않습니다.

## 저장 효과

커밋 시 판단 해결, 메서드가 선택한 프로젝트 연속성 기록, 그에 따른 차단 사유 또는 요약 상태를 지속할 수 있습니다. 정확한 저장 효과는 아래 저장 담당 문서가 담당합니다.

## 최소 유효 요청

메서드 안의 전제: `uj_empty_001`은 `proj_empty_001`의 `task_empty_001`과 `cu_empty_001`에 속한 기존 대기 `product_decision`입니다. 현재 프로젝트 `state_version`은 `62`이고, `keep`은 그 선택지 식별자 중 하나입니다.

```yaml
method: volicord.record_user_judgment
params:
  envelope:
    project_id: proj_empty_001
    task_id: task_empty_001
    request_id: req_empty_answer_001
    idempotency_key: idem_empty_answer_001
    expected_state_version: 62
    dry_run: false
    locale: en-US
  user_judgment_id: uj_empty_001
  judgment_kind: product_decision
  selected_option_id: keep
  answer:
    product_decision:
      judgment:
        decision: accepted
    technical_decision: null
    scope_decision: null
    sensitive_action_scope: null
    final_acceptance: null
    residual_risk_acceptance: null
    cancellation: null
  rationale:
    summary: "사용자가 이 Task의 빈 상태 일러스트를 유지하기로 했습니다."
    selected_reason: "일러스트가 문구를 바꾸지 않으면서 의도한 빈 상태 톤을 보조합니다."
    considered_alternatives:
      - "일러스트를 텍스트 전용 상태로 교체합니다."
    rejected_alternatives:
      - "모달형 빈 상태를 사용합니다."
    assumptions:
      - "이 판단은 빈 상태 일러스트만 다룹니다."
    tradeoffs:
      - "일러스트를 유지하면 시각적 연속성을 보존하지만 문구는 그대로 둡니다."
    uncertainties:
      - "향후 사용성 피드백은 여전히 텍스트 전용 상태를 선호할 수 있습니다."
    review_triggers:
      - "접근성 증거나 제품 톤 지침이 바뀌면 검토합니다."
    related_refs: []
    artifact_refs: []
  note: null
  accepted_risks: []
```

## 대표 응답

축약된 결과 분기(`RecordUserJudgmentResult`, 커밋됨):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 63
  events:
    - event_id: evt_empty_001
      event_kind: user_judgment_recorded
user_judgment_ref:
  record_kind: user_judgment
  record_id: uj_empty_001
  project_id: proj_empty_001
  task_id: task_empty_001
  state_version: 63
user_judgment:
  judgment_id: uj_empty_001
  project_id: proj_empty_001
  task_id: task_empty_001
  change_unit_id: cu_empty_001
  judgment_kind: product_decision
  status: resolved
  presentation: short
  question: "빈 상태 일러스트를 유지할까요?"
  options:
    - option_id: keep
      label: "일러스트 유지"
      description: "일러스트를 유지한다는 사용자 소유 제품 결정을 기록합니다."
      consequence: "선택되면 Core는 일러스트 유지 제품 결정을 기록합니다."
      machine_action: accept
      resolution_outcome: accepted
      is_default: true
    - option_id: replace
      label: "일러스트 교체"
      description: "일러스트를 교체한다는 사용자 소유 제품 결정을 기록합니다."
      consequence: "선택되면 Core는 일러스트 교체 제품 결정을 기록합니다."
      machine_action: accept
      resolution_outcome: accepted
      is_default: false
  context:
    summary: "빈 상태 화면에 제안된 일러스트가 있으며 사용자 소유 제품 결정이 필요합니다."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "이 판단은 빈 상태 일러스트 선택만 다룹니다."
  affected_refs:
    - record_kind: task
      record_id: task_empty_001
      project_id: proj_empty_001
      task_id: task_empty_001
      state_version: 62
  basis:
    task_id: task_empty_001
    change_unit_id: cu_empty_001
    scope_revision: 1
    close_basis_revision: null
    baseline_ref: baseline_empty_001
    result_refs: []
    residual_risk_ids: []
    sensitive_action_scope: null
    created_at_state_version: 62
    compatibility_status: current
  required_for:
    - close_complete
  resolution:
    selected_option_id: keep
    machine_action: accept
    resolution_outcome: accepted
    answer:
      product_decision:
        judgment:
          decision: accepted
      technical_decision: null
      scope_decision: null
      sensitive_action_scope: null
      final_acceptance: null
      residual_risk_acceptance: null
      cancellation: null
    rationale:
      summary: "사용자가 이 Task의 빈 상태 일러스트를 유지하기로 했습니다."
      selected_reason: "일러스트가 문구를 바꾸지 않으면서 의도한 빈 상태 톤을 보조합니다."
      considered_alternatives:
        - "일러스트를 텍스트 전용 상태로 교체합니다."
      rejected_alternatives:
        - "모달형 빈 상태를 사용합니다."
      assumptions:
        - "이 판단은 빈 상태 일러스트만 다룹니다."
      tradeoffs:
        - "일러스트를 유지하면 시각적 연속성을 보존하지만 문구는 그대로 둡니다."
      uncertainties:
        - "향후 사용성 피드백은 여전히 텍스트 전용 상태를 선호할 수 있습니다."
      review_triggers:
        - "접근성 증거나 제품 톤 지침이 바뀌면 검토합니다."
      related_refs: []
      artifact_refs: []
    note: null
    accepted_risks: []
    resolved_by_actor_source: local_user
    resolved_verification_basis: cli_direct_user_channel
    resolved_assurance_level: local_user_channel
  expires_at: null
  created_at: "<example-created-at>"
  resolved_at: "<example-resolved-at>"
updated_refs:
  - record_kind: user_judgment
    record_id: uj_empty_001
    project_id: proj_empty_001
    task_id: task_empty_001
    state_version: 63
  - record_kind: project_continuity_record
    record_id: continuity_empty_decision_001
    project_id: proj_empty_001
    task_id: task_empty_001
    state_version: 63
state:
  project_id: proj_empty_001
  state_version: 63
  task_ref:
    record_kind: task
    record_id: task_empty_001
    project_id: proj_empty_001
    task_id: task_empty_001
    state_version: 62
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Decide empty-state illustration."
  scope_summary: "Empty-state illustration decision."
  non_goals:
    - "Changing empty-state copy."
  acceptance_criteria:
    - "The empty-state illustration follows the user's product decision."
  autonomy_boundary: "Stay within empty-state illustration choice."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_empty_001
    project_id: proj_empty_001
    task_id: task_empty_001
    state_version: 62
  baseline_ref: baseline_empty_001
  shaping_readiness: null
  pending_user_judgment_refs: []
  blocker_refs: []
  write_check_summary: null
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display: null
next_actions:
  - action_kind: close_task
    owner_method: volicord.close_task
    label: "Evaluate close readiness after recording the user's product decision."
    blocking_question: null
    required_refs:
      - record_kind: user_judgment
        record_id: uj_empty_001
        project_id: proj_empty_001
        task_id: task_empty_001
        state_version: 63
```

## 담당 문서 링크

- 요청 래퍼, 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `UserJudgment`, `JudgmentRationale`, `RecordUserJudgmentPayload`, `SensitiveActionScope`, `AcceptedRiskInput`: [API 판단 스키마](schema-judgment.md).
- 상태 참조와 요약: [API 상태 스키마](schema-state.md).
- 판단 값과 지원되는 메서드 내부 값: [API 값 집합](schema-value-sets.md).
- 사용자 소유 판단, 최종 수락, 잔여 위험 수락, 비대체 규칙: [Core 모델](../core-model.md).
- 정확한 저장 효과: [저장 효과](../storage-effects.md#volicordrecord_user_judgment).
- 공개 오류, 우선순위, 거절 응답 처리 경로: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md).
- 대기 중인 판단 요청 생성: [`volicord.request_user_judgment`](method-request-user-judgment.md).
