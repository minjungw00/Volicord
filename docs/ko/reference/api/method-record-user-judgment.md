<a id="harnessrecord_user_judgment"></a>

# `harness.record_user_judgment` 참조

## 담당하는 것

이 문서는 기준 범위의 `harness.record_user_judgment` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 기존 대기 중인 `UserJudgment` 하나에 대한 사용자의 답을 기록하는 동작
- 그 대기 중인 사용자 소유 판단을 해결, 거절, 연기, 차단, 또는 다른 지원 상태로 표시하는 메서드별 경계
- record-user-judgment 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, `dry_run`, 거절 응답 스키마 본문
- `UserJudgment`, `RecordUserJudgmentPayload`, `SensitiveActionScope`, `AcceptedRiskInput`, 값 집합, 상태 필드 정의
- Core의 사용자 소유 판단 의미, 최종 수락 의미, 잔여 위험 의미, 민감 동작 승인 의미, `Write Authorization` 의미
- 저장 기록 레이아웃, 정확한 저장 효과, 공개 오류 코드 의미, 공개 오류 우선순위, 공통 응답 분기 처리 경로

## 목적

`harness.record_user_judgment`는 기존 대기 중인 `UserJudgment` 하나에 대한 사용자의 답을 기록합니다.

이 메서드는 사용자의 답에 따라 지정된 대기 판단을 갱신합니다. 답변을 관련 없는 승인, 현재 적용 범위 확장, 최종 수락, 잔여 위험 수락, 민감 동작 승인, `Write Authorization`으로 넓히지 않습니다.

## 필수 입력

- 유효한 `ToolEnvelope`. 커밋되는 `dry_run`이 아닌 요청에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- 기존 대기 판단을 가리키는 `user_judgment_id`.
- 일치하는 `judgment_kind`.
- `selected_option_id`, `answer`, `note`, `accepted_risks`.
- 대기 중인 `judgment_kind`에 맞는 판단별 요청 본문 분기만 담은 `answer`.

`selected_option_id`와 `note`는 요청 수준에 남습니다. `RecordUserJudgmentPayload`는 판단별 답변 분기 안에서 이 필드를 반복하면 안 됩니다.

## 접근 요구사항

이 메서드에는 아래 조건이 필요합니다.

- `VerifiedSurfaceContext.access_class=core_mutation`
- `verified=true`
- 요청이 선택한 같은 프로젝트와 호환되는 `Task`에 속한, 지정된 대기 판단

로컬 접근 실패, 읽을 수 없는 판단 식별자, 부족한 로컬 역량은 커밋 전에 거절됩니다.

## 상태 버전 동작

커밋된 `dry_run`이 아닌 결과:

- `project_state.state_version`을 정확히 한 번 올립니다.
- 지정된 `user_judgments` 행을 갱신합니다.
- 저장 효과 담당 문서가 허용하는 경우에만 종속 차단 사유 또는 요약 상태를 갱신할 수 있습니다.

비주장:

- `dry_run`과 거절은 판단 해결, 차단 사유 갱신, 이벤트, 재실행 행, 상태 버전 증가를 만들지 않습니다.
- 기록된 `scope_decision`은 현재 적용 범위나 현재 적용 Change Unit 기록을 조용히 바꾸지 않습니다. 그 기록은 여전히 `harness.update_scope` 같은 범위 담당 문서가 정의한 전이가 필요합니다.

## 성공 결과

아래 값을 담은 `RecordUserJudgmentResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `user_judgment_ref`
- 갱신된 `user_judgment`
- `updated_refs`
- 현재 `state`
- `next_actions`

사용자의 답이 그렇거나 초점이 맞는 판단의 호환 결과가 그렇다면 이 메서드는 지정된 판단을 `resolved`, `rejected`, `deferred`, `blocked` 또는 다른 지원 판단 상태로 커밋할 수 있습니다.

결과는 포함된 차단 사유와 판단에 의존하는 요약만 갱신합니다. 관련 없는 승인, 증거, 범위 갱신, `Write Authorization`, 닫기 상태, 기록된 판단 자체를 넘어서는 잔여 위험 수락을 만들지 않습니다.

## 차단 결과

이 메서드에는 별도의 커밋된 차단 응답 분기가 없습니다.

커밋된 `user_judgment.status=blocked`는 기록된 판단 결과이지, `ToolRejectedResponse`도 아니고 `PrepareWriteResult` 방식의 차단 결정도 아닙니다.

## 거절 결과

아래와 같은 커밋 전 실패에는 `ToolRejectedResponse`를 반환합니다.

- 오래된 `expected_state_version`
- 알 수 없거나 `pending`이 아닌 판단
- `judgment_kind` 불일치
- 유효하지 않은 선택지
- 유효하지 않은 답변 요청 본문
- 만료된 대기 판단
- 대기 판단과 호환되지 않는 답변
- 로컬 접근 실패
- 검증기 실패

공개 오류 코드 의미, 우선순위, 거절 응답 처리 경로는 아래 오류 담당 문서가 담당합니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 미리보기:

- `ToolDryRunResponse`를 반환합니다.
- 판단을 해결하지 않습니다.
- 차단 사유, 이벤트, 재실행 행, 상태 버전을 갱신하지 않습니다.

## 저장 효과

커밋 시 판단 해결과 그에 따른 차단 사유 또는 요약 상태를 지속할 수 있습니다. 정확한 저장 효과는 아래 저장 담당 문서가 담당합니다.

## 최소 유효 요청

```yaml
method: harness.record_user_judgment
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: user
    surface_id: surface_local
    request_id: req_judgment_answer_001
    idempotency_key: idem_judgment_answer_001
    expected_state_version: 22
    dry_run: false
    locale: ko-KR
  user_judgment_id: uj_001
  judgment_kind: product_decision
  selected_option_id: accept
  answer:
    product_decision:
      judgment:
        decision: accepted
        rationale: "인보이스 다운로드 확인 문구는 이 Task에 충분히 명확합니다."
    technical_decision: null
    scope_decision: null
    sensitive_action_scope: null
    final_acceptance: null
    residual_risk_acceptance: null
    cancellation: null
  note: null
  accepted_risks: []
```

## 대표 응답

결과 분기(`RecordUserJudgmentResult`, 커밋됨):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 23
  events:
    - event_id: evt_1006
      event_kind: user_judgment_recorded
user_judgment_ref:
  record_kind: user_judgment
  record_id: uj_001
  project_id: proj_123
  task_id: task_456
  state_version: 23
user_judgment:
  judgment_id: uj_001
  project_id: proj_123
  task_id: task_456
  change_unit_id: cu_001
  judgment_kind: product_decision
  status: resolved
  presentation: short
  question: "이 Task에서 인보이스 다운로드 확인 문구가 충분합니까?"
  options:
    - option_id: accept
      label: "충분함"
      description: "문구가 충분하다는 사용자 소유 제품 판단을 기록합니다."
      consequence: "닫기 준비 상태가 이 제품 판단을 해결된 것으로 평가할 수 있습니다."
      is_default: true
    - option_id: revise
      label: "수정 필요"
      description: "확인 문구를 더 수정해야 하므로 Task를 열어 둡니다."
      consequence: "이 제품 판단이 남아 있어 닫기가 계속 차단됩니다."
      is_default: false
  context:
    summary: "확인 문구는 인보이스 PDF 다운로드 전에 표시되며, 사용자가 청구 문서를 다운로드하려 한다는 점을 알립니다."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "인보이스 PDF 다운로드 확인은 범위 안에 있고, 인보이스 생성 방식은 범위 밖입니다."
  affected_refs:
    - record_kind: task
      record_id: task_456
      project_id: proj_123
      task_id: task_456
      state_version: 21
  required_for: close
  resolution:
    selected_option_id: accept
    answer:
      product_decision:
        judgment:
          decision: accepted
          rationale: "인보이스 다운로드 확인 문구는 이 Task에 충분히 명확합니다."
    note: null
    accepted_risks: []
    resolved_by_actor_kind: user
  expires_at: null
  created_at: "<example-created-at>"
  resolved_at: "<example-resolved-at>"
updated_refs:
  - record_kind: user_judgment
    record_id: uj_001
    project_id: proj_123
    task_id: task_456
    state_version: 23
state:
  project_id: proj_123
  state_version: 23
next_actions:
  - action_kind: close_task
    owner_method: harness.close_task
    label: "사용자 소유 제품 판단을 기록한 뒤 닫기 준비 상태를 평가한다."
    blocking_question: null
    required_refs:
      - record_kind: user_judgment
        record_id: uj_001
        project_id: proj_123
        task_id: task_456
        state_version: 23
```

## 담당 문서 링크

- 요청 래퍼, 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `UserJudgment`, `RecordUserJudgmentPayload`, `SensitiveActionScope`, `AcceptedRiskInput`: [API 판단 스키마](schema-judgment.md).
- 상태 참조와 요약: [API 상태 스키마](schema-state.md).
- 판단 값과 지원되는 메서드 내부 값: [API 값 집합](schema-value-sets.md).
- 사용자 소유 판단, 최종 수락, 잔여 위험 수락, 비대체 규칙: [Core 모델](../core-model.md).
- 정확한 저장 효과: [저장 효과](../storage-effects.md#harnessrecord_user_judgment).
- 공개 오류, 우선순위, 거절 응답 처리 경로: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md).
- 대기 중인 판단 요청 생성: [`harness.request_user_judgment`](method-request-user-judgment.md).
