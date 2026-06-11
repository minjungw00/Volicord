# 현재 MVP API

## 이 문서로 할 수 있는 일

이 참조 문서는 현재 MVP API 표면을 찾아볼 때 사용합니다. [API 값 집합](schema-value-sets.md)이 담당하는 활성 메서드 이름 집합에 대해, 메서드 수준 요청, 응답, 상태 효과, 저장소 담당 문서, 오류, 보안 경계 요약을 이 문서가 담당합니다.

이 파일은 현재 MVP의 모든 활성 메서드 동작을 현재 담당합니다. [현재 MVP API 메서드 분할 기준](../../maintain/authoring-guide.md#active-mvp-api-method-split-threshold)을 만족하면 메서드별 담당 문서를 만들어야 합니다.

이 문서는 계획을 위한 향후 하네스 서버 동작을 설명합니다. 이 저장소에는 현재 하네스 런타임이나 서버 구현이 없습니다. 향후 API 또는 스키마 후보는 이 활성 참조가 아니라 [이후 후보 색인](../../later/index.md)에 둡니다. 저장소 DDL과 전체 공통 스키마 본문은 이 메서드 참조 밖의 담당 문서가 소유합니다.

## 핵심 생각

현재 MVP API는 한 사용자 작업 루프를 위한 작은 로컬 MCP 표면입니다. 작업을 접수하고, 상태를 보여 주고, 활성 범위를 갱신하고, 제안된 제품 쓰기를 현재 Core 상태와 비교하며, Run과 증거 참조를 기록하고, 사용자 소유 판단을 묻고 기록하며, 활성 차단 사유가 허용할 때만 닫을 수 있습니다.

이 API는 협력형 하네스 기록/확인 동작만 반환합니다. 보안 비주장과 보장 표현은 [보안](../security.md)이 담당합니다.

요구사항 구체화는 아래 경로를 사용합니다.

- 활성 Task.
- Change Unit.
- `user_judgment`.
- 증거 요약.
- 차단 사유 경로.
- 다음 행동.
- 파생된 `ShapingReadiness` 보기.

비주장: API는 모호한 요청에서 안전한 첫 Change Unit으로 이동하기 위해 별도의 활성 커밋된 계획 아티팩트를 도입하지 않습니다. 해당 비주장에는 Discovery Brief, Question Queue, Assumption Register와 비슷한 아티팩트가 포함됩니다.

<a id="active-mvp-method-behavior"></a>

## 현재 MVP 메서드 동작

정확한 활성 메서드 이름 값 집합은 [API 값 집합](schema-value-sets.md)이 담당합니다. 이 페이지는 현재 메서드의 동작을 담당합니다.

| 메서드 | 활성 역할 |
|---|---|
| [`harness.intake`](#harnessintake) | 평소 사용자 작업을 시작, 재개, 분류합니다. |
| [`harness.status`](#harnessstatus) | 현재 상태 요약, 차단 사유, 대기 중인 판단, 증거 요약, 닫기 상태, 다음 안전한 행동을 반환합니다. |
| [`harness.update_scope`](#harnessupdate_scope) | `harness.intake` 이후 활성 Task 범위와 활성 Change Unit을 갱신합니다. |
| [`harness.prepare_write`](#harnessprepare_write) | 제안된 제품 파일 쓰기를 현재 범위, 상태, 필요한 별도 민감 동작 승인, 기준선, 접점 역량과 비교합니다. |
| [`harness.stage_artifact`](#harnessstage_artifact) | 호출자가 제공한 안전한 아티팩트 바이트 또는 안전한 알림을 나중에 `record_run`이 승격할 수 있는 임시 스테이징 핸들로 스테이징합니다. |
| [`harness.record_run`](#harnessrecord_run) | `shaping_update`, `direct`, `implementation` 종류의 작업과 간결한 증거/아티팩트 참조를 기록합니다. |
| [`harness.request_user_judgment`](#harnessrequest_user_judgment) | 대기 중인 사용자 소유 판단 요청 하나를 만듭니다. |
| [`harness.record_user_judgment`](#harnessrecord_user_judgment) | 기존 대기 중인 `UserJudgment`에 대한 사용자의 답을 기록합니다. |
| [`harness.close_task`](#harnessclose_task) | 닫기 준비 상태를 확인하고, 차단 사유가 허용할 때만 `complete`, `cancel`, `supersede` 값을 가진 `intent`를 처리합니다. |

이 문서는 메서드 역할과 메서드별 결과 동작을 이름 붙입니다. 분기, 저장 효과, `dry_run`, 재실행, 상태 버전 규칙의 기준 설명은 [API 코어 스키마](schema-core.md), [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md)를 확인하세요.

<a id="shared-request-rules"></a>

## 공통 요청 규칙

모든 메서드는 [`ToolEnvelope`](schema-core.md#tool-envelope)를 사용합니다.

각 공개 메서드 응답은 정확히 하나의 응답 분기를 가집니다.

- 구체적인 메서드별 `MethodResult`
- `ToolRejectedResponse`
- `ToolDryRunResponse`

메서드 결과:

- [`ToolResultBase`](schema-core.md#common-response)를 사용합니다.
- `response_kind=result`를 설정합니다.
- 실제 읽기 결과, 성공한 스테이징 결과, Core 커밋 결과, 또는 메서드 상태 효과 표가 허용하는 커밋된 차단 결과를 이름 붙입니다.

`ToolRejectedResponse`와 `ToolDryRunResponse`:

- [공통 응답 분기](schema-core.md#common-response)의 스키마를 사용합니다.
- 메서드별 result 전용 필드를 상속하지 않습니다.

예시 읽기 규칙:

- 아래 예시는 간결한 분기 예시이지 전체 스키마 정의가 아닙니다.
- 최소 요청 예시는 해당 메서드의 유효한 호출을 구성하는 데 필요한 필드를 포함합니다.
- 대표 응답 예시는 분기 이해에 중요한 필드를 보여 줍니다.
- 설명 중인 동작에 영향을 주지 않는 스키마 담당 중첩 필드는 생략할 수 있습니다.
- 전체 형태는 연결된 스키마 담당 문서를 사용합니다.

커밋되는 `dry_run=false` 상태 변경 호출의 조건:

- `idempotency_key`가 `null`이 아닙니다.
- `expected_state_version`이 현재 프로젝트 전체 상태 버전입니다.

예외:

- 읽기 전용 호출.
- 유효한 `dry_run` 미리보기.
- 스테이징 유틸리티 호출.

위 예외의 세부사항은 각 담당 문서가 정의합니다.

응답 분기 선택은 [공통 응답 분기](schema-core.md#common-response)가 담당합니다. 저장과 재실행 효과는 [저장 효과](../storage-effects.md)와 [저장소 버전 관리](../storage-versioning.md)가 담당합니다. 공개 오류, 오래된 상태 우선순위, 닫기 차단 사유 경로는 [API 오류](errors.md)가 담당합니다.

메서드에 도구별 `task_id`가 있으면 Core는 아래 순서로 기본 Task를 해석합니다.

1. 메서드 필드.
2. `ToolEnvelope.task_id`.
3. 활성 Task.

비주장: 이 해석은 담당 기록을 고르는 것이지 별도 상태 시계를 만들지 않습니다.

로컬 접근 등급의 경계:

- 결과: 로컬 접근 등급은 하네스 API 호환성 등급입니다.
- 비주장: OS 권한 등급이 아닙니다.
- 담당 문서: 활성 `access_class` 값은 [접근 등급 값](schema-value-sets.md#접근-등급-값)이 담당합니다.
- 담당 문서: 커넥터 도출과 역량 태세는 [에이전트 통합](../agent-integration.md)과 [보안](../security.md)이 담당합니다.

요청 수준 접근 등급 경계:

- 조건: 공개 API 요청 하나가 있습니다.
- 결과: 요청 수준 접근 등급은 정확히 하나입니다.
- 비주장: `ArtifactInput[]` 같은 중첩 페이로드는 두 번째 접근 등급을 추가하지 않습니다.
- 담당 문서: 아티팩트 스테이징, 승격, 본문 읽기 경계는 [API 아티팩트 스키마](schema-artifacts.md)와 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다.

<a id="harnessintake"></a>

## `harness.intake`

### 목적

평소 사용자 작업 루프를 시작, 재개, 대체, 거절하고 요청된 모드를 구체적인 `advisor`, `direct`, `work` Task 상태로 확정합니다. `harness.intake`는 쓰기 가능한 작업의 첫 범위 후보를 만들 수 있지만, 이후 범위 변경은 `harness.update_scope`가 담당합니다.

### 필수 입력

- `ToolEnvelope`: `project_id`, `surface_id`, `request_id`, `dry_run`이 필요하며, `dry_run=false` 커밋에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `plain_language_request`, `requested_mode`, `resume_policy`.
- 알고 있는 첫 범위 후보는 `initial_scope.boundary`, `initial_scope.non_goals`, `initial_scope.acceptance_criteria`에 둡니다. 목록 필드와 `initial_context_refs`에 알려진 항목이 없으면 빈 배열을 사용합니다.

### 접근 요구사항

조건:

- `dry_run=false` 커밋입니다.
- `VerifiedSurfaceContext.access_class=core_mutation`입니다.
- `verified=true`입니다.

비주장: `surface_id`는 등록된 로컬 접점을 고르는 선택자일 뿐, 그 자체가 권한이 아닙니다.

### 상태 버전 동작

커밋된 `dry_run=false` 결과:

- 프로젝트 전체 `project_state.state_version`을 정확히 한 번 올립니다.
- 멱등 키에 대한 재실행 행을 만듭니다.

아래 경우는 Task, Change Unit, 이벤트, 재실행 행, 차단 사유 갱신, 상태 버전 증가를 만들지 않습니다.

- `dry_run`.
- 읽기 실패.
- 검증 실패.
- 로컬 접근 실패.
- 오래된 `expected_state_version`.

### 성공 결과

`base.response_kind=result`, `base.effect_kind=core_committed`인 `IntakeResult`를 반환합니다. 결과에는 `task_ref`, 선택적 `change_unit_ref`, 현재 `state`, `next_actions`가 들어갑니다. `requested_mode=auto`라면 저장되고 표시되는 모드는 확정된 구체적 모드여야 하며 `auto`가 되면 안 됩니다.

### 차단 결과

이 메서드는 쓰기 준비 경로 대신 shaping 또는 차단 사유 상태를 기록하는 커밋된 `IntakeResult`를 반환할 수 있습니다.

차단 질문은 아래 필드로 표현해야 합니다.

- Task.
- Change Unit.
- 사용자 판단.
- 증거.
- 차단 사유.
- 다음 행동.

비주장: 별도 Discovery Brief, Question Queue, Assumption Register 아티팩트는 만들지 않습니다.

### 거절 결과

커밋 전 실패가 있으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- 검증 실패.
- 오래된 `expected_state_version`.
- Core 또는 로컬 접점 사용 불가.
- 로컬 접근 불일치.
- 활성 Task 호환성 부족.
- validator 실패.

공개 오류 코드 의미와 우선순위는 [API 오류](errors.md)가 담당합니다.

### `dry_run` 동작

`dry_run=true`에서 유효한 상태 효과 미리보기는 `IntakeResult`가 아니라 `ToolDryRunResponse`를 반환합니다. 분기 형태는 [API 코어 스키마](schema-core.md)가 담당하고, 저장 효과 없음 의미는 [저장 효과](../storage-effects.md)가 담당합니다.

### 저장 효과

커밋 시 `harness.intake`가 담당하는 Task 또는 Change Unit 상태를 지속할 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당하고, 저장 기록 형태는 [저장소 기록](../storage-records.md)이 담당합니다.

### 최소 유효 요청

```yaml
method: harness.intake
params:
  envelope:
    project_id: proj_123
    task_id: null
    actor_kind: agent
    surface_id: surface_local
    request_id: req_intake_001
    idempotency_key: idem_intake_001
    expected_state_version: 17
    dry_run: false
    locale: ko-KR
  plain_language_request: "계정 데이터 내보내기 전에 명시적 확인 단계를 추가한다."
  requested_mode: auto
  resume_policy: create_new
  initial_scope:
    boundary: "계정 데이터 내보내기 흐름과 계정 데이터 내보내기 확인 테스트만."
    non_goals:
      - "계정 삭제 동작"
    acceptance_criteria:
      - "계정 데이터 내보내기 전에 명시적 확인 단계가 필요하다."
  initial_context_refs: []
```

### 대표 응답

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
    - "계정 데이터 내보내기 전에 명시적 확인 단계가 필요하다."
  active_change_unit_ref: null
  blocker_refs: []
next_actions:
  - action: harness.update_scope
    reason: "쓰기 확인 전에 첫 활성 Change Unit을 만든다."
```

### 담당 문서 링크

- 요청 래퍼와 응답 분기: [`ToolEnvelope`](schema-core.md#tool-envelope), [공통 응답 분기](schema-core.md#common-response).
- 상태 참조, `StateSummary`, `ShapingReadiness`, 다음 행동: [API 상태 스키마](schema-state.md).
- 활성 메서드 이름, 모드 값, `resume_policy`, `response_kind`, `effect_kind`, 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류와 상태 버전 충돌: [API 오류](errors.md).
- 저장 효과: [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).

<a id="harnessupdate_scope"></a>

## `harness.update_scope`

### 목적

`harness.intake` 이후 활성 Task의 목표 요약, 범위 경계, 범위 밖 항목, 수락 기준, 자율성 경계, 기준선 참조, 활성 Change Unit을 갱신합니다. 사용자 소유 차단 사유가 처리되면 shaping 상태를 안전한 첫 Change Unit으로 옮기는 활성 경로입니다.

### 필수 입력

- `ToolEnvelope`: `dry_run=false` 커밋에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `task_id`.
- 바꿀 상위 범위 필드. `null`은 현재 값을 유지한다는 뜻이고, 빈 배열은 해당 목록을 빈 목록으로 교체합니다.
- `change_unit.operation`과 그 작업에 필요한 필드.
- 해결된 `judgment_kind=scope_decision`을 적용한다면 `related_scope_decision_refs`.

### 접근 요구사항

조건:

- `dry_run=false` 커밋입니다.
- `VerifiedSurfaceContext.access_class=core_mutation`입니다.
- `verified=true`입니다.
- 요청은 같은 프로젝트의 호환되는 Task를 식별합니다.
- 활성 Change Unit을 만들거나 교체할 때는 다음 안전한 행동을 정직하게 만들 만큼의 범위를 제공합니다.

### 상태 버전 동작

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

### 성공 결과

`base.response_kind=result`, `base.effect_kind=core_committed`인 `UpdateScopeResult`를 반환합니다. 결과에는 `task_ref`, 선택적 `change_unit_ref`, 연결된 `scope_decision` 참조, `status=stale` 쓰기 승인 참조, 차단 사유 참조, 현재 `state`, `next_actions`가 들어갑니다.

### 차단 결과

범위가 아직 준비되지 않았을 때 메서드가 소유한 차단 사유 또는 현재 행 갱신을 커밋할 수 있습니다.

커밋된 차단 범위 결과는 필요한 사용자 소유 판단 범주를 식별해야 합니다.

- `product_decision`.
- `technical_decision`.
- `scope_decision`.
- `sensitive_approval`.

비주장: 필요한 판단을 막연한 모호함 뒤에 숨기면 안 됩니다.

### 거절 결과

커밋 전 실패가 있으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- 오래된 `expected_state_version`.
- 유효하지 않은 Task 식별.
- 유효하지 않은 Change Unit 작업.
- 필요한 범위 누락.
- 범위 위반.
- 미해결 필수 판단.
- 자율성 경계 위반.
- 기준선이 오래되었습니다.
- 로컬 접근 실패.
- validator 실패.

공개 오류 코드 의미와 우선순위는 [API 오류](errors.md)가 담당합니다.

### `dry_run` 동작

`dry_run=true`에서 유효한 상태 효과 미리보기는 `ToolDryRunResponse`를 반환합니다. 분기 형태는 [API 코어 스키마](schema-core.md)가 담당하고, 저장 효과 없음 의미는 [저장 효과](../storage-effects.md)가 담당합니다.

### 저장 효과

커밋 시 범위 담당 현재 상태와 `status=stale` 승인 결과를 지속할 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당합니다.

### 최소 유효 요청

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
  scope_boundary: "계정 데이터 내보내기 명시적 확인 단계를 추가한다. 계정 데이터 내보내기 확인 테스트를 갱신한다."
  non_goals:
    - "계정 삭제 동작"
  acceptance_criteria:
    - "계정 데이터 내보내기 전에 명시적 확인 단계가 필요하다."
  autonomy_boundary: "계정 데이터 내보내기 명시적 확인 단계와 계정 데이터 내보내기 확인 테스트 범위 안에서만 작업한다."
  baseline_ref: baseline_account_export_001
  change_unit:
    operation: create_active
    scope_summary: "계정 데이터 내보내기 명시적 확인 단계를 추가하고 계정 데이터 내보내기 확인 테스트를 갱신한다."
    affected_areas:
      - "계정 데이터 내보내기 명시적 확인 단계"
      - "계정 데이터 내보내기 확인 테스트"
    affected_paths:
      - src/account/export.ts
      - src/account/export-confirmation.ts
      - tests/account-export.test.ts
    constraints:
      - "계정 삭제 동작은 범위에서 제외한다."
  related_scope_decision_refs: []
```

### 대표 응답

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
  scope_summary: "계정 데이터 내보내기 명시적 확인 단계를 추가한다. 계정 데이터 내보내기 확인 테스트를 갱신한다."
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

### 담당 문서 링크

- 요청 래퍼와 응답 분기: [API 코어 스키마](schema-core.md).
- 상태 참조, `StateSummary`, `ShapingReadiness`, 차단 사유, 다음 행동: [API 상태 스키마](schema-state.md).
- 범위 관련 사용자 판단 형태: [API 판단 스키마](schema-judgment.md).
- 활성 값 집합과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류: [API 오류](errors.md).
- 저장 효과와 `status=stale` 쓰기 승인 동작: [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).

<a id="harnessstatus"></a>

## `harness.status`

### 목적

Core 상태의 읽기 전용 현재 위치 보기를 반환합니다. 활성 Task 요약, 차단 사유, 대기 중인 사용자 판단, 쓰기 승인 요약, 증거 요약, 닫기 상태, 닫기 준비 상태 발견 사항, 보장 표시, 다음 안전한 행동을 포함할 수 있습니다.

### 필수 입력

- `ToolEnvelope`: `project_id`, `surface_id`, `request_id`, `dry_run`이 필요합니다. `idempotency_key`와 `expected_state_version`은 `null`일 수 있습니다.
- 호출자가 필요한 요약을 고르는 `include` 플래그.

### 접근 요구사항

조건:

- 보호된 Core 세부정보를 반환합니다.
- 같은 프로젝트의 활성 로컬 접점이 있습니다.
- `VerifiedSurfaceContext.access_class=read_status`입니다.

비주장: 오래된 상태 보기, 대화 요약, 생성된 Markdown 파일, 캐시된 텍스트는 상태 권한 근거가 아닙니다.

### 상태 버전 동작

상태 변경은 없고 `project_state.state_version`을 올리지 않습니다.

결과:

- 현재 관찰된 상태 버전을 보고할 수 있습니다.

비주장:

- 이벤트를 만들지 않습니다.
- 재실행 행을 만들지 않습니다.
- 닫기 변경을 만들지 않습니다.
- 아티팩트 효과를 만들지 않습니다.
- 스테이징 핸들을 소비하지 않습니다.
- 증거를 갱신하지 않습니다.
- 쓰기 승인을 변경하지 않습니다.

### 성공 결과

`base.response_kind=result`, `base.effect_kind=read_only`인 `StatusResult`를 반환합니다. `include.close=true`일 때 `StatusResult.close_blockers`는 읽기 전용 관찰인 `CloseReadinessBlocker[]`입니다. 저장된 `close_task` 결과가 아닙니다.

### 차단 결과

커밋된 차단 분기는 없습니다. `StatusResult`의 차단 사유와 닫기 차단 사유는 계산된 응답 필드일 뿐입니다.

### 거절 결과

읽기를 안전하게 제공할 수 없으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- Core 사용 불가.
- 로컬 접근 불일치.
- 요청한 보호 세부정보에 대한 역량 부족.
- Task 범위 읽기에 필요한 활성 Task 없음.
- 요청한 상태 보기가 오래되었거나 사용 불가.

공개 오류 코드 의미와 우선순위는 [API 오류](errors.md)가 담당합니다.

### `dry_run` 동작

이 읽기 전용 메서드에서는 `dry_run=true`가 `ToolDryRunResponse` 분기를 만들지 않습니다. 유효한 요청은 같은 `StatusResult` 형태를 반환하며 `base.dry_run=true`, `base.effect_kind=read_only`를 사용합니다. 분기 규칙은 [API 코어 스키마](schema-core.md)가 담당합니다.

### 저장 효과

이 메서드는 읽기 전용입니다. 정확한 저장 효과 없음 의미는 [저장 효과](../storage-effects.md)가 담당합니다.

### 최소 유효 요청

```yaml
method: harness.status
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_status_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: ko-KR
  include:
    task: true
    pending_user_judgments: true
    write_authority: true
    evidence: true
    close: true
    guarantees: true
```

### 대표 응답

결과 분기(`StatusResult`, 읽기 전용):

```yaml
base:
  response_kind: result
  effect_kind: read_only
  dry_run: false
  state_version: 19
  events: []
active_task:
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
  scope_summary: "계정 데이터 내보내기 명시적 확인 단계를 추가하고 계정 데이터 내보내기 확인 테스트를 갱신한다."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_001
    project_id: proj_123
    task_id: task_456
    state_version: 19
status_summary: "계정 데이터 내보내기 확인 테스트가 기록되었습니다. 계정 데이터 내보내기 명시적 확인 단계 문구에 대한 사용자 수락은 아직 대기 중입니다."
next_actions:
  - action: harness.request_user_judgment
    reason: "닫기 전에 계정 데이터 내보내기 명시적 확인 단계 문구에 대한 사용자 판단을 요청합니다."
pending_user_judgments: []
write_authority_summary: null
evidence_summary:
  status: sufficient
  coverage_items:
    - claim: "계정 데이터 내보내기 확인 테스트가 통과했습니다."
      required_for_close: true
      coverage_state: supported
      supporting_refs:
        - record_kind: run
          record_id: run_account_export_tests_001
          project_id: proj_123
          task_id: task_456
          state_version: 21
      supporting_artifact_refs: []
      gap_refs: []
  artifact_refs: []
blocker_refs: []
close_readiness:
  ready: false
  blockers:
    - code: missing_user_judgment
      message: "사용자가 계정 데이터 내보내기 명시적 확인 단계 문구를 아직 수락하지 않았습니다."
guarantee_display:
  level: cooperative
  notes:
    - "더 강한 로컬 보장이 활성화되지 않았습니다."
```

### 담당 문서 링크

- 요청 래퍼와 응답 분기: [API 코어 스키마](schema-core.md).
- 상태, 닫기 준비 상태 형태, 증거 요약, 보장 표시: [API 상태 스키마](schema-state.md).
- 활성 값과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류와 닫기 차단 사유 경로: [API 오류](errors.md), [`close_task` 차단 사유 매핑](errors.md#harnessclose_task-close-blockers).
- 저장 효과: [저장 효과](../storage-effects.md).

<a id="harnessprepare_write"></a>

## `harness.prepare_write`

### 목적

제안된 제품 파일 쓰기 하나를 아래 항목과 비교합니다.

- 현재 Task.
- 활성 Change Unit.
- 범위.
- 기준선.
- 필요한 별도 민감 동작 승인.
- 확인된 로컬 접점 역량.

결과:

- 허용되면 소비 가능한 단일 사용 쓰기 승인(`Write Authorization`)을 만듭니다.
- 허용되지 않으면 그 쓰기 승인 경로를 거부하거나 미룹니다.

보안 비주장은 [보안](../security.md)이 담당합니다.

### 필수 입력

- `ToolEnvelope`: `dry_run=false` 커밋에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `task_id`와 `change_unit_id`. 담당 해석이 활성 Task와 활성 Change Unit을 모호하지 않게 사용할 수 있을 때만 `null`을 사용할 수 있습니다.
- `intended_operation`, `intended_paths`, `product_file_write_intended`, `sensitive_categories`, `baseline_ref`.

### 접근 요구사항

조건:

- `VerifiedSurfaceContext.access_class=write_authorization`입니다.
- `verified=true`입니다.
- 호환되는 활성 범위가 있습니다.
- 기준선이 호환됩니다.
- 필요한 사용자 소유 판단이 처리되어 있습니다.
- 필요한 경우 별도 `sensitive_approval`이 있습니다.
- 의도한 제품 파일 쓰기 확인에 필요한 로컬 접점 역량이 있습니다.

### 상태 버전 동작

| 결과 | 상태 버전 효과 | 쓰기 승인 효과 |
|---|---|---|
| 커밋된 `decision=allowed` | `project_state.state_version`을 정확히 한 번 올립니다. | 경로 수준 `AuthorizedAttemptScope`에 대한 활성 쓰기 승인 하나를 만듭니다. |
| 커밋된 `decision=blocked`, `decision=approval_required`, `decision=decision_required` | 메서드가 소유한 쓰기 결정 이유 상태를 저장하기 위해서만 올릴 수 있습니다. | 소비 가능한 쓰기 승인을 만들면 안 됩니다. |
| 커밋 전 거절 또는 `dry_run` | 올리지 않습니다. | 만들지 않습니다. |

### 성공 결과

`PrepareWriteResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=core_committed`

`decision=allowed`일 때:

- `write_authorization_ref`는 `null`이 아닙니다.
- `write_authorization`은 `null`이 아닙니다.
- `authorization_effect`는 새 커밋에서 `created`, 멱등 재실행에서 `returned`입니다.

### 차단 결과

커밋된 차단 결정은 아래 `decision` 값 중 하나를 가진 `PrepareWriteResult`입니다.

조건:

- `write_decision_reasons`는 비어 있으면 안 됩니다.

비주장:

- `write_decision_reasons`는 `CloseReadinessBlocker` 값이 아닙니다.
- 쓰기 결정 이유는 닫기 준비 상태를 평가하지 않습니다.
- 소비 가능한 쓰기 승인은 만들어지지 않습니다.

### 거절 결과

`decision` 평가나 커밋 전에 실패가 있으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- 오래된 `expected_state_version`.
- 멱등 요청 해시 충돌.
- 요청 검증 실패.
- 활성 Task 또는 Change Unit 없음.
- 로컬 접근 실패.
- Core 사용 불가.
- 기준선이 오래되었습니다.
- 유효하지 않은 요청 보장.
- 역량 실패.

비주장: `STATE_VERSION_CONFLICT`는 항상 거절 응답 오류이며 쓰기 결정 이유가 아닙니다.

### `dry_run` 동작

`dry_run=true`에서 유효한 미리보기는 `ToolDryRunResponse`를 반환합니다. 분기 형태는 [API 코어 스키마](schema-core.md)가 담당하고, 저장 효과 없음 의미는 [저장 효과](../storage-effects.md)가 담당합니다.

### 저장 효과

커밋 시 메서드 결과에 따라 쓰기 승인 또는 쓰기 결정 상태를 지속할 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당합니다.

### 최소 유효 요청

```yaml
method: harness.prepare_write
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_prepare_001
    idempotency_key: idem_prepare_001
    expected_state_version: 19
    dry_run: false
    locale: ko-KR
  task_id: task_456
  change_unit_id: cu_001
  intended_operation: "계정 데이터 내보내기 명시적 확인 단계 갱신"
  intended_paths:
    - src/account/export.ts
    - src/account/export-confirmation.ts
    - tests/account-export.test.ts
  product_file_write_intended: true
  sensitive_categories: []
  baseline_ref: baseline_account_export_001
```

### 대표 응답

필요한 승인이 이미 있을 때의 허용 분기(`PrepareWriteResult`, `decision=allowed`):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 20
  events:
    - event_id: evt_1003
      event_kind: write_authorization_created
decision: allowed
state:
  project_id: proj_123
  state_version: 20
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 20
write_authorization_ref:
  record_kind: write_authorization
  record_id: wa_001
  project_id: proj_123
  task_id: task_456
  state_version: 20
write_authorization:
  authorization_id: wa_001
  status: active
  basis_state_version: 19
  authorized_paths:
    - src/account/export.ts
    - src/account/export-confirmation.ts
    - tests/account-export.test.ts
authorization_effect: created
active_user_judgment_refs: []
write_decision_reasons: []
user_judgment_candidate: null
guarantee_display:
  level: cooperative
  notes:
    - "쓰기 승인(`Write Authorization`)은 하네스 호환성 기록이며 OS 권한이 아닙니다."
```

승인이 없을 때의 승인 필요 분기 발췌:

```yaml
decision: approval_required
write_authorization_ref: null
write_authorization: null
authorization_effect: none
write_decision_reasons:
  - code: sensitive_export_flow
    message: "계정 데이터 내보내기는 개인정보를 포함할 수 있으므로 명시적 확인 단계에 대한 승인이 필요합니다."
```

### 담당 문서 링크

- 요청 래퍼, 공통 결과 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `WriteAuthorizationSummary`, 상태 요약, 참조: [API 상태 스키마](schema-state.md).
- `SensitiveActionScope`와 사용자 소유 승인 경계: [API 판단 스키마](schema-judgment.md).
- 활성 값과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류, `STATE_VERSION_CONFLICT`, 차단/`dry_run` 동작: [API 오류](errors.md).
- 저장 효과와 상태 시계: [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).

<a id="harnessstage_artifact"></a>

## `harness.stage_artifact`

### 목적

호출자가 제공한 안전한 아티팩트 바이트 또는 안전한 알림을 같은 프로젝트와 Task에 대한 임시 `StagedArtifactHandle`로 스테이징합니다.

결과:

- 스테이징은 입력 준비일 뿐입니다.

비주장:

- 기준 증거를 만들지 않습니다.
- 지속 `ArtifactRef`를 만들지 않습니다.
- 관문 충족을 만들지 않습니다.
- 최종 수락을 만들지 않습니다.
- 잔여 위험 수락을 만들지 않습니다.
- 닫기 준비 상태를 만들지 않습니다.

### 필수 입력

- `ToolEnvelope`: `project_id`, `task_id`, `surface_id`, `request_id`, `dry_run`이 필요합니다. `idempotency_key`와 `expected_state_version`은 `null`일 수 있습니다.
- `task_id`, `display_name`, `content_type`, `redaction_state`, `safe_bytes_or_notice`, `expected_sha256`, `expected_size_bytes`, `relation_hint`.

### 접근 요구사항

조건:

- `VerifiedSurfaceContext.access_class=artifact_registration`입니다.
- `verified=true`입니다.
- `project_id`와 `task_id`가 호환됩니다.
- `manual_artifact_attachment_supported=true`입니다.

결과:

- 향후 서버는 확인된 로컬 접점에서 `created_by_surface_id`와 `created_by_surface_instance_id`를 기록합니다.

비주장:

- 호출자는 이 값을 권한 근거로 제출하지 않습니다.

### 상태 버전 동작

성공한 스테이징 결과의 효과:

- Core 상태를 바꾸지 않습니다.
- `project_state.state_version`을 올리지 않습니다.
- `tool_invocations` 재실행 행을 만들지 않습니다.

비주장: 거절과 `dry_run` 요청은 저장 효과가 없습니다.

### 성공 결과

`base.response_kind=result`, `base.effect_kind=staging_created`인 `StageArtifactResult`를 반환합니다. 결과에는 임시 `staged_artifact_handle`과 `expires_at`이 들어갑니다. 지속 `ArtifactRef`는 포함하지 않습니다.

### 차단 결과

커밋된 차단 분기는 없습니다.

- 유효하지 않은 스테이징 요청은 Core 변경 전에 거절됩니다.
- 스테이징 가용성이나 역량 문제는 차단 사유를 만들지 않습니다.

### 거절 결과

아래 경우는 `ToolRejectedResponse`를 반환합니다.

- 유효하지 않은 요청 형태.
- 체크섬 또는 크기 불일치.
- 안전하지 않은 아티팩트 입력.
- 지원하지 않는 가림 처리 상태.
- Core 또는 로컬 접점 사용 불가.
- 로컬 접근 불일치.
- 아티팩트 등록 역량 부족.

공개 오류 코드 의미와 우선순위는 [API 오류](errors.md)가 담당합니다.

### `dry_run` 동작

`dry_run=true`에서 유효한 스테이징 미리보기는 `StageArtifactResult`가 아니라 `ToolDryRunResponse`를 반환합니다. 분기 형태는 [API 코어 스키마](schema-core.md)가 담당하고, 스테이징 효과 없음 의미는 [저장 효과](../storage-effects.md)와 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다.

### 저장 효과

성공 시 임시 스테이징 결과만 만듭니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당하고, 아티팩트 생명주기 세부사항은 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다.

### 아티팩트 데이터 예시

스테이징할 아티팩트는 안정적인 제품 테스트 출력입니다. 임시 스테이징 핸들은 나중에 `harness.record_run`에 제출할 수 있지만, 스테이징만으로 정식 증거가 생기지는 않습니다.

```yaml
artifact:
  kind: test_log
  name: account_export_confirmation_test.log
  description: "계정 데이터 내보내기 확인 테스트 출력."
staged_artifact_handle: staged_artifact_account_export_test_log_001
```

### 최소 유효 요청

```yaml
method: harness.stage_artifact
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_stage_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: ko-KR
  task_id: task_456
  display_name: "account_export_confirmation_test.log"
  content_type: text/plain
  redaction_state: none
  safe_bytes_or_notice: "계정 데이터 내보내기 확인 테스트 출력."
  expected_sha256: null
  expected_size_bytes: null
  relation_hint: "test_log"
```

### 대표 응답

결과 분기(`StageArtifactResult`, 스테이징 생성):

```yaml
base:
  response_kind: result
  effect_kind: staging_created
  dry_run: false
  state_version: null
  events: []
staged_artifact_handle:
  handle_id: staged_artifact_account_export_test_log_001
  project_id: proj_123
  task_id: task_456
  created_by_surface_id: surface_local
  created_by_surface_instance_id: surface_instance_01
  content_type: text/plain
  sha256: sha256:example
  size_bytes: 65
  redaction_state: none
  expires_at: "2026-06-10T12:30:00Z"
  consumed: false
expires_at: "2026-06-10T12:30:00Z"
```

### 담당 문서 링크

- 요청 래퍼, 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `StagedArtifactHandle`, `ArtifactInput`, `ArtifactRef`: [API 아티팩트 스키마](schema-artifacts.md).
- 활성 아티팩트 값과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류: [API 오류](errors.md).
- 저장 효과와 아티팩트 생명주기: [저장 효과](../storage-effects.md), [아티팩트 저장소](../storage-artifacts.md).

<a id="harnessrecord_run"></a>

## `harness.record_run`

### 목적

`harness.record_run`은 아래 작업을 기록합니다.

- `shaping_update`.
- `direct`.
- `implementation`.

추가 결과:

- 간결한 증거 범위를 갱신합니다.
- 제품 쓰기를 기록할 때 호환되는 쓰기 승인을 소비합니다.
- 기존 아티팩트를 연결합니다.
- 허용되는 경우 적격 스테이징 핸들을 지속 `ArtifactRef`로 승격합니다.

### 필수 입력

- `ToolEnvelope`: `dry_run=false` 커밋에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `task_id`, `change_unit_id`, `kind`, `run_id`, `baseline_ref`, `write_authorization_id`, `summary`, `observed_changes`, `artifact_inputs`, `evidence_updates`.
- 제품 쓰기 Run은 `harness.prepare_write`가 만든 호환되는 활성 쓰기 승인이 필요합니다.
- 새 아티팩트 바이트는 이미 유효한 `StagedArtifactHandle`로 표현되어 있어야 합니다. `record_run`은 새 바이트를 스테이징하지 않습니다.

### 접근 요구사항

조건:

- `VerifiedSurfaceContext.access_class=run_recording`입니다.
- `verified=true`입니다.
- `source_kind=staged_artifact`에서는 현재 확인된 `surface_id`와 `surface_instance_id`가 스테이징 핸들의 기록된 출처와 일치해야 합니다.

비주장:

- `ArtifactInput[]`는 `artifact_registration`을 추가하지 않습니다.
- 현재 MVP에는 접점 간 스테이징 핸들 인계가 없습니다.

### 상태 버전 동작

호환되는 커밋 결과:

- `project_state.state_version`을 정확히 한 번 올립니다.

제품 쓰기 기록이 활성 쓰기 승인을 소비하려면 아래 조건을 모두 만족해야 합니다.

- 현재 상태 버전이 승인 기준 상태와 여전히 맞습니다.
- 관찰된 변경 경로가 승인된 시도와 호환됩니다.

예외:

- 오래된 `expected_state_version`은 소비 전에 거절됩니다.
- 승인 기준 상태가 오래되었으면 소비 전에 거절됩니다.

### 성공 결과

`base.response_kind=result`, `base.effect_kind=core_committed`인 `RecordRunResult`를 반환합니다. 결과에는 `run_summary`, `registered_artifacts`, 갱신된 `evidence_summary`, `blocker_refs`, 현재 `state`가 들어갑니다.

### 차단 결과

Run 자체는 기록 가능하지만 결과가 증거 공백 같은 차단 사유를 만들거나 유지할 때 호환되는 Run 관련 차단 사유 상태를 커밋할 수 있습니다.

비주장: 아래 실패를 숨기기 위해 커밋된 차단 결과를 사용하면 안 됩니다.

- 유효하지 않은 스테이징 핸들.
- 누락된 쓰기 승인.
- 상태가 오래되었습니다.
- 승인 기준 상태가 오래되었습니다.
- 로컬 접근 실패.

위 경우는 커밋 전에 거절됩니다.

### 거절 결과

아래 경우는 `ToolRejectedResponse`를 반환합니다.

- 오래된 `expected_state_version`.
- 쓰기 승인 기준 상태가 오래되었습니다.
- 제품 쓰기에 필요한 쓰기 승인 누락 또는 무효.
- 유효하지 않은 스테이징 핸들.
- 스테이징 핸들 출처 불일치.
- 누락된 아티팩트.
- 범위 위반.
- 기준선이 오래되었습니다.
- 로컬 접근 실패.
- 역량 부족.
- validator 실패.

비주장: 유효하지 않은 스테이징 핸들은 아티팩트 입력 세부정보가 있는 검증 실패입니다. 요청 수준 로컬 접근 자체가 실패한 경우가 아니라면 로컬 접근 불일치가 아닙니다.

### `dry_run` 동작

`dry_run=true`에서 유효한 미리보기는 `ToolDryRunResponse`를 반환합니다. 분기 형태는 [API 코어 스키마](schema-core.md)가 담당하고, 저장 및 승격 효과 없음 의미는 [저장 효과](../storage-effects.md)와 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다.

### 저장 효과

커밋 시 Run, 증거, 차단 사유, 쓰기 승인 소비, 아티팩트 연결 결과를 지속할 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당하고, 아티팩트 승격 세부사항은 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다.

### 실행 데이터 예시

이 실행은 제품 테스트 실행을 기록하며, 스테이징된 테스트 로그를 증거로 소비할 수 있습니다.

```yaml
command: "npm test -- account-export"
summary: "계정 데이터 내보내기 확인 테스트가 통과했습니다."
artifacts:
  - staged_artifact_account_export_test_log_001
run_ref: run_account_export_tests_001
```

### 최소 유효 요청

```yaml
method: harness.record_run
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_run_001
    idempotency_key: idem_run_001
    expected_state_version: 20
    dry_run: false
    locale: ko-KR
  task_id: task_456
  change_unit_id: cu_001
  kind: implementation
  run_id: null
  baseline_ref: baseline_account_export_001
  write_authorization_id: null
  summary: "계정 데이터 내보내기 확인 테스트가 통과했습니다."
  observed_changes:
    changed_paths: []
    product_file_write_observed: false
    sensitive_categories: []
    baseline_ref: baseline_account_export_001
  artifact_inputs:
    - artifact_input_id: artifact_input_account_export_test_log_001
      source_kind: staged_artifact
      staged_artifact_handle:
        handle_id: staged_artifact_account_export_test_log_001
        project_id: proj_123
        task_id: task_456
        created_by_surface_id: surface_local
        created_by_surface_instance_id: surface_instance_01
        content_type: text/plain
        sha256: sha256:example
        size_bytes: 65
        redaction_state: none
        expires_at: "2026-06-10T12:30:00Z"
        consumed: false
      existing_artifact_ref: null
      relation_hint: "test_log"
      claim: "계정 데이터 내보내기 확인 테스트 출력."
      expected_sha256: null
      expected_size_bytes: null
      redaction_state: none
  evidence_updates:
    - claim: "계정 데이터 내보내기 확인 테스트가 통과했습니다."
      required_for_close: true
      coverage_state: supported
      supporting_refs: []
      supporting_artifact_refs: []
      gap_refs: []
```

### 대표 응답

결과 분기(`RecordRunResult`, 커밋됨):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 21
  events:
    - event_id: evt_1004
      event_kind: run_recorded
run_summary:
  run_ref:
    record_kind: run
    record_id: run_account_export_tests_001
    project_id: proj_123
    task_id: task_456
    state_version: 21
  kind: implementation
  summary: "계정 데이터 내보내기 확인 테스트가 통과했습니다."
  observed_changes:
    changed_paths: []
    product_file_write_observed: false
    sensitive_categories: []
    baseline_ref: baseline_account_export_001
  artifact_refs:
    - artifact_id: artifact_account_export_test_log_001
      project_id: proj_123
      task_id: task_456
      display_name: "account_export_confirmation_test.log"
      content_type: text/plain
      sha256: sha256:example
      size_bytes: 65
      redaction_state: none
      availability: available
      created_by_run_ref:
        record_kind: run
        record_id: run_account_export_tests_001
        project_id: proj_123
        task_id: task_456
        state_version: 21
      created_by_surface_id: surface_local
      created_by_surface_instance_id: surface_instance_01
      storage_ref: artifact://artifact_account_export_test_log_001
registered_artifacts:
  - artifact_id: artifact_account_export_test_log_001
    project_id: proj_123
    task_id: task_456
    display_name: "account_export_confirmation_test.log"
    content_type: text/plain
    sha256: sha256:example
    size_bytes: 65
    redaction_state: none
    availability: available
    created_by_run_ref:
      record_kind: run
      record_id: run_account_export_tests_001
      project_id: proj_123
      task_id: task_456
      state_version: 21
    created_by_surface_id: surface_local
    created_by_surface_instance_id: surface_instance_01
    storage_ref: artifact://artifact_account_export_test_log_001
evidence_summary:
  status: sufficient
  coverage_items:
    - claim: "계정 데이터 내보내기 확인 테스트가 통과했습니다."
      required_for_close: true
      coverage_state: supported
      supporting_refs:
        - record_kind: run
          record_id: run_account_export_tests_001
          project_id: proj_123
          task_id: task_456
          state_version: 21
      supporting_artifact_refs:
        - artifact_id: artifact_account_export_test_log_001
          project_id: proj_123
          task_id: task_456
          display_name: "account_export_confirmation_test.log"
          content_type: text/plain
          sha256: sha256:example
          size_bytes: 65
          redaction_state: none
          availability: available
          created_by_run_ref:
            record_kind: run
            record_id: run_account_export_tests_001
            project_id: proj_123
            task_id: task_456
            state_version: 21
          created_by_surface_id: surface_local
          created_by_surface_instance_id: surface_instance_01
          storage_ref: artifact://artifact_account_export_test_log_001
      gap_refs: []
  artifact_refs:
    - artifact_id: artifact_account_export_test_log_001
      project_id: proj_123
      task_id: task_456
      display_name: "account_export_confirmation_test.log"
      content_type: text/plain
      sha256: sha256:example
      size_bytes: 65
      redaction_state: none
      availability: available
      created_by_run_ref:
        record_kind: run
        record_id: run_account_export_tests_001
        project_id: proj_123
        task_id: task_456
        state_version: 21
      created_by_surface_id: surface_local
      created_by_surface_instance_id: surface_instance_01
      storage_ref: artifact://artifact_account_export_test_log_001
blocker_refs: []
state:
  project_id: proj_123
  state_version: 21
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 21
```

### 담당 문서 링크

- 요청 래퍼, 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `RunSummary`, `EvidenceSummary`, `EvidenceCoverageItem`, `StateSummary`, 참조: [API 상태 스키마](schema-state.md).
- `ArtifactInput`, `StagedArtifactHandle`, `ArtifactRef`: [API 아티팩트 스키마](schema-artifacts.md).
- 쓰기 승인과 닫기 관련 증거 경계: [Core 모델](../core-model.md).
- 활성 값과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류: [API 오류](errors.md).
- 저장 효과와 아티팩트 승격: [저장 효과](../storage-effects.md), [아티팩트 저장소](../storage-artifacts.md).

<a id="harnessrequest_user_judgment"></a>

## `harness.request_user_judgment`

### 목적

초점이 분명한 사용자 소유 결정 하나에 대해 대기 중인 `UserJudgment`를 만듭니다.

결과:

- 이 메서드는 사용자에게 묻는 경로입니다.

비주장:

- 에이전트가 사용자를 대신해 답하지 않습니다.
- 에이전트가 사용자를 대신해 추론하지 않습니다.
- 에이전트가 질문 범위를 넓히지 않습니다.
- 에이전트가 결정을 내리지 않습니다.

### 필수 입력

- `ToolEnvelope`: `dry_run=false` 커밋에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `task_id`, `change_unit_id`, `judgment_kind`, `presentation`, `question`, `options`, `context`, `affected_refs`, `required_for`, `expires_at`.
- 사용자가 정확한 사안을 판단할 수 있도록 초점이 분명한 질문, 이해 가능한 선택지, 충분한 맥락.

### 접근 요구사항

`VerifiedSurfaceContext.access_class=core_mutation`과 `verified=true`가 필요합니다. 요청은 같은 프로젝트의 호환되는 Task와 선택적 Change Unit을 대상으로 해야 합니다.

### 상태 버전 동작

커밋된 `dry_run=false` 결과:

- `project_state.state_version`을 정확히 한 번 올립니다.
- 대기 중인 판단을 만듭니다.

비주장:

- 다른 메서드가 반환한 후보는 이 메서드가 커밋하기 전까지 지속 기록이 아닙니다.
- `dry_run`과 거절은 대기 중인 판단, 차단 사유 갱신, 이벤트, 재실행 행, 상태 버전 증가를 만들지 않습니다.

### 성공 결과

`base.response_kind=result`, `base.effect_kind=core_committed`인 `RequestUserJudgmentResult`를 반환합니다. 결과에는 `user_judgment_ref`, 대기 중인 `user_judgment`, 영향을 받은 `blocker_refs`, 현재 `state`가 들어갑니다.

### 차단 결과

별도 커밋된 차단 응답 분기는 없습니다. 요청이 유효하지 않거나 선행조건을 확인할 수 없어 판단을 만들 수 없으면 메서드는 커밋 전에 거절합니다.

### 거절 결과

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

### `dry_run` 동작

`dry_run=true`에서 유효한 미리보기는 `ToolDryRunResponse`를 반환합니다. 분기 형태는 [API 코어 스키마](schema-core.md)가 담당하고, 저장 효과 없음 의미는 [저장 효과](../storage-effects.md)가 담당합니다.

### 저장 효과

커밋 시 대기 중인 판단과 관련 차단 사유 상태를 지속할 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당합니다.

### 최소 유효 요청

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
  question: "개인정보를 포함할 수 있는 계정 데이터 내보내기의 명시적 확인 단계 문구가 충분합니까?"
  options:
    - option_id: accept
      label: "충분함"
      description: "확인 문구가 충분하다는 사용자의 판단을 기록합니다."
      consequence: "닫기 준비 상태 평가에서 제품 판단을 해결된 것으로 볼 수 있습니다."
      is_default: true
    - option_id: revise
      label: "수정"
      description: "확인 문구 수정을 위해 Task를 열어 둡니다."
      consequence: "제품 판단 때문에 닫기가 계속 차단됩니다."
      is_default: false
  context:
    summary: "개인정보를 포함할 수 있는 계정 데이터 내보내기의 명시적 확인 단계 문구가 충분한지는 사용자가 판단해야 합니다."
    related_refs: []
    artifact_refs:
      - artifact_id: artifact_account_export_confirmation_copy_001
        project_id: proj_123
        task_id: task_456
        display_name: "account_export_confirmation_copy.txt"
        content_type: text/plain
        sha256: sha256:example
        size_bytes: 65
        redaction_state: none
        availability: available
        created_by_run_ref:
          record_kind: run
          record_id: run_account_export_tests_001
          project_id: proj_123
          task_id: task_456
          state_version: 21
        created_by_surface_id: surface_local
        created_by_surface_instance_id: surface_instance_01
        storage_ref: artifact://artifact_account_export_confirmation_copy_001
    visible_risks: []
    constraints:
      - "현재 Task 제약이 적용됩니다"
  affected_refs:
    - record_kind: task
      record_id: task_456
      project_id: proj_123
      task_id: task_456
      state_version: 21
  required_for: close
  expires_at: null
```

### 대표 응답

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
  question: "개인정보를 포함할 수 있는 계정 데이터 내보내기의 명시적 확인 단계 문구가 충분합니까?"
  options: []
  context:
    summary: "개인정보를 포함할 수 있는 계정 데이터 내보내기의 명시적 확인 단계 문구가 충분한지는 사용자가 판단해야 합니다."
    related_refs: []
    artifact_refs:
      - artifact_id: artifact_account_export_confirmation_copy_001
        project_id: proj_123
        task_id: task_456
        display_name: "account_export_confirmation_copy.txt"
        content_type: text/plain
        sha256: sha256:example
        size_bytes: 65
        redaction_state: none
        availability: available
        created_by_run_ref:
          record_kind: run
          record_id: run_account_export_tests_001
          project_id: proj_123
          task_id: task_456
          state_version: 21
        created_by_surface_id: surface_local
        created_by_surface_instance_id: surface_instance_01
        storage_ref: artifact://artifact_account_export_confirmation_copy_001
    visible_risks: []
    constraints:
      - "현재 Task 제약이 적용됩니다"
  affected_refs: []
  required_for: close
  resolution: null
  expires_at: null
  created_at: "2026-06-10T12:00:00Z"
  resolved_at: null
blocker_refs: []
state:
  project_id: proj_123
  state_version: 22
```

### 담당 문서 링크

- 요청 래퍼, 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `UserJudgment`, 선택지, 맥락, 답변 페이로드: [API 판단 스키마](schema-judgment.md).
- 상태 참조와 요약: [API 상태 스키마](schema-state.md).
- 판단 종류와 활성 값: [API 값 집합](schema-value-sets.md).
- 사용자 소유 판단과 비대체 규칙: [Core 모델](../core-model.md).
- 공개 오류와 저장 효과: [API 오류](errors.md), [저장 효과](../storage-effects.md).

<a id="harnessrecord_user_judgment"></a>

## `harness.record_user_judgment`

### 목적

기존 대기 중인 `UserJudgment` 하나에 대한 사용자의 답을 기록합니다.

결과:

- 사용자의 답에 따라 특정 대기 판단을 `resolved`, `rejected`, `deferred`, `blocked` 또는 해당 상태로 표시합니다.

비주장:

- 답변을 관련 없는 승인으로 넓히지 않습니다.
- 답변을 범위 확장으로 넓히지 않습니다.
- 답변을 수락이나 잔여 위험 수락으로 넓히지 않습니다.
- 답변을 쓰기 승인으로 넓히지 않습니다.

### 필수 입력

- `ToolEnvelope`: `dry_run=false` 커밋에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `user_judgment_id`, 일치하는 `judgment_kind`, `selected_option_id`, `answer`, `note`, `accepted_risks`.
- `answer`에는 대기 중인 `judgment_kind`에 맞는 결정별 페이로드 분기만 담아야 합니다. `selected_option_id`와 `note`는 요청 수준에 남습니다.

### 접근 요구사항

`VerifiedSurfaceContext.access_class=core_mutation`과 `verified=true`가 필요합니다. 대기 중인 판단은 요청이 선택한 같은 프로젝트와 호환되는 Task에 속해야 합니다.

### 상태 버전 동작

커밋된 `dry_run=false` 결과:

- `project_state.state_version`을 정확히 한 번 올립니다.
- 지정된 `user_judgments` 행을 갱신합니다.

비주장:

- `dry_run`과 거절은 판단 해결, 차단 사유 갱신, 이벤트, 재실행 행, 상태 버전 증가를 만들지 않습니다.

### 성공 결과

`base.response_kind=result`, `base.effect_kind=core_committed`인 `RecordUserJudgmentResult`를 반환합니다. 결과에는 `user_judgment_ref`, 갱신된 `user_judgment`, `updated_refs`, 현재 `state`, `next_actions`가 들어갑니다.

### 차단 결과

사용자의 답이 그렇거나 초점이 맞는 판단의 호환 결과가 그렇다면 지정된 판단은 `rejected`, `deferred`, `blocked` 또는 차단 사유를 만드는 상태로 커밋될 수 있습니다.

결과:

- 포함된 차단 사유와 판단에 의존하는 요약만 갱신합니다.

비주장:

- 해결된 `scope_decision`만으로 활성 범위나 활성 Change Unit 필드가 바뀌지 않습니다.
- 해당 필드를 바꾸려면 여전히 `harness.update_scope`가 필요합니다.

### 거절 결과

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

### `dry_run` 동작

`dry_run=true`에서 유효한 미리보기는 `ToolDryRunResponse`를 반환합니다. 분기 형태는 [API 코어 스키마](schema-core.md)가 담당하고, 저장 효과 없음 의미는 [저장 효과](../storage-effects.md)가 담당합니다.

### 저장 효과

커밋 시 판단 해결과 그에 따른 차단 사유 또는 요약 상태를 지속할 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당합니다.

### 최소 유효 요청

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
        rationale: "확인 문구가 내보내기에 개인정보가 포함될 수 있음을 명확히 알립니다."
    technical_decision: null
    scope_decision: null
    sensitive_action_scope: null
    final_acceptance: null
    residual_risk_acceptance: null
    cancellation: null
  note: null
  accepted_risks: []
```

### 대표 응답

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
  question: "개인정보를 포함할 수 있는 계정 데이터 내보내기의 명시적 확인 단계 문구가 충분합니까?"
  options: []
  context:
    summary: "개인정보를 포함할 수 있는 계정 데이터 내보내기의 명시적 확인 단계 문구가 충분한지는 사용자가 판단해야 합니다."
    related_refs: []
    artifact_refs:
      - artifact_id: artifact_account_export_confirmation_copy_001
        project_id: proj_123
        task_id: task_456
        display_name: "account_export_confirmation_copy.txt"
        content_type: text/plain
        sha256: sha256:example
        size_bytes: 65
        redaction_state: none
        availability: available
        created_by_run_ref:
          record_kind: run
          record_id: run_account_export_tests_001
          project_id: proj_123
          task_id: task_456
          state_version: 21
        created_by_surface_id: surface_local
        created_by_surface_instance_id: surface_instance_01
        storage_ref: artifact://artifact_account_export_confirmation_copy_001
    visible_risks: []
    constraints: []
  affected_refs: []
  required_for: close
  resolution:
    selected_option_id: accept
    answer:
      product_decision:
        judgment:
          decision: accepted
          rationale: "확인 문구가 내보내기에 개인정보가 포함될 수 있음을 명확히 알립니다."
    note: null
    accepted_risks: []
    resolved_by_actor_kind: user
  expires_at: null
  created_at: "2026-06-10T12:00:00Z"
  resolved_at: "2026-06-10T12:05:00Z"
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

### 담당 문서 링크

- 요청 래퍼, 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `UserJudgment`, `RecordUserJudgmentPayload`, `SensitiveActionScope`, `AcceptedRiskInput`: [API 판단 스키마](schema-judgment.md).
- 상태 참조와 요약: [API 상태 스키마](schema-state.md).
- 판단 값과 활성 메서드 내부 값: [API 값 집합](schema-value-sets.md).
- 사용자 소유 판단, 최종 수락, 잔여 위험 수락, 비대체 규칙: [Core 모델](../core-model.md).
- 공개 오류와 저장 효과: [API 오류](errors.md), [저장 효과](../storage-effects.md).

<a id="harnessclose_task"></a>

## `harness.close_task`

### 목적

활성 Task의 닫기 준비 상태를 평가합니다.

조건:

- 선택한 `intent`가 허용됩니다.
- 차단 사유가 없습니다.

결과:

- `complete`, `cancel`, `supersede`를 커밋할 수 있습니다.
- `harness.close_task`는 닫기 차단 사유를 반환할 수 있습니다.

비주장:

- 닫기는 Core 상태 전이이며 보고서가 아닙니다.
- 대화, 상태 텍스트, 최종 수락만, 잔여 위험 수락만, 증거만, 렌더링된 보기에서 닫기를 추론하지 않습니다.

### 필수 입력

- `ToolEnvelope`: `project_id`, `surface_id`, `request_id`, `dry_run`이 필요합니다.
- `task_id`, `intent`, `close_reason`, `superseding_task_id`, `user_note`.
- `intent=complete`, `intent=cancel`, `intent=supersede`와 `dry_run=false`에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `intent=check`에서는 `idempotency_key`와 `expected_state_version`이 `null`일 수 있고, `close_reason`은 `null`이어야 합니다.

### 접근 요구사항

| `intent` 종류 | 조건 |
|---|---|
| `intent=check` | 보호된 닫기 준비 상태 세부정보를 위해 `VerifiedSurfaceContext.access_class=read_status`가 필요합니다. |
| 상태 변경 `intent` | `VerifiedSurfaceContext.access_class=core_mutation`, `verified=true`, 호환되는 Task 식별, 유효한 생명주기, 닫기 관련 담당 기록이 필요합니다. |

### 상태 버전 동작

| 경우 | 상태 버전 효과 |
|---|---|
| `intent=check` | `dry_run=true`여도 항상 읽기 전용이며 상태를 올리지 않습니다. |
| 상태 변경 `intent`의 커밋된 종료 닫기 또는 커밋된 차단 닫기 | `project_state.state_version`을 정확히 한 번 올립니다. |
| 닫기 사전 확인 거절, 오래된 `expected_state_version`, 닫기 관련 `WriteAuthorization.basis_state_version` 오래됨, 멱등 요청 해시 충돌, `dry_run` 미리보기 | 아무것도 올리지 않습니다. |

### 성공 결과

`base.response_kind=result`인 `CloseTaskResult`를 반환합니다.

| 경우 | 효과 | `close_state` |
|---|---|---|
| `intent=check` | `base.effect_kind=read_only` | 계산된 현재 닫기 상태. |
| 성공한 종료 상태 변경 | `base.effect_kind=core_committed` | `closed`, `cancelled`, `superseded` 중 하나. |

### 차단 결과

조건:

- 닫기 사전 확인이 성공했습니다.
- `intent=complete`입니다.

결과:

- `blockers: CloseReadinessBlocker[]`를 가진 `CloseTaskResult(close_state=blocked)`를 반환할 수 있습니다.
- 상태 변경 `intent`는 메서드 상태 효과 표가 그 커밋된 차단 결과를 허용할 때만 차단 사유 상태 효과를 저장할 수 있습니다.

비주장:

- `CloseReadinessBlocker`가 있다는 사실만으로 저장을 뜻하지 않습니다.
- `STATE_VERSION_CONFLICT`는 절대 `CloseReadinessBlocker.code`가 아닙니다.

### 거절 결과

닫기 준비 상태 평가 전 사전 확인 실패가 있으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- 검증 실패.
- 로컬 접근 실패.
- 오래된 `expected_state_version`.
- 닫기 관련 `WriteAuthorization.basis_state_version` 오래됨.
- 멱등 요청 해시 충돌.
- 잘못된 프로젝트 또는 읽을 수 없는 Task 식별.
- Core 사용 불가.
- 역량 부족.

비주장:

- 거절 응답은 `CloseTaskResult.blockers`를 반환하지 않습니다.
- 거절 응답은 닫기 효과를 만들지 않습니다.

### `dry_run` 동작

`intent=check`와 `dry_run=true`는 읽기 전용 `CloseTaskResult` 분기에 남습니다. 상태 변경 `intent`의 `dry_run=true`는 유효할 때 공통 미리보기 분기를 사용합니다. 분기 형태와 계획 차단 사유 표현은 [API 코어 스키마](schema-core.md)와 [API 오류](errors.md)가 담당합니다.

### 저장 효과

`intent=check`에는 저장 효과가 없습니다. 상태 변경 닫기 `intent`는 메서드 결과에 따라 닫기 또는 차단 결과를 지속할 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당합니다.

### 닫기 준비 상태 시나리오 데이터

리터럴 `intent=complete`는 완료 의도를 고르는 API 값입니다. 전체 닫기 준비 상태 평가 순서를 뜻하는 산문 표현이 아닙니다.

계정 데이터 내보내기 명시적 확인 단계 시나리오에서 성공한 닫기 준비 상태 관찰 예시는 아래와 같습니다.

```yaml
close_readiness:
  ready: true
  evidence:
    - "계정 데이터 내보내기 확인 테스트가 통과했습니다."
    - "사용자가 계정 데이터 내보내기 명시적 확인 단계 문구를 수락했습니다."
```

같은 시나리오에서 차단된 닫기 준비 상태 관찰 예시는 아래와 같습니다.

```yaml
close_readiness:
  ready: false
  blockers:
    - code: missing_user_judgment
      message: "사용자가 계정 데이터 내보내기 명시적 확인 단계 문구를 아직 수락하지 않았습니다."
```

### 최소 유효 요청

```yaml
method: harness.close_task
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_close_check_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: ko-KR
  task_id: task_456
  intent: check
  close_reason: null
  superseding_task_id: null
  user_note: null
```

### 대표 응답

차단된 읽기 전용 결과 분기(`CloseTaskResult`, `intent=check`):

```yaml
base:
  response_kind: result
  effect_kind: read_only
  dry_run: false
  state_version: 23
  events: []
close_state: blocked
state:
  project_id: proj_123
  state_version: 23
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 23
blockers:
  - category: user_judgment
    code: missing_user_judgment
    message: "사용자가 계정 데이터 내보내기 명시적 확인 단계 문구를 아직 수락하지 않았습니다."
    related_refs: []
evidence_summary:
  status: sufficient
  coverage_items:
    - claim: "계정 데이터 내보내기 확인 테스트가 통과했습니다."
      required_for_close: true
      coverage_state: supported
      supporting_refs:
        - record_kind: run
          record_id: run_account_export_tests_001
          project_id: proj_123
          task_id: task_456
          state_version: 21
      supporting_artifact_refs: []
      gap_refs: []
  artifact_refs: []
artifact_refs: []
next_actions:
  - action: harness.request_user_judgment
    reason: "닫기를 시도하기 전에 계정 데이터 내보내기 명시적 확인 단계 문구에 대한 사용자 판단을 요청한다."
```

### 담당 문서 링크

- 요청 래퍼, 공통 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- 닫기 준비 상태 형태, `CloseReadinessBlocker`, `EvidenceSummary`, `StateSummary`: [API 상태 스키마](schema-state.md).
- 닫기 상태, 생명주기, 닫기 이유, 차단 사유 값: [API 값 집합](schema-value-sets.md).
- 전체 닫기 준비 상태 평가 순서와 정직한 닫기: [Core 모델의 닫기 준비 상태](../core-model.md#close_task).
- 공개 오류와 닫기 차단 사유 경로: [API 오류](errors.md), [`close_task` 차단 사유 매핑](errors.md#harnessclose_task-close-blockers).
- 저장 효과와 상태 버전 동작: [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).
