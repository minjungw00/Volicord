<a id="harnessintake"></a>

# `harness.intake` 참조

## 담당하는 것

이 문서는 기준 범위의 `harness.intake` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 공유 계정 데이터 내보내기 확인 시나리오의 요청 필드와 대표 응답
- 메서드 수준 저장 효과 요약과 저장 담당 문서 링크

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`의 공통 스키마 본문
- 상태, 아티팩트, 사용자 판단, 값 집합, 오류의 중첩 스키마 정의
- 저장 DDL, 저장 기록 레이아웃, 아티팩트 생명주기, 보안 보장, Core 제품 의미

## 목적

평소 사용자 작업 루프를 시작, 재개, 대체, 거절합니다.

이 메서드는 요청된 모드를 구체적인 `Task` 상태로 확정합니다.

- `advisor`
- `direct`
- `work`

범위 경계:

- `harness.intake`는 쓰기 가능한 작업의 첫 범위 후보를 만들 수 있습니다.
- 이후 범위 변경은 `harness.update_scope`가 담당합니다.

## 필수 입력

- `ToolEnvelope`: `project_id`, `surface_id`, `request_id`, `dry_run`이 필요하며, `dry_run=false` 커밋에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `plain_language_request`, `requested_mode`, `resume_policy`.
- 알고 있는 첫 범위 후보는 `initial_scope.boundary`, `initial_scope.non_goals`, `initial_scope.acceptance_criteria`에 둡니다. 목록 필드와 `initial_context_refs`에 알려진 항목이 없으면 빈 배열을 사용합니다.

## 접근 요구사항

조건:

- `dry_run=false` 커밋입니다.
- `VerifiedSurfaceContext.access_class=core_mutation`입니다.
- `verified=true`입니다.

비주장: `surface_id`는 등록된 로컬 접점을 고르는 선택자일 뿐, 그 자체가 권한이 아닙니다.

## 상태 버전 동작

커밋된 `dry_run=false` 결과:

- 프로젝트 전체 `project_state.state_version`을 정확히 한 번 올립니다.
- 멱등 키에 대한 재실행 행을 만듭니다.

아래 경우는 `Task`, Change Unit, 이벤트, 재실행 행, 차단 사유 갱신, 상태 버전 증가를 만들지 않습니다.

- `dry_run`.
- 읽기 실패.
- 검증 실패.
- 로컬 접근 실패.
- 오래된 `expected_state_version`.

## 성공 결과

아래 값을 담은 `IntakeResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `task_ref`
- 선택적 `change_unit_ref`
- 현재 `state`
- `next_actions`

`requested_mode=auto`라면 저장되고 표시되는 모드는 확정된 구체적 모드여야 하며 `auto`가 되면 안 됩니다.

## 차단 결과

이 메서드는 쓰기 준비 경로 대신 shaping 또는 차단 사유 상태를 기록하는 커밋된 `IntakeResult`를 반환할 수 있습니다.

차단 질문은 아래 필드로 표현해야 합니다.

- `Task`.
- Change Unit.
- 사용자 판단.
- 증거.
- 차단 사유.
- 다음 행동.

비주장: 별도 Discovery Brief, Question Queue, Assumption Register 아티팩트는 만들지 않습니다.

## 거절 결과

커밋 전 실패가 있으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- 검증 실패.
- 오래된 `expected_state_version`.
- Core 또는 로컬 접점 사용 불가.
- 로컬 접근 불일치.
- 활성 `Task` 호환성 부족.
- 검증기 실패.

공개 오류 코드 의미와 우선순위는 [API 오류](errors.md)가 담당합니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 상태 효과 미리보기:

- `ToolDryRunResponse`를 반환합니다.
- `IntakeResult`를 반환하지 않습니다.

분기 형태는 [API 코어 스키마](schema-core.md)가 담당하고, 저장 효과 없음 의미는 [저장 효과](../storage-effects.md)가 담당합니다.

## 저장 효과

커밋 시 `harness.intake`가 담당하는 `Task` 또는 Change Unit 상태를 지속할 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당하고, 저장 기록 형태는 [저장소 기록](../storage-records.md)이 담당합니다.

## 시나리오 요청 예시

```yaml
method: harness.intake
params:
  plain_language_request: "계정 데이터 내보내기 전에 명시적 확인 단계를 추가한다."
  initial_scope:
    boundary: "계정 데이터 내보내기 흐름과 계정 데이터 내보내기 확인 테스트만."
    non_goals:
      - "계정 삭제 동작"
    acceptance_criteria:
      - "계정 데이터 내보내기 파일을 다운로드하기 전에 명시적 확인 단계가 필요하다."
```

## 대표 응답

결과 분기(`IntakeResult`, 커밋됨):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 18
  events:
    - event_id: evt_1001
      event_kind: task_intake
task_ref:
  record_kind: task
  record_id: task_456
  project_id: proj_123
  task_id: task_456
  state_version: 18
change_unit_ref: null
state:
  project_id: proj_123
  state_version: 18
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 18
  mode: work
  lifecycle:
    lifecycle_phase: shaping
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "계정 데이터 내보내기 전에 명시적 확인 단계를 추가한다."
  scope_summary: "계정 데이터 내보내기 흐름과 계정 데이터 내보내기 확인 테스트만."
  non_goals:
    - "계정 삭제 동작"
  acceptance_criteria:
    - "계정 데이터 내보내기 파일을 다운로드하기 전에 명시적 확인 단계가 필요하다."
  active_change_unit_ref: null
  blocker_refs: []
next_actions:
  - action: harness.update_scope
    reason: "쓰기 확인 전에 첫 활성 Change Unit을 만든다."
```

## 담당 문서 링크

- 요청 래퍼와 응답 분기: [`ToolEnvelope`](schema-core.md#tool-envelope), [공통 응답 분기](schema-core.md#common-response).
- 상태 참조, `StateSummary`, `ShapingReadiness`, 다음 행동: [API 상태 스키마](schema-state.md).
- 활성 메서드 이름, 모드 값, `resume_policy`, `response_kind`, `effect_kind`, 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류와 상태 버전 충돌: [API 오류](errors.md).
- 저장 효과: [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).
