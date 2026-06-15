<a id="harnessstatus"></a>

# `harness.status` 참조

## 담당하는 것

이 문서는 기준 범위의 `harness.status` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 현재 Core 상태에 대한 읽기 전용 status 동작
- status 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, `dry_run`, 거절 응답 스키마 본문
- 상태, 아티팩트, 판단, 값 집합, 오류의 중첩 스키마 정의
- 저장 DDL, 저장 기록 레이아웃, 정확한 저장 효과, 아티팩트 생명주기, 보안 보장, Core 권한 의미
- 공개 오류 코드 의미, 공개 오류 우선순위, 공통 응답 분기 처리 경로

## 목적

`harness.status`는 Core 상태의 읽기 전용 현재 위치 보기를 반환합니다. 현재 `Task` 요약, 차단 사유, 대기 중인 사용자 판단, `Write Authorization` 요약, 증거 요약, 닫기 상태, 닫기 준비 상태 발견 사항, 보장 표시, 다음 안전한 행동을 포함할 수 있습니다.

## 필수 입력

- 유효한 `ToolEnvelope`. `idempotency_key`와 `expected_state_version`은 `null`일 수 있습니다.
- 호출자가 필요한 요약을 고르는 `include` 플래그.

## 접근 요구사항

보호된 Core 세부정보를 요청할 때 읽기에는 아래 조건이 필요합니다.

- 같은 프로젝트의 현재 로컬 접점
- `VerifiedSurfaceContext.access_class=read_status`

이 응답에서 상태 권한 근거는 `StatusResult`가 요약하는 Core 소유 상태입니다.

## 상태 버전 동작

상태 변경은 없고 `project_state.state_version`은 절대 증가하지 않습니다.

결과는 현재 관찰된 상태 버전을 보고할 수 있습니다.

이 메서드는 아래 항목을 만들지 않습니다.

- 이벤트
- 재실행 행
- 닫기 변경
- 아티팩트 효과
- 스테이징 핸들 소비
- 증거 갱신
- `Write Authorization` 변경

## 성공 결과

아래 값을 담은 `StatusResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=read_only`

`include.close=true`일 때 `StatusResult.close_blockers`는 읽기 전용 관찰인 `CloseReadinessBlocker[]`입니다.

비주장: `StatusResult.close_blockers`는 저장된 `close_task` 결과가 아닙니다.

## 차단 결과

커밋된 차단 분기는 없습니다.

`StatusResult`의 차단 사유와 닫기 차단 사유는 계산된 응답 필드일 뿐입니다.

## 거절 결과

읽기를 안전하게 제공할 수 없으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- Core 사용 불가
- 로컬 접근 불일치
- 요청한 보호 세부정보에 대한 역량 부족
- `Task` 범위 읽기에 필요한 현재 `Task` 없음
- 요청한 상태 보기가 오래되었거나 사용 불가

공개 오류 코드 의미, 우선순위, 거절 응답 처리 경로는 아래 오류 담당 문서가 담당합니다.

## `dry_run` 동작

이 읽기 전용 메서드에서는 `dry_run=true`가 `ToolDryRunResponse` 분기를 만들지 않습니다.

유효한 요청은 같은 `StatusResult` 형태를 반환합니다.

- `base.dry_run=true`
- `base.effect_kind=read_only`

## 저장 효과

이 메서드는 읽기 전용입니다. 정확한 저장 효과 없음 의미는 아래 저장 담당 문서가 담당합니다.

## 최소 유효 요청

```yaml
method: harness.status
params:
  envelope:
    project_id: proj_export_001
    task_id: task_export_001
    actor_kind: agent
    surface_id: surface_status
    request_id: req_status_export_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: en-US
  include:
    task: true
    pending_user_judgments: true
    write_authority: false
    evidence: false
    close: true
    guarantees: true
```

## 대표 응답

결과 분기(`StatusResult`, 읽기 전용):

```yaml
base:
  response_kind: result
  effect_kind: read_only
  dry_run: false
  state_version: 42
  events: []
active_task:
  project_id: proj_export_001
  state_version: 42
  task_ref:
    record_kind: task
    record_id: task_export_001
    project_id: proj_export_001
    task_id: task_export_001
    state_version: 42
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Add CSV summary export for dashboard totals."
  scope_summary: "CSV export column order and summary totals."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_export_001
    project_id: proj_export_001
    task_id: task_export_001
    state_version: 41
status_summary: "A user-owned product decision about CSV column order is pending."
next_actions:
  - action_kind: record_user_judgment
    owner_method: harness.record_user_judgment
    label: "Record the user's answer for the pending CSV column decision."
    blocking_question: "What is the user's answer for the pending CSV column decision?"
    required_refs:
      - record_kind: user_judgment
        record_id: uj_export_columns_001
        project_id: proj_export_001
        task_id: task_export_001
        state_version: 42
pending_user_judgments:
  - record_kind: user_judgment
    record_id: uj_export_columns_001
    project_id: proj_export_001
    task_id: task_export_001
    state_version: 42
blocker_refs: []
close_state: blocked
close_blockers:
  - category: user_judgment
    code: missing_user_judgment
    message: "User-owned product decision about CSV column order is still pending."
    related_refs:
      - record_kind: user_judgment
        record_id: uj_export_columns_001
        project_id: proj_export_001
        task_id: task_export_001
        state_version: 42
    next_actions:
      - action_kind: record_user_judgment
        owner_method: harness.record_user_judgment
        label: "Record the user's answer for the pending CSV column decision."
        blocking_question: "What is the user's answer for the pending CSV column decision?"
        required_refs:
          - record_kind: user_judgment
            record_id: uj_export_columns_001
            project_id: proj_export_001
            task_id: task_export_001
            state_version: 42
guarantee_display:
  level: cooperative
  basis: "No stronger local guarantee is currently applied."
  capability_refs: []
```

## 담당 문서 링크

- 요청 래퍼와 응답 분기: [API 코어 스키마](schema-core.md).
- 상태, 닫기 준비 상태 형태, 증거 요약, 보장 표시: [API 상태 스키마](schema-state.md).
- 지원되는 값과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류, 우선순위, 거절 응답 처리 경로: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md).
- 닫기 준비 상태 차단 사유 처리 경로: [API 차단 사유 처리 경로](blocker-routing.md).
- 저장 효과: [저장 효과](../storage-effects.md).
