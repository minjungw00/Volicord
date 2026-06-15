<a id="harnessrequest_user_judgment"></a>

# `harness.request_user_judgment` 참조

## 담당하는 것

이 문서는 기준 범위의 `harness.request_user_judgment` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 초점이 분명한 사용자 소유 판단 하나에 대해 대기 중인 `UserJudgment`를 만드는 동작
- request-user-judgment 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, `dry_run`, 거절 응답 스키마 본문
- `UserJudgment`, 선택지, 맥락, 답변 요청 본문, 값 집합, 상태 필드 정의
- Core의 사용자 소유 판단 의미, 최종 수락 의미, 잔여 위험 의미, 민감 동작 승인 의미, `Write Authorization` 의미
- 저장 기록 레이아웃, 정확한 저장 효과, 공개 오류 코드 의미, 공개 오류 우선순위, 공통 응답 분기 처리 경로

## 목적

`harness.request_user_judgment`는 초점이 분명한 사용자 소유 판단 하나에 대해 대기 중인 `UserJudgment`를 만듭니다. 이 메서드는 사용자에게 묻는 경로입니다. 에이전트는 사용자를 대신해 답하거나, 추론하거나, 판단 범위를 넓히거나, 결정해서는 안 됩니다.

대기 중인 판단은 결정을 요청하는 기록입니다. 결정 자체가 아니며, 증거를 만들거나, 현재 적용 범위를 바꾸거나, `Write Authorization`을 만들거나, `Task`를 닫지 않습니다.

## 필수 입력

- 유효한 `ToolEnvelope`. 커밋되는 `dry_run`이 아닌 요청에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `task_id`, `change_unit_id`, `judgment_kind`, `presentation`, `question`, `options`, `context`, `affected_refs`, `required_for`, `expires_at`.
- 서로 이해할 수 있는 `options`를 가진 초점이 분명한 `question`.
- 사용자가 숨은 대화 상태에 기대지 않고 정확한 사안을 판단할 수 있는 충분한 `context`.

## 접근 요구사항

이 메서드에는 아래 조건이 필요합니다.

- `VerifiedSurfaceContext.access_class=core_mutation`
- `verified=true`
- 같은 프로젝트의 호환되는 `Task`와 선택적 Change Unit

로컬 접근 실패, 읽을 수 없는 프로젝트나 `Task` 식별자, 부족한 로컬 역량은 커밋 전에 거절됩니다.

## 상태 버전 동작

커밋된 `dry_run`이 아닌 결과:

- `project_state.state_version`을 정확히 한 번 올립니다.
- 대기 중인 `UserJudgment` 하나를 만듭니다.
- 저장 효과 담당 문서가 허용하는 경우에만 영향받은 차단 사유 상태를 갱신할 수 있습니다.

비주장:

- 다른 메서드가 반환한 `UserJudgmentCandidate`는 `harness.request_user_judgment`가 커밋하기 전까지 지속 판단이 아닙니다.
- `dry_run`과 거절은 대기 중인 판단, 차단 사유 갱신, 이벤트, 재실행 행, 상태 버전 증가를 만들지 않습니다.

## 성공 결과

아래 값을 담은 `RequestUserJudgmentResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `user_judgment_ref`
- 대기 중인 `user_judgment`
- 영향받은 `blocker_refs`
- 현재 `state`

## 차단 결과

이 메서드에는 별도의 커밋된 차단 응답 분기가 없습니다.

대기 중인 판단을 만들 수 없으면 메서드는 커밋 전에 거절합니다.

## 거절 결과

커밋 전 실패가 있으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- 유효하지 않은 요청 형태
- 지원되지 않거나 호환되지 않는 `judgment_kind`
- 없거나 호환되지 않는 `Task` 식별자
- 미해결 선행 판단
- 로컬 접근 실패
- 부족한 역량
- 오래된 `expected_state_version`
- 검증기 실패

거절된 시도는 대기 중인 판단을 만들지 않으며, 요청처럼 보이는 차단 사유 데이터를 부수 효과로 지속하지 않습니다.

공개 오류 코드 의미, 우선순위, 거절 응답 처리 경로는 아래 오류 담당 문서가 담당합니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 미리보기:

- `ToolDryRunResponse`를 반환합니다.
- 지속되는 `user_judgment_ref`를 반환하지 않습니다.
- 대기 중인 `UserJudgment`를 만들지 않습니다.

## 저장 효과

커밋 시 대기 중인 `user_judgments` 행과 관련 차단 사유 상태를 지속할 수 있습니다. 정확한 저장 효과는 아래 저장 담당 문서가 담당합니다.

## 최소 유효 요청

메서드 안의 전제: `task_banner_001`과 `cu_banner_001`은 `proj_banner_001`에 이미 있으며, 현재 프로젝트 `state_version`은 `51`입니다.

```yaml
method: harness.request_user_judgment
params:
  envelope:
    project_id: proj_banner_001
    task_id: task_banner_001
    actor_kind: agent
    surface_id: surface_banner
    request_id: req_banner_request_001
    idempotency_key: idem_banner_request_001
    expected_state_version: 51
    dry_run: false
    locale: en-US
  task_id: task_banner_001
  change_unit_id: cu_banner_001
  judgment_kind: product_decision
  presentation: short
  question: "Should the dashboard banner use concise copy?"
  options:
    - option_id: concise
      label: "Use concise copy"
      description: "Record the user-owned product decision to keep the shorter banner copy."
      consequence: "The pending banner-copy decision can be treated as resolved."
      is_default: true
    - option_id: expanded
      label: "Use expanded copy"
      description: "Record that the banner copy should include a longer explanation."
      consequence: "The Task remains open for the expanded banner-copy change."
      is_default: false
  context:
    summary: "The dashboard banner has two candidate copy lengths and needs a user-owned product decision."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "Only banner copy length is in scope for this judgment request."
  affected_refs:
    - record_kind: task
      record_id: task_banner_001
      project_id: proj_banner_001
      task_id: task_banner_001
      state_version: 51
  required_for: close
  expires_at: null
```

## 대표 응답

축약된 결과 분기(`RequestUserJudgmentResult`, 커밋됨):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 52
  events:
    - event_id: evt_banner_001
      event_kind: user_judgment_requested
user_judgment_ref:
  record_kind: user_judgment
  record_id: uj_banner_001
  project_id: proj_banner_001
  task_id: task_banner_001
  state_version: 52
user_judgment:
  judgment_id: uj_banner_001
  project_id: proj_banner_001
  task_id: task_banner_001
  change_unit_id: cu_banner_001
  judgment_kind: product_decision
  status: pending
  presentation: short
  question: "Should the dashboard banner use concise copy?"
  options:
    - option_id: concise
      label: "Use concise copy"
      description: "Record the user-owned product decision to keep the shorter banner copy."
      consequence: "The pending banner-copy decision can be treated as resolved."
      is_default: true
    - option_id: expanded
      label: "Use expanded copy"
      description: "Record that the banner copy should include a longer explanation."
      consequence: "The Task remains open for the expanded banner-copy change."
      is_default: false
  context:
    summary: "The dashboard banner has two candidate copy lengths and needs a user-owned product decision."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "Only banner copy length is in scope for this judgment request."
  affected_refs:
    - record_kind: task
      record_id: task_banner_001
      project_id: proj_banner_001
      task_id: task_banner_001
      state_version: 51
  required_for: close
  resolution: null
  expires_at: null
  created_at: "<example-created-at>"
  resolved_at: null
blocker_refs: []
state:
  project_id: proj_banner_001
  state_version: 52
```

## 담당 문서 링크

- 요청 래퍼, 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `UserJudgment`, 선택지, 맥락, 답변 요청 본문: [API 판단 스키마](schema-judgment.md).
- 상태 참조와 요약: [API 상태 스키마](schema-state.md).
- 판단 종류와 지원 값: [API 값 집합](schema-value-sets.md).
- 사용자 소유 판단과 비대체 규칙: [Core 모델](../core-model.md).
- 정확한 저장 효과: [저장 효과](../storage-effects.md#harnessrequest_user_judgment).
- 공개 오류, 우선순위, 거절 응답 처리 경로: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md).
- 대기 중인 판단에 대한 사용자 답변 기록: [`harness.record_user_judgment`](method-record-user-judgment.md).
