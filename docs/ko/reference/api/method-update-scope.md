<a id="harnessupdate_scope"></a>

# `harness.update_scope` 참조

## 담당하는 것

이 문서는 기준 범위의 `harness.update_scope` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- `harness.intake` 이후 범위와 Change Unit을 갱신하는 동작
- 범위 갱신 예시

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
- `change_unit.operation`과 그 작업에 필요한 필드. 지원되는 작업 값과 그 의미는 [API 값 집합](schema-value-sets.md#method-local-values)이 담당합니다.
- 해결된 `judgment_kind=scope_decision`을 적용한다면 `related_scope_decision_refs`.

## 요청 스키마

이 메서드는 아래 최상위 `params` 요청 형태를 담당합니다. `envelope`는 [API 코어 스키마](schema-core.md#tool-envelope)의 공통 `ToolEnvelope`이며, 이 블록은 `ToolEnvelope` 필드를 다시 정의하지 않습니다.

```yaml
UpdateScopeRequest:
  envelope: ToolEnvelope
  task_id: string
  goal_summary: string | null
  scope_update: object | null
  scope_boundary: string | null
  non_goals: string[] | null
  acceptance_criteria: string[] | null
  autonomy_boundary: string | null
  baseline_ref: string | null
  change_unit: object
  related_scope_decision_refs: StateRecordRef[]
```

중첩 형태 담당 문서:
- `related_scope_decision_refs`는 `StateRecordRef[]`를 사용합니다. 중첩 형태는 [API 상태 스키마](schema-state.md)가 담당합니다.
- `change_unit.operation` 값은 [API 값 집합의 메서드 내부 값](schema-value-sets.md#method-local-values)이 담당합니다.

## 접근 요구사항

커밋되는 `dry_run`이 아닌 요청에는 아래 조건이 필요합니다.

- `access_class=core_mutation`인 서버 파생 `VerifiedSurfaceContext`
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

## 메서드 결과 필드

`UpdateScopeResult`는 성공적으로 커밋된 범위 갱신에 대한 메서드별 결과 분기입니다. 이 결과는 `base: ToolResultBase`와 아래 메서드 소유 최상위 필드를 담습니다.

| 필드 | 결과 필드 의미 |
|---|---|
| `base` | 공통 결과 메타데이터입니다. `events`를 포함한 `ToolResultBase` 형태는 [API 코어 스키마](schema-core.md#common-response)가 담당합니다. `base.events[].event_kind`가 있을 때 그 값은 불투명한 예시용 분류 문자열입니다. |
| `task_ref` | 범위 결과가 갱신한 `Task`의 `StateRecordRef`입니다. |
| `change_unit_ref` | 작업 뒤 현재 적용 Change Unit의 `StateRecordRef | null`입니다. 현재 적용 Change Unit이 없으면 `null`입니다. |
| `linked_scope_decision_refs` | 갱신에 적용된 `scope_decision` 사용자 판단의 `StateRecordRef[]`입니다. |
| `stale_write_authorization_refs` | 커밋된 갱신 때문에 오래된 상태가 된 `Write Authorization` 기록의 `StateRecordRef[]`입니다. 저장 효과와 버전 관리는 지속 세부사항을 담당합니다. |
| `blocker_refs` | 메서드가 소유하며 갱신에서 커밋했거나 계속 관련되는 차단 사유의 `StateRecordRef[]`입니다. |
| `state` | 범위 갱신 뒤의 현재 `StateSummary`입니다. 현재 적용 범위와 현재 적용 Change Unit 표시 필드를 포함합니다. |
| `next_actions` | 다음 안전한 API 단계를 설명하는 `NextActionSummary[]`입니다. |

지원되는 `change_unit.operation` 값은 [API 값 집합](schema-value-sets.md#method-local-values)이 담당합니다. 이 메서드는 각 작업이 `change_unit_ref`, `state.active_change_unit_ref`, 오래된 `Write Authorization` 참조, 차단 사유 참조, `next_actions`에 어떻게 반영되는지를 담당합니다.

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

커밋 시 범위 담당 현재 상태와 오래된 `Write Authorization` 처리 결과를 지속할 수 있습니다. 정확한 저장 효과는 아래 저장 담당 문서가 담당합니다.

아래 예시는 메서드 안에서만 성립하도록 짧게 구성했습니다. 대표 응답은 범위 갱신 결과 분기, 참조, 상태 버전, 현재 적용 범위, 현재 적용 Change Unit, 생명주기, 다음 행동을 보여 주는 데 필요한 필드로 축약했습니다.

메서드 안의 전제: `task_filter_001`은 `proj_filter_001`에 `state_version: 18`로 이미 있으며, 알맞은 현재 적용 Change Unit이 없습니다. 이 요청은 `cu_filter_001`을 현재 적용 Change Unit으로 만듭니다.

## 최소 유효 요청

```yaml
method: harness.update_scope
params:
  envelope:
    project_id: proj_filter_001
    task_id: task_filter_001
    actor_kind: agent
    surface_id: surface_scope
    request_id: req_scope_filter_001
    idempotency_key: idem_scope_filter_001
    expected_state_version: 18
    dry_run: false
    locale: en-US
  task_id: task_filter_001
  goal_summary: "Limit saved search filters to owner and label fields."
  scope_update:
    include:
      - "Constrain saved-filter edits to owner and label fields."
      - "Update saved-filter validation tests."
    exclude:
      - "Search indexing behavior."
  scope_boundary: "Saved-filter owner and label edits plus related tests."
  non_goals:
    - "Search indexing behavior."
  acceptance_criteria:
    - "Saved filters reject changes outside owner and label fields."
  autonomy_boundary: "Stay within saved-filter edit validation and related tests."
  baseline_ref: baseline_filter_001
  change_unit:
    operation: create_current
    scope_summary: "Saved-filter owner and label edit validation."
    affected_areas:
      - "Saved-filter edit form"
      - "Saved-filter validation tests"
    affected_paths:
      - src/search/saved-filter.ts
      - src/search/filter-form.ts
      - tests/saved-filter.test.ts
    constraints:
      - "Leave search indexing behavior out of scope."
  related_scope_decision_refs: []
```

## 대표 응답

축약한 결과 분기(`UpdateScopeResult`, 커밋됨):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 19
  events:
    - event_id: evt_filter_001
      event_kind: scope_updated
task_ref:
  record_kind: task
  record_id: task_filter_001
  project_id: proj_filter_001
  task_id: task_filter_001
  state_version: 19
change_unit_ref:
  record_kind: change_unit
  record_id: cu_filter_001
  project_id: proj_filter_001
  task_id: task_filter_001
  state_version: 19
linked_scope_decision_refs: []
stale_write_authorization_refs: []
blocker_refs: []
state:
  project_id: proj_filter_001
  state_version: 19
  task_ref:
    record_kind: task
    record_id: task_filter_001
    project_id: proj_filter_001
    task_id: task_filter_001
    state_version: 19
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Limit saved search filters to owner and label fields."
  scope_summary: "Saved-filter owner and label edit validation."
  non_goals:
    - "Search indexing behavior."
  acceptance_criteria:
    - "Saved filters reject changes outside owner and label fields."
  autonomy_boundary: "Stay within saved-filter edit validation and related tests."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_filter_001
    project_id: proj_filter_001
    task_id: task_filter_001
    state_version: 19
  baseline_ref: baseline_filter_001
  shaping_readiness: null
  pending_user_judgment_refs: []
  blocker_refs: []
  write_authority_summary: null
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display: null
next_actions:
  - action_kind: prepare_write
    owner_method: harness.prepare_write
    label: "Check the saved-filter change against current scope."
    blocking_question: null
    required_refs:
      - record_kind: task
        record_id: task_filter_001
        project_id: proj_filter_001
        task_id: task_filter_001
        state_version: 19
      - record_kind: change_unit
        record_id: cu_filter_001
        project_id: proj_filter_001
        task_id: task_filter_001
        state_version: 19
```

## 담당 문서 링크

- 요청 래퍼와 응답 분기: [API 코어 스키마](schema-core.md).
- 상태 참조, `StateSummary`, `ShapingReadiness`, 차단 사유, 다음 행동: [API 상태 스키마](schema-state.md).
- 범위 관련 사용자 판단 형태: [API 판단 스키마](schema-judgment.md).
- 지원되는 값 집합, `change_unit.operation` 의미, 접근 등급: [API 값 집합](schema-value-sets.md#method-local-values), [접근 등급 값](schema-value-sets.md#access-class-values).
- 공개 오류, 우선순위, 거절 응답 처리 경로: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md).
- 저장 효과와 오래된 `Write Authorization` 동작: [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).
