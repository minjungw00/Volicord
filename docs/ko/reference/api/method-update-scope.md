<a id="harnessupdate_scope"></a>

# `harness.update_scope` 참조

## 담당하는 것

이 문서는 기준 범위의 `harness.update_scope` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- intake 이후 범위와 Change Unit을 갱신하는 동작
- update-scope 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, `dry_run`, 거절 응답 스키마 본문
- 상태, 아티팩트, 판단, 값 집합, 오류의 중첩 스키마 정의
- 저장 DDL, 저장 기록 레이아웃, 정확한 저장 효과, 아티팩트 생명주기, 보안 보장, Core 권한 의미
- 공개 오류 코드 의미, 공개 오류 우선순위, 공통 응답 분기 처리 경로

## 목적

`harness.update_scope`는 `harness.intake` 이후 현재 `Task`와 현재 적용 Change Unit 필드를 갱신합니다.

- 목표 요약
- 범위 경계
- 범위 밖 항목
- 수락 기준
- 자율성 경계
- 기준선 참조
- 현재 적용 Change Unit

이 메서드는 사용자 소유 차단 사유가 처리되면 shaping 상태를 안전한 첫 Change Unit으로 옮기는 지원 경로입니다.

## 필수 입력

- 유효한 `ToolEnvelope`. 커밋되는 `dry_run`이 아닌 요청에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `task_id`.
- 바꿀 범위 필드. 포함/제외 방식으로 범위를 갱신할 때는 `scope_update.include`에 범위에 포함할 제품 작업을, `scope_update.exclude`에 범위에서 제외할 제품 동작을 둡니다. `null`은 기존 값을 유지한다는 뜻이고, 빈 배열은 그 목록을 빈 목록으로 교체합니다.
- `change_unit.operation`과 그 작업에 필요한 필드.
- 해결된 `judgment_kind=scope_decision`을 적용한다면 `related_scope_decision_refs`.

## 접근 요구사항

커밋되는 `dry_run`이 아닌 요청에는 아래 조건이 필요합니다.

- `VerifiedSurfaceContext.access_class=core_mutation`
- `verified=true`
- 같은 프로젝트의 호환되는 `Task`
- 현재 적용 Change Unit을 만들거나 교체할 때 다음 안전한 행동을 정직하게 만들 만큼 충분한 범위

## 상태 버전 동작

커밋된 `dry_run`이 아닌 결과는 `project_state.state_version`을 정확히 한 번 올립니다.

기준이 아래 항목과 더 이상 맞지 않으면 Core는 `status=active`인 `Write Authorization`(쓰기 권한 부여)을 `status=stale`로 표시합니다.

- 현재 적용 범위
- 기준선
- 수락 기준
- 범위 밖 항목
- 자율성 경계
- 현재 적용 Change Unit
- 프로젝트 상태

비주장: `status=stale` 표시는 소비, 철회, 만료, 조용한 재사용이 아닙니다.

## 성공 결과

아래 값을 담은 `UpdateScopeResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `task_ref`
- 선택적 `change_unit_ref`
- 연결된 `scope_decision` 참조
- 오래된 `Write Authorization` 참조
- 차단 사유 참조
- 현재 `state`
- `next_actions`

## 차단 결과

범위가 아직 준비되지 않았을 때 메서드가 소유한 차단 사유 또는 현재 행 갱신을 커밋할 수 있습니다.

커밋된 차단 범위 결과는 필요한 사용자 소유 판단 범주를 식별해야 합니다.

- `product_decision`
- `technical_decision`
- `scope_decision`
- `sensitive_approval`

허용되지 않는 것:

- 차단된 범위 결과는 필요한 판단을 막연한 모호함 뒤에 숨기면 안 됩니다.

## 거절 결과

커밋 전 실패가 있으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- 오래된 `expected_state_version`
- 유효하지 않은 `Task` 식별
- 유효하지 않은 Change Unit 작업
- 필요한 범위 누락
- 범위 위반
- 미해결 필수 판단
- 자율성 경계 위반
- 오래된 기준선
- 로컬 접근 실패
- 검증기 실패

공개 오류 코드 의미, 우선순위, 거절 응답 처리 경로는 아래 오류 담당 문서가 담당합니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 상태 효과 미리보기:

- `ToolDryRunResponse`를 반환합니다.
- 범위, Change Unit, 차단 사유, `Write Authorization` 상태를 만들지 않습니다.

## 저장 효과

커밋 시 범위 담당 현재 상태와 오래된 권한 부여 결과를 지속할 수 있습니다. 정확한 저장 효과는 아래 저장 담당 문서가 담당합니다.

## 최소 유효 요청

```yaml
method: harness.update_scope
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_scope_001
    idempotency_key: idem_scope_001
    expected_state_version: 18
    dry_run: false
    locale: ko-KR
  task_id: task_456
  goal_summary: "인보이스 PDF 다운로드 전에 확인 단계를 추가한다."
  scope_update:
    include:
      - "인보이스 PDF 다운로드 흐름에서 확인을 요구하도록 갱신한다."
      - "인보이스 다운로드 확인 테스트를 갱신한다."
    exclude:
      - "인보이스 생성 방식."
  scope_boundary: "인보이스 PDF 다운로드 확인과 관련 테스트."
  non_goals:
    - "인보이스 생성 방식."
  acceptance_criteria:
    - "인보이스 PDF를 다운로드하려면 명시적 확인이 필요하다."
  autonomy_boundary: "인보이스 PDF 다운로드 확인과 관련 테스트 안에서만 작업한다."
  baseline_ref: baseline_invoice_download_001
  change_unit:
    operation: create_current
    scope_summary: "인보이스 PDF 다운로드 확인과 관련 테스트."
    affected_areas:
      - "인보이스 PDF 다운로드 흐름"
      - "인보이스 다운로드 확인 테스트"
    affected_paths:
      - src/billing/invoice-download.ts
      - src/billing/invoice-download-confirmation.ts
      - tests/invoice-download.test.ts
    constraints:
      - "인보이스 생성 방식은 범위 밖으로 둔다."
  related_scope_decision_refs: []
```

## 대표 응답

결과 분기(`UpdateScopeResult`, 커밋됨):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 19
  events:
    - event_id: evt_1002
      event_kind: scope_updated
task_ref:
  record_kind: task
  record_id: task_456
  project_id: proj_123
  task_id: task_456
  state_version: 19
change_unit_ref:
  record_kind: change_unit
  record_id: cu_001
  project_id: proj_123
  task_id: task_456
  state_version: 19
linked_scope_decision_refs: []
stale_write_authorization_refs: []
blocker_refs: []
state:
  project_id: proj_123
  state_version: 19
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 19
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "인보이스 PDF 다운로드 전에 확인 단계를 추가한다."
  scope_summary: "인보이스 PDF 다운로드 확인과 관련 테스트."
  non_goals:
    - "인보이스 생성 방식."
  acceptance_criteria:
    - "인보이스 PDF를 다운로드하려면 명시적 확인이 필요하다."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_001
    project_id: proj_123
    task_id: task_456
    state_version: 19
next_actions:
  - action_kind: prepare_write
    owner_method: harness.prepare_write
    label: "인보이스 다운로드 변경을 현재 적용 범위와 비교한다."
    blocking_question: null
    required_refs:
      - record_kind: task
        record_id: task_456
        project_id: proj_123
        task_id: task_456
        state_version: 19
      - record_kind: change_unit
        record_id: cu_001
        project_id: proj_123
        task_id: task_456
        state_version: 19
```

## 담당 문서 링크

- 요청 래퍼와 응답 분기: [API 코어 스키마](schema-core.md).
- 상태 참조, `StateSummary`, `ShapingReadiness`, 차단 사유, 다음 행동: [API 상태 스키마](schema-state.md).
- 범위 관련 사용자 판단 형태: [API 판단 스키마](schema-judgment.md).
- 지원되는 값 집합과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류, 우선순위, 거절 응답 처리 경로: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md).
- 저장 효과와 오래된 권한 부여 동작: [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).
