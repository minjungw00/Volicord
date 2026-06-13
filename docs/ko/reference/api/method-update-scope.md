<a id="harnessupdate_scope"></a>

# `harness.update_scope` 참조

## 담당하는 것

이 문서는 기준 범위의 `harness.update_scope` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 공유 계정 데이터 내보내기 확인 시나리오의 최소 요청과 대표 응답
- 메서드 수준 저장 효과 요약과 저장 담당 문서 링크

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`의 공통 스키마 본문
- 상태, 아티팩트, 사용자 판단, 값 집합, 오류의 중첩 스키마 정의
- 저장 DDL, 저장 기록 레이아웃, 아티팩트 생명주기, 보안 보장, Core 제품 의미

## 목적

`harness.intake` 이후 활성 `Task`와 Change Unit 필드를 갱신합니다.

- 목표 요약
- 범위 경계
- 범위 밖 항목
- 수락 기준
- 자율성 경계
- 기준선 참조
- 활성 Change Unit

결과: 사용자 소유 차단 사유가 처리되면 shaping 상태를 안전한 첫 Change Unit으로 옮기는 활성 경로입니다.

## 필수 입력

- `ToolEnvelope`: `dry_run=false` 커밋에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `task_id`.
- 바꿀 범위 필드. 포함/제외 방식으로 범위를 갱신할 때는 `scope_update.include`에 범위에 포함할 제품 작업을, `scope_update.exclude`에 범위에서 제외할 제품 동작을 둡니다. `null`은 현재 값을 유지한다는 뜻이고, 빈 배열은 그 목록을 빈 목록으로 교체합니다.
- `change_unit.operation`과 그 작업에 필요한 필드.
- 해결된 `judgment_kind=scope_decision`을 적용한다면 `related_scope_decision_refs`.

## 접근 요구사항

조건:

- `dry_run=false` 커밋입니다.
- `VerifiedSurfaceContext.access_class=core_mutation`입니다.
- `verified=true`입니다.
- 요청은 같은 프로젝트의 호환되는 `Task`를 식별합니다.
- 활성 Change Unit을 만들거나 교체할 때는 다음 안전한 행동을 정직하게 만들 만큼의 범위를 제공합니다.

## 상태 버전 동작

커밋된 `dry_run=false` 결과:

- `project_state.state_version`을 정확히 한 번 올립니다.

활성 쓰기 승인(`Write Authorization`)의 기준 상태와 더 이상 맞지 않으면 Core는 그 승인을 `status=stale`로 표시합니다. 비교 대상은 아래와 같습니다.

- 범위.
- 기준선.
- 수락 기준.
- 범위 밖 항목.
- 자율성 경계.
- Change Unit.
- 프로젝트 상태.

비주장: `status=stale` 표시는 소비, 철회, 만료, 조용한 재사용이 아닙니다.

## 성공 결과

아래 값을 담은 `UpdateScopeResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `task_ref`
- 선택적 `change_unit_ref`
- 연결된 `scope_decision` 참조
- `status=stale` 쓰기 승인 참조
- 차단 사유 참조
- 현재 `state`
- `next_actions`

## 차단 결과

범위가 아직 준비되지 않았을 때 메서드가 소유한 차단 사유 또는 현재 행 갱신을 커밋할 수 있습니다.

커밋된 차단 범위 결과는 필요한 사용자 소유 판단 범주를 식별해야 합니다.

- `product_decision`.
- `technical_decision`.
- `scope_decision`.
- `sensitive_approval`.

비주장: 필요한 판단을 막연한 모호함 뒤에 숨기면 안 됩니다.

## 거절 결과

커밋 전 실패가 있으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- 오래된 `expected_state_version`.
- 유효하지 않은 `Task` 식별.
- 유효하지 않은 Change Unit 작업.
- 필요한 범위 누락.
- 범위 위반.
- 미해결 필수 판단.
- 자율성 경계 위반.
- 기준선이 오래되었습니다.
- 로컬 접근 실패.
- 검증기 실패.

공개 오류 코드 의미와 우선순위는 [API 오류](errors.md)가 담당합니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 상태 효과 미리보기는 `ToolDryRunResponse`를 반환합니다. 분기 형태는 [API 코어 스키마](schema-core.md)가 담당하고, 저장 효과 없음 의미는 [저장 효과](../storage-effects.md)가 담당합니다.

## 저장 효과

커밋 시 범위 담당 현재 상태와 `status=stale` 승인 결과를 지속할 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당합니다.

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
  goal_summary: "계정 데이터 내보내기 전에 명시적 확인 단계를 추가한다."
  scope_update:
    include:
      - "계정 데이터 내보내기 파일을 다운로드하기 전에 명시적 확인 단계가 필요하도록 계정 데이터 내보내기 흐름을 갱신한다."
      - "계정 데이터 내보내기 확인 테스트를 갱신한다."
    exclude:
      - "계정 삭제 동작"
  scope_boundary: "계정 데이터 내보내기 흐름과 계정 데이터 내보내기 확인 테스트."
  non_goals:
    - "계정 삭제 동작"
  acceptance_criteria:
    - "계정 데이터 내보내기 파일을 다운로드하기 전에 명시적 확인 단계가 필요하다."
  autonomy_boundary: "계정 데이터 내보내기 흐름과 계정 데이터 내보내기 확인 테스트 범위 안에서만 작업한다."
  baseline_ref: baseline_account_export_001
  change_unit:
    operation: create_active
    scope_summary: "계정 데이터 내보내기 흐름과 계정 데이터 내보내기 확인 테스트."
    affected_areas:
      - "계정 데이터 내보내기 흐름"
      - "계정 데이터 내보내기 확인 테스트"
    affected_paths:
      - src/account/export.ts
      - src/account/export-confirmation.ts
      - tests/account-export.test.ts
    constraints:
      - "계정 삭제 동작은 범위에서 제외한다."
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
  goal_summary: "계정 데이터 내보내기 전에 명시적 확인 단계를 추가한다."
  scope_summary: "계정 데이터 내보내기 흐름과 계정 데이터 내보내기 확인 테스트."
  non_goals:
    - "계정 삭제 동작"
  acceptance_criteria:
    - "계정 데이터 내보내기 파일을 다운로드하기 전에 명시적 확인 단계가 필요하다."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_001
    project_id: proj_123
    task_id: task_456
    state_version: 19
next_actions:
  - action: harness.prepare_write
    reason: "계정 데이터 내보내기 변경을 활성 범위와 비교한다."
```

## 담당 문서 링크

- 요청 래퍼와 응답 분기: [API 코어 스키마](schema-core.md).
- 상태 참조, `StateSummary`, `ShapingReadiness`, 차단 사유, 다음 행동: [API 상태 스키마](schema-state.md).
- 범위 관련 사용자 판단 형태: [API 판단 스키마](schema-judgment.md).
- 활성 값 집합과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류: [API 오류](errors.md).
- 저장 효과와 `status=stale` 쓰기 승인 동작: [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).
