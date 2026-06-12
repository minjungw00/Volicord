# `harness.request_user_judgment`와 `harness.record_user_judgment` 참조

## 담당하는 것

이 문서는 현재 MVP의 `harness.request_user_judgment`와 `harness.record_user_judgment` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 공유 계정 데이터 내보내기 확인 시나리오의 최소 요청과 대표 응답
- 대기 중인 사용자 판단을 만드는 동작과 사용자의 답을 기록하는 동작의 메서드 경계

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`의 공통 스키마 본문
- `UserJudgment` 스키마 필드 정의, 판단 값 집합, 공개 오류 우선순위, 저장 기록 레이아웃
- Core의 사용자 소유 판단 의미, 최종 수락 의미, 잔여 위험 의미, 닫기 준비 상태 의미

## 목적


<a id="harnessrequest_user_judgment"></a>

### `harness.request_user_judgment`

초점이 분명한 사용자 소유 결정 하나에 대해 대기 중인 `UserJudgment`를 만듭니다.

결과:

- 이 메서드는 사용자에게 묻는 경로입니다.

비주장:

- 에이전트가 사용자를 대신해 답하지 않습니다.
- 에이전트가 사용자를 대신해 추론하지 않습니다.
- 에이전트가 질문 범위를 넓히지 않습니다.
- 에이전트가 결정을 내리지 않습니다.


<a id="harnessrecord_user_judgment"></a>

### `harness.record_user_judgment`

기존 대기 중인 `UserJudgment` 하나에 대한 사용자의 답을 기록합니다.

결과:

- 사용자의 답에 따라 특정 대기 판단을 `resolved`, `rejected`, `deferred`, `blocked` 또는 해당 상태로 표시합니다.

비주장:

- 답변을 관련 없는 승인으로 넓히지 않습니다.
- 답변을 범위 확장으로 넓히지 않습니다.
- 답변을 수락이나 잔여 위험 수락으로 넓히지 않습니다.
- 답변을 쓰기 승인으로 넓히지 않습니다.

## 필수 입력


### `harness.request_user_judgment`

- `ToolEnvelope`: `dry_run=false` 커밋에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `task_id`, `change_unit_id`, `judgment_kind`, `presentation`, `question`, `options`, `context`, `affected_refs`, `required_for`, `expires_at`.
- 사용자가 정확한 사안을 판단할 수 있도록 초점이 분명한 판단 프롬프트(`question`), 이해 가능한 선택지, 충분한 맥락.


### `harness.record_user_judgment`

- `ToolEnvelope`: `dry_run=false` 커밋에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `user_judgment_id`, 일치하는 `judgment_kind`, `selected_option_id`, `answer`, `note`, `accepted_risks`.
- `answer`에는 대기 중인 `judgment_kind`에 맞는 결정별 페이로드 분기만 담아야 합니다. `selected_option_id`와 `note`는 요청 수준에 남습니다.

## 접근 요구사항


### `harness.request_user_judgment`

`VerifiedSurfaceContext.access_class=core_mutation`과 `verified=true`가 필요합니다. 요청은 같은 프로젝트의 호환되는 Task와 선택적 Change Unit을 대상으로 해야 합니다.


### `harness.record_user_judgment`

`VerifiedSurfaceContext.access_class=core_mutation`과 `verified=true`가 필요합니다. 대기 중인 판단은 요청이 선택한 같은 프로젝트와 호환되는 Task에 속해야 합니다.

## 상태 버전 동작


### `harness.request_user_judgment`

커밋된 `dry_run=false` 결과:

- `project_state.state_version`을 정확히 한 번 올립니다.
- 대기 중인 판단을 만듭니다.

비주장:

- 다른 메서드가 반환한 후보는 이 메서드가 커밋하기 전까지 지속 기록이 아닙니다.
- `dry_run`과 거절은 대기 중인 판단, 차단 사유 갱신, 이벤트, 재실행 행, 상태 버전 증가를 만들지 않습니다.


### `harness.record_user_judgment`

커밋된 `dry_run=false` 결과:

- `project_state.state_version`을 정확히 한 번 올립니다.
- 지정된 `user_judgments` 행을 갱신합니다.

비주장:

- `dry_run`과 거절은 판단 해결, 차단 사유 갱신, 이벤트, 재실행 행, 상태 버전 증가를 만들지 않습니다.

## 성공 결과


### `harness.request_user_judgment`

`base.response_kind=result`, `base.effect_kind=core_committed`인 `RequestUserJudgmentResult`를 반환합니다. 결과에는 `user_judgment_ref`, 대기 중인 `user_judgment`, 영향을 받은 `blocker_refs`, 현재 `state`가 들어갑니다.


### `harness.record_user_judgment`

`base.response_kind=result`, `base.effect_kind=core_committed`인 `RecordUserJudgmentResult`를 반환합니다. 결과에는 `user_judgment_ref`, 갱신된 `user_judgment`, `updated_refs`, 현재 `state`, `next_actions`가 들어갑니다.

## 차단 결과


### `harness.request_user_judgment`

별도 커밋된 차단 응답 분기는 없습니다. 요청이 유효하지 않거나 선행조건을 확인할 수 없어 판단을 만들 수 없으면 메서드는 커밋 전에 거절합니다.


### `harness.record_user_judgment`

사용자의 답이 그렇거나 초점이 맞는 판단의 호환 결과가 그렇다면 지정된 판단은 `rejected`, `deferred`, `blocked` 또는 차단 사유를 만드는 상태로 커밋될 수 있습니다.

결과:

- 포함된 차단 사유와 판단에 의존하는 요약만 갱신합니다.

비주장:

- 해결된 `scope_decision`만으로 활성 범위나 활성 Change Unit 필드가 바뀌지 않습니다.
- 해당 필드를 바꾸려면 여전히 `harness.update_scope`가 필요합니다.

## 거절 결과


### `harness.request_user_judgment`

아래 경우는 `ToolRejectedResponse`를 반환합니다.

- 유효하지 않은 질문 형태.
- 유효하지 않은 `judgment_kind`.
- Task 없음.
- 미해결 선행 판단.
- 로컬 접근 실패.
- 역량 부족.
- 오래된 `expected_state_version`.
- validator 실패.

공개 오류 코드 의미와 우선순위는 [API 오류](errors.md)가 담당합니다.


### `harness.record_user_judgment`

아래 경우는 `ToolRejectedResponse`를 반환합니다.

- 오래된 `expected_state_version`.
- 알 수 없거나 `pending`이 아닌 판단.
- `judgment_kind` 불일치.
- 유효하지 않은 선택지.
- 유효하지 않은 답변 페이로드.
- 만료되었거나 호환되지 않는 승인.
- 로컬 접근 실패.
- validator 실패.

공개 오류 코드 의미와 우선순위는 [API 오류](errors.md)가 담당합니다.

## `dry_run` 동작


### `harness.request_user_judgment`

`dry_run=true`에서 유효한 미리보기는 `ToolDryRunResponse`를 반환합니다. 분기 형태는 [API 코어 스키마](schema-core.md)가 담당하고, 저장 효과 없음 의미는 [저장 효과](../storage-effects.md)가 담당합니다.


### `harness.record_user_judgment`

`dry_run=true`에서 유효한 미리보기는 `ToolDryRunResponse`를 반환합니다. 분기 형태는 [API 코어 스키마](schema-core.md)가 담당하고, 저장 효과 없음 의미는 [저장 효과](../storage-effects.md)가 담당합니다.

## 저장 효과


### `harness.request_user_judgment`

커밋 시 대기 중인 판단과 관련 차단 사유 상태를 지속할 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당합니다.


### `harness.record_user_judgment`

커밋 시 판단 해결과 그에 따른 차단 사유 또는 요약 상태를 지속할 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당합니다.

## 예시 정합성

현재 `UserJudgment` 스키마에서 사용자에게 보이는 판단 프롬프트는 `question` 필드입니다. 계정 내보내기 확인 문구에 대한 사용자 판단은 이 프롬프트와 `context.summary`에 담습니다. 이 예시는 아티팩트를 근거로 들지 않으므로 `context.artifact_refs: []`는 의도한 값입니다.

요청과 응답 예시는 같은 `options` 선택지 값과 같은 Task를 가리키는 `affected_refs` 영향 받는 ref를 유지합니다. `record_user_judgment` 예시는 `accept`를 선택하고 `decision: accepted`를 기록하며, 근거는 충분함 선택지와 같은 의미입니다. 시간 필드는 `null` 또는 플레이스홀더 값을 사용합니다.

## 최소 유효 요청


### `harness.request_user_judgment`

```yaml
method: harness.request_user_judgment
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_judgment_001
    idempotency_key: idem_judgment_001
    expected_state_version: 21
    dry_run: false
    locale: ko-KR
  task_id: task_456
  change_unit_id: cu_001
  judgment_kind: product_decision
  presentation: short
  question: "다운로드에 개인정보가 포함될 수 있다고 알리는 계정 내보내기 확인 문구를 충분한 것으로 수락해도 됩니까?"
  options:
    - option_id: accept
      label: "충분함"
      description: "계정 내보내기 확인 문구가 충분하다는 사용자 판단을 기록합니다."
      consequence: "닫기 준비 상태가 제품 판단을 해결된 것으로 평가할 수 있습니다."
      is_default: true
    - option_id: revise
      label: "수정 필요"
      description: "수정된 계정 내보내기 확인 문구가 필요하므로 Task를 열어 둡니다."
      consequence: "제품 판단이 남아 있어 닫기가 계속 차단됩니다."
      is_default: false
  context:
    summary: "다운로드 전에 보이는 계정 내보내기 확인 문구는 계정 데이터 내보내기에 개인정보가 포함될 수 있음을 알립니다."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "계정 내보내기 흐름과 계정 내보내기 확인 테스트는 범위 안에 있고, 계정 삭제 동작은 범위 밖입니다."
  affected_refs:
    - record_kind: task
      record_id: task_456
      project_id: proj_123
      task_id: task_456
      state_version: 21
  required_for: close
  expires_at: null
```


### `harness.record_user_judgment`

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
        rationale: "계정 내보내기 확인 문구는 계정 데이터 내보내기에 개인정보가 포함될 수 있음을 명확히 알립니다."
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


### `harness.request_user_judgment`

결과 분기(`RequestUserJudgmentResult`, 커밋됨):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 22
  events:
    - event_id: evt_1005
      event_kind: user_judgment_requested
user_judgment_ref:
  record_kind: user_judgment
  record_id: uj_001
  project_id: proj_123
  task_id: task_456
  state_version: 22
user_judgment:
  judgment_id: uj_001
  project_id: proj_123
  task_id: task_456
  change_unit_id: cu_001
  judgment_kind: product_decision
  status: pending
  presentation: short
  question: "다운로드에 개인정보가 포함될 수 있다고 알리는 계정 내보내기 확인 문구를 충분한 것으로 수락해도 됩니까?"
  options:
    - option_id: accept
      label: "충분함"
      description: "계정 내보내기 확인 문구가 충분하다는 사용자 판단을 기록합니다."
      consequence: "닫기 준비 상태가 제품 판단을 해결된 것으로 평가할 수 있습니다."
      is_default: true
    - option_id: revise
      label: "수정 필요"
      description: "수정된 계정 내보내기 확인 문구가 필요하므로 Task를 열어 둡니다."
      consequence: "제품 판단이 남아 있어 닫기가 계속 차단됩니다."
      is_default: false
  context:
    summary: "다운로드 전에 보이는 계정 내보내기 확인 문구는 계정 데이터 내보내기에 개인정보가 포함될 수 있음을 알립니다."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "계정 내보내기 흐름과 계정 내보내기 확인 테스트는 범위 안에 있고, 계정 삭제 동작은 범위 밖입니다."
  affected_refs:
    - record_kind: task
      record_id: task_456
      project_id: proj_123
      task_id: task_456
      state_version: 21
  required_for: close
  resolution: null
  expires_at: null
  created_at: "<example-created-at>"
  resolved_at: null
blocker_refs: []
state:
  project_id: proj_123
  state_version: 22
```


### `harness.record_user_judgment`

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
  question: "다운로드에 개인정보가 포함될 수 있다고 알리는 계정 내보내기 확인 문구를 충분한 것으로 수락해도 됩니까?"
  options:
    - option_id: accept
      label: "충분함"
      description: "계정 내보내기 확인 문구가 충분하다는 사용자 판단을 기록합니다."
      consequence: "닫기 준비 상태가 제품 판단을 해결된 것으로 평가할 수 있습니다."
      is_default: true
    - option_id: revise
      label: "수정 필요"
      description: "수정된 계정 내보내기 확인 문구가 필요하므로 Task를 열어 둡니다."
      consequence: "제품 판단이 남아 있어 닫기가 계속 차단됩니다."
      is_default: false
  context:
    summary: "다운로드 전에 보이는 계정 내보내기 확인 문구는 계정 데이터 내보내기에 개인정보가 포함될 수 있음을 알립니다."
    related_refs: []
    artifact_refs: []
    visible_risks: []
    constraints:
      - "계정 내보내기 흐름과 계정 내보내기 확인 테스트는 범위 안에 있고, 계정 삭제 동작은 범위 밖입니다."
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
          rationale: "계정 내보내기 확인 문구는 계정 데이터 내보내기에 개인정보가 포함될 수 있음을 명확히 알립니다."
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
  - action: harness.close_task
    reason: "사용자 판단을 기록한 뒤 닫기 준비 상태를 평가한다."
```

## 담당 문서 링크


### `harness.request_user_judgment`

- 요청 래퍼, 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `UserJudgment`, 선택지, 맥락, 답변 페이로드: [API 판단 스키마](schema-judgment.md).
- 상태 참조와 요약: [API 상태 스키마](schema-state.md).
- 판단 종류와 활성 값: [API 값 집합](schema-value-sets.md).
- 사용자 소유 판단과 비대체 규칙: [Core 모델](../core-model.md).
- 공개 오류와 저장 효과: [API 오류](errors.md), [저장 효과](../storage-effects.md).


### `harness.record_user_judgment`

- 요청 래퍼, 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `UserJudgment`, `RecordUserJudgmentPayload`, `SensitiveActionScope`, `AcceptedRiskInput`: [API 판단 스키마](schema-judgment.md).
- 상태 참조와 요약: [API 상태 스키마](schema-state.md).
- 판단 값과 활성 메서드 내부 값: [API 값 집합](schema-value-sets.md).
- 사용자 소유 판단, 최종 수락, 잔여 위험 수락, 비대체 규칙: [Core 모델](../core-model.md).
- 공개 오류와 저장 효과: [API 오류](errors.md), [저장 효과](../storage-effects.md).
