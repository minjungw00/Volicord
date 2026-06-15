<a id="harnessintake"></a>

# `harness.intake` 참조

## 담당하는 것

이 문서는 기준 범위의 `harness.intake` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 사용자 작업 루프를 시작, 재개, 대체, 거절하는 intake 처리
- intake 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, `dry_run`, 거절 응답 스키마 본문
- 상태, 아티팩트, 판단, 값 집합, 오류의 중첩 스키마 정의
- 저장 DDL, 저장 기록 레이아웃, 정확한 저장 효과, 아티팩트 생명주기, 보안 보장, Core 권한 의미
- 공개 오류 코드 의미, 공개 오류 우선순위, 공통 응답 분기 처리 경로

## 목적

`harness.intake`는 일반 사용자 작업 루프를 시작, 재개, 대체, 거절합니다.

이 메서드는 요청된 모드를 구체적인 `Task` 모드로 확정합니다.

- `advisor`
- `direct`
- `work`

범위 경계:

- `harness.intake`는 쓰기 가능한 작업의 첫 범위 후보를 만들 수 있습니다.
- 이후 범위 변경은 `harness.update_scope`가 담당합니다.

## 필수 입력

- 유효한 `ToolEnvelope`. 커밋되는 `dry_run`이 아닌 요청에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `plain_language_request`, `requested_mode`, `resume_policy`.
- 알고 있는 첫 범위 후보는 `initial_scope.boundary`, `initial_scope.non_goals`, `initial_scope.acceptance_criteria`에 둡니다. 알려진 목록 항목이 없으면 빈 배열을 사용합니다.

## 접근 요구사항

커밋되는 `dry_run`이 아닌 요청에는 아래 조건이 필요합니다.

- `VerifiedSurfaceContext.access_class=core_mutation`
- `verified=true`

접점 식별 경계:

- `surface_id`는 등록된 로컬 접점을 고르는 선택자일 뿐, 그 자체가 권한이 아닙니다.

## 상태 버전 동작

커밋된 `dry_run`이 아닌 결과:

- 프로젝트 전체 `project_state.state_version`을 정확히 한 번 올립니다.
- 멱등 키에 대한 재실행 행을 만듭니다.

아래 경우는 `Task`, Change Unit, 이벤트, 재실행 행, 차단 사유 갱신, 상태 버전 증가를 만들지 않습니다.

- `dry_run`
- 읽기 실패
- 검증 실패
- 로컬 접근 실패
- 오래된 `expected_state_version`

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

- `Task` 또는 Change Unit 상태
- 사용자 판단, 증거, 차단 사유, 다음 행동 필드
- 중첩 필드 형태를 담당하는 아래 스키마 문서

## 거절 결과

커밋 전 실패가 있으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- 검증 실패
- 오래된 `expected_state_version`
- Core 또는 로컬 접점 사용 불가
- 로컬 접근 불일치
- 현재 `Task` 호환성 누락
- 검증기 실패

공개 오류 코드 의미, 우선순위, 거절 응답 처리 경로는 아래 오류 담당 문서가 담당합니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 상태 효과 미리보기:

- `ToolDryRunResponse`를 반환합니다.
- `IntakeResult`를 반환하지 않습니다.
- 지속되는 intake 상태를 만들지 않습니다.

## 저장 효과

커밋 시 `harness.intake`가 담당하는 `Task` 또는 Change Unit 상태를 지속할 수 있습니다. 정확한 저장 효과와 저장 기록 형태는 아래 저장 담당 문서가 담당합니다.

아래 예시는 메서드 안에서만 성립하도록 짧게 구성했습니다. 대표 응답은 intake 분기, 참조, 상태 버전, 생명주기, 현재 적용 범위, 현재 적용 Change Unit, 다음 행동을 보여 주는 데 필요한 필드로 축약했습니다.

## 최소 유효 요청

```yaml
method: harness.intake
params:
  envelope:
    project_id: proj_onboard_001
    task_id: null
    actor_kind: agent
    surface_id: surface_onboard
    request_id: req_intake_onboard_001
    idempotency_key: idem_intake_onboard_001
    expected_state_version: 17
    dry_run: false
    locale: en-US
  plain_language_request: "Create a first-run checklist for new workspace setup."
  requested_mode: work
  resume_policy: create_new
  initial_scope:
    boundary: "First-run checklist for new workspace setup."
    non_goals:
      - "Changing account creation."
    acceptance_criteria:
      - "New users see the checklist after opening a workspace."
  initial_context_refs: []
```

## 대표 응답

축약한 결과 분기(`IntakeResult`, 커밋됨):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 18
  events:
    - event_id: evt_onboard_001
      event_kind: task_intake
task_ref:
  record_kind: task
  record_id: task_onboard_001
  project_id: proj_onboard_001
  task_id: task_onboard_001
  state_version: 18
change_unit_ref: null
state:
  project_id: proj_onboard_001
  state_version: 18
  task_ref:
    record_kind: task
    record_id: task_onboard_001
    project_id: proj_onboard_001
    task_id: task_onboard_001
    state_version: 18
  mode: work
  lifecycle:
    lifecycle_phase: shaping
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Create a first-run checklist for new workspace setup."
  scope_summary: "First-run checklist for new workspace setup."
  non_goals:
    - "Changing account creation."
  acceptance_criteria:
    - "New users see the checklist after opening a workspace."
  autonomy_boundary: null
  active_change_unit_ref: null
  baseline_ref: null
  shaping_readiness: null
  pending_user_judgment_refs: []
  blocker_refs: []
  write_authority_summary: null
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display: null
next_actions:
  - action_kind: update_scope
    owner_method: harness.update_scope
    label: "Create the first currently applied Change Unit before write checking."
    blocking_question: null
    required_refs:
      - record_kind: task
        record_id: task_onboard_001
        project_id: proj_onboard_001
        task_id: task_onboard_001
        state_version: 18
```

## 담당 문서 링크

- 요청 래퍼와 응답 분기: [`ToolEnvelope`](schema-core.md#tool-envelope), [공통 응답 분기](schema-core.md#common-response).
- 상태 참조, `StateSummary`, `ShapingReadiness`, 다음 행동: [API 상태 스키마](schema-state.md).
- 지원되는 메서드 이름, 모드 값, `resume_policy`, `response_kind`, `effect_kind`, 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류, 우선순위, 거절 응답 처리 경로: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md).
- 저장 효과와 저장 기록: [저장 효과](../storage-effects.md), [저장소 기록](../storage-records.md), [저장소 버전 관리](../storage-versioning.md).
