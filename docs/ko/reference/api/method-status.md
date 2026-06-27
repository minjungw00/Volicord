<a id="volicordstatus"></a>

# `volicord.status` 참조

## 담당하는 것

이 문서는 기준 범위의 `volicord.status` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 현재 Core 상태에 대한 읽기 전용 상태 조회 동작
- 상태 조회 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, `dry_run`, 거절 응답 스키마 본문
- 상태, 아티팩트, 판단, 값 집합, 오류의 중첩 스키마 정의
- 저장 DDL, 저장 기록 레이아웃, 정확한 저장 효과, 아티팩트 생명주기, 보안 보장, Core 권한 의미
- 공개 오류 코드 의미, 공개 오류 우선순위, 공통 응답 분기 처리 경로

## 목적

`volicord.status`는 Core 상태의 읽기 전용 현재 위치 보기를 반환합니다. 현재 `Task` 요약, 차단 사유, 대기 중인 사용자 판단, `Write Check` 요약, 증거 요약, 닫기 상태, 닫기 준비 상태 발견 사항, 프로젝트 연속성 요약, 보장 표시, 다음 안전한 행동을 포함할 수 있습니다.

## 필수 입력

- 유효한 `ToolEnvelope`. `idempotency_key`와 `expected_state_version`은 `null`일 수 있습니다.
- 호출자가 필요한 요약을 고르는 `include` 플래그.

## 요청 스키마

이 메서드는 아래 최상위 `params` 요청 형태를 담당합니다. `envelope`는 [API 코어 스키마](schema-core.md#tool-envelope)의 공통 `ToolEnvelope`이며, 이 블록은 `ToolEnvelope` 필드를 다시 정의하지 않습니다.

이 메서드 소유 요청 블록에 표시된 모든 필드는 필드 참고가 명시적으로 선택 필드라고 표시하지 않는 한 `params`의 필수 멤버입니다. `T | null`은 멤버가 반드시 있어야 하며 JSON `null`을 담을 수 있다는 뜻입니다.

```yaml
StatusRequest:
  envelope: ToolEnvelope
  include: object
```

필드 참고:
- `include`는 상태 조회 요약을 고르는 메서드 내부 플래그 객체이며, 최소 유효 요청 예시에 표시되어 있습니다.

## 접근 요구사항

보호된 Core 세부정보를 요청할 때 읽기에는 아래 조건이 필요합니다.

- 같은 프로젝트의 확인된 호출 맥락
- `operation_category=read`

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
- `Write Check` 변경

## 성공 결과

아래 값을 담은 `StatusResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=read_only`

`include.close=true`일 때 `StatusResult.close_blockers`는 읽기 전용 관찰인 `CloseReadinessBlocker[]`입니다.

비주장: `StatusResult.close_blockers`는 저장된 `close_task` 결과가 아닙니다.

`include` 상태 보기 계약:

- `include.task`는 선택된 `Task` 요약과 현재 Change Unit을 `active_task`로 반환합니다.
- `include.pending_user_judgments`는 현재 대기 판단 참조를 반환하며, 관련 있는 오래됨 또는 대체됨 판단 상태는 `blocker_refs`, `next_actions.required_refs` 같은 기존 결과 필드로 나타납니다.
- `include.write_check`는 `Write Check`(쓰기 확인)의 Core 상태 호환성 기록에서 활성, 만료, 오래됨, 소비됨 또는 그 밖의 관련 상태를 `write_check_summary`로 반환합니다.
- `write_check_summary`는 호환성 요약일 뿐이며 파일시스템 접근, 셸 승인, 최종 수락, 일반 쓰기 승인이 아닙니다.
- `include.evidence`는 사용할 수 있을 때 현재 `EvidenceSummary`와 범위를 반환합니다.
- `include.close`는 `CurrentCloseBasis | null`, 닫기 상태, 계산된 차단 사유, 위험 수락 범위, 관련 다음 행동을 반환합니다. 차단 사유는 `volicord.close_task intent=check`와 같은 닫기 준비 상태 계산을 사용합니다.
- `include.guarantees`는 프로젝트 강제 프로필, 확인된 호출 맥락, 활성화된 강제 메커니즘, 지원되는 기준 범위에서 파생된 보장만 반환합니다.
- `include.continuity`는 오래 유지하는 프로젝트 수준 맥락의 활성 `ProjectContinuitySummary[]` 항목을 반환합니다.
- `include.evidence=false`는 증거 요약, 범위, 아티팩트 증거 참조, 증거 전용 다음 행동을 계산하지도 반환하지도 않는다는 뜻입니다.
- `include.close=false`는 닫기 준비 상태를 계산하지 않고 `CurrentCloseBasis`, 닫기 상태, 닫기 차단 사유, 잔여 위험 범위, 닫기 전용 다음 행동을 반환하지 않는다는 뜻입니다.
- `include.guarantees=false`는 보장 표시를 파생하지도 반환하지도 않는다는 뜻입니다.
- `include.continuity=false`는 프로젝트 연속성 요약을 읽거나 반환하지 않는다는 뜻입니다.

정직한 상태 보기 규칙:
- 계산하지 않았거나, 선택하지 않은 데이터는 스키마가 허용하는 곳에서 생략합니다. 선택된 상태 보기를 계산했지만 사용할 수 없을 때만 `null`을 사용합니다. "계산했고 없음"을 암시하는 빈 값으로 표현하면 안 됩니다.
- 닫기 차단 사유의 빈 배열을 포함한 빈 배열은 메서드가 그 필드를 계산했고 항목이 없었다는 뜻입니다.
- 호스트 지침, 연결 모드, 생성된 텍스트만으로는 보장이 생기지 않습니다. 협력형 전용 배포는 `detective`를 주장하면 안 됩니다.
- `GuaranteeDisplay.capability_refs`는 해당 참조를 사용할 수 있을 때 호출 바인딩, Agent Connection, 관찰 사실을 식별해야 합니다.

`include.close=true`와 [`volicord.close_task`](method-close-task.md)의 `intent=check`는 같은 닫기 준비 상태 계산을 사용합니다. `volicord.status`는 읽기 전용으로 남으며 재실행 행, 이벤트, 상태 변경, 닫기 변경, 상태 버전 증가를 만들지 않습니다.

## 메서드 결과 필드

`StatusResult`는 성공적인 상태 조회에 대한 메서드별 결과 분기입니다. 이 결과는 `base: ToolResultBase`와 아래 메서드 소유 최상위 필드를 담습니다.

| 필드 | 결과 필드 의미 |
|---|---|
| `base` | 공통 결과 메타데이터입니다. `ToolResultBase` 형태는 [API 코어 스키마](schema-core.md#common-response)가 담당합니다. 읽기 전용 상태 조회 결과는 `events: []`를 사용합니다. 공통 응답 분기에 `EventRef.event_kind`가 있을 때 그 값은 불투명한 예시용 분류 문자열로 남습니다. |
| `active_task` | 현재 선택된 `Task` 요약의 `StateSummary | null`입니다. |
| `status_summary` | 현재 상태 조회 보기를 요약하는 자유 형식 표시 문자열입니다. 닫기 준비 상태 보기가 선택되면 현재 닫기 준비 상태나 첫 번째 닫기 차단 사유 코드를 요약할 수 있습니다. 구조화된 권한 사실은 다른 결과 필드에 남습니다. |
| `next_actions` | 다음 안전한 API 단계를 설명하는 `NextActionSummary[]`입니다. |
| `pending_user_judgments` | 상태 조회 보기에 선택된 대기 중 사용자 판단 기록의 `StateRecordRef[]`입니다. |
| `blocker_refs` | 현재 상태 조회 보기에 보이는 차단 사유 기록의 `StateRecordRef[]`입니다. |
| `close_state` | 현재 보기의 닫기 상태 값입니다. 현재 닫기 상태가 없을 때의 `none`을 포함한 지원 값은 [API 값 집합](schema-value-sets.md#task-lifecycle-values)이 담당합니다. |
| `current_close_basis` | 닫기 상태 조회 보기에 선택된 `CurrentCloseBasis | null`입니다. 형태는 [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)가 담당합니다. |
| `risk_acceptance_coverage` | 닫기 상태 조회 보기에서 현재 잔여 위험 수락 범위를 나타내는 `RiskAcceptanceCoverage[]`입니다. 형태는 [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)가 담당합니다. |
| `close_blockers` | 현재 보기에 대한 읽기 전용 `CloseReadinessBlocker[]` 관찰입니다. 저장된 `close_task` 결과가 아닙니다. |
| `guarantee_display` | 현재 상태 조회 보기에 대한 `GuaranteeDisplay | null`입니다. |
| `continuity_summary` | `include.continuity=true`일 때의 `ProjectContinuitySummary[]`입니다. 이 상태 보기를 선택하지 않으면 생략합니다. 형태는 [API 상태 스키마](schema-state.md#project-continuity-shapes)가 담당합니다. |

중첩된 `StateSummary`, `StateRecordRef`, `ProjectContinuitySummary`, `CurrentCloseBasis`, `RiskAcceptanceCoverage`, `CloseReadinessBlocker`, `GuaranteeDisplay`, `NextActionSummary` 형태는 [API 상태 스키마](schema-state.md)가 담당합니다.

## 차단 결과

커밋된 차단 분기는 없습니다.

`StatusResult`의 차단 사유와 닫기 차단 사유는 계산된 응답 필드일 뿐입니다.

## 거절 결과

읽기를 안전하게 제공할 수 없으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- Core 사용 불가
- 행위자 출처 또는 작업 범주 불일치
- 요청한 보호 세부정보에 대한 지원되지 않는 호출 맥락
- `Task` 범위 읽기에 필요한 현재 `Task` 없음
- 상태 보기 기반 응답을 요청했지만 상태 보기가 오래되었거나 사용 불가

공개 오류 코드 의미, 우선순위, 거절 응답 처리 경로는 아래 오류 담당 문서가 담당합니다.

## `dry_run` 동작

이 읽기 전용 메서드에서는 `dry_run=true`가 `ToolDryRunResponse` 분기를 만들지 않습니다.

유효한 요청은 같은 `StatusResult` 형태를 반환합니다.

- `base.dry_run=true`
- `base.effect_kind=read_only`

## 저장 효과

이 메서드는 읽기 전용입니다. 정확한 저장 효과 없음 의미는 아래 저장 담당 문서가 담당합니다.

아래 예시는 메서드 안에서만 성립하도록 짧게 구성했습니다. 대표 응답은 상태 조회 결과 분기, 관찰된 참조, 상태 버전, 현재 적용 범위, 현재 적용 Change Unit, 닫기 상태, 다음 행동을 보여 주는 데 필요한 필드로 축약했습니다.

메서드 안의 전제: `task_export_001`, `cu_export_001`, `uj_export_columns_001`은 `proj_export_001`에 이미 있고 아래 상태 버전을 가집니다. 읽기 전용 응답은 이 참조를 관찰할 뿐 새로 만들지 않습니다.

## 최소 유효 요청

```yaml
method: volicord.status
params:
  envelope:
    project_id: proj_export_001
    task_id: task_export_001
    request_id: req_status_export_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: en-US
  include:
    task: true
    pending_user_judgments: true
    write_check: false
    evidence: true
    close: true
    guarantees: true
    continuity: false
```

## 대표 응답

축약한 결과 분기(`StatusResult`, 읽기 전용):

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
  non_goals:
    - "Changing dashboard chart rendering."
  acceptance_criteria:
    - "CSV exports include the selected columns in the approved order."
  autonomy_boundary: "Stay within CSV summary export behavior."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_export_001
    project_id: proj_export_001
    task_id: task_export_001
    state_version: 41
  baseline_ref: baseline_export_001
  shaping_readiness: null
  pending_user_judgment_refs:
    - record_kind: user_judgment
      record_id: uj_export_columns_001
      project_id: proj_export_001
      task_id: task_export_001
      state_version: 42
  blocker_refs: []
  write_check_summary: null
  evidence_summary: null
  close_state: blocked
  close_blockers:
    - category: pending_user_judgment
      code: pending_user_judgment
      message: "User-owned product decision about CSV column order is still pending."
      related_refs:
        - record_kind: user_judgment
          record_id: uj_export_columns_001
          project_id: proj_export_001
          task_id: task_export_001
          state_version: 42
      next_actions:
        - action_kind: record_user_judgment
          owner_method: volicord.record_user_judgment
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
status_summary: "Close readiness is blocked by pending_user_judgment."
next_actions:
  - action_kind: record_user_judgment
    owner_method: volicord.record_user_judgment
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
current_close_basis: null
risk_acceptance_coverage: []
close_blockers:
  - category: pending_user_judgment
    code: pending_user_judgment
    message: "User-owned product decision about CSV column order is still pending."
    related_refs:
      - record_kind: user_judgment
        record_id: uj_export_columns_001
        project_id: proj_export_001
        task_id: task_export_001
        state_version: 42
    next_actions:
      - action_kind: record_user_judgment
        owner_method: volicord.record_user_judgment
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
- 상태, 현재 닫기 근거, 닫기 준비 상태 형태, 증거 요약, 보장 표시: [API 상태 스키마](schema-state.md).
- 지원되는 값과 작업 범주: [API 값 집합](schema-value-sets.md#operation-category-values).
- 공개 오류, 우선순위, 거절 응답 처리 경로: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md).
- 닫기 준비 상태 차단 사유 처리 경로: [API 차단 사유 처리 경로](blocker-routing.md).
- 저장 효과: [저장 효과](../storage-effects.md).
