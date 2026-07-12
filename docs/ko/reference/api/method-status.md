<a id="volicordstatus"></a>

# `volicord.status` 참조

## 담당하는 것

이 문서는 기준 범위의 `volicord.status` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 현재 Core 상태 조회 동작과 상태 버전 효과 없음 경계
- 상태 조회 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, `dry_run`, 거절 응답 스키마 본문
- 상태, 아티팩트, 판단, 값 집합, 오류의 중첩 스키마 정의
- 저장 DDL, 저장 기록 레이아웃, 정확한 저장 효과, 아티팩트 생명주기, 보안 보장, Core 권한 의미
- 공개 오류 코드 의미, 공개 오류 우선순위, 공통 응답 분기 처리 경로

## 목적

`volicord.status`는 Core 상태의 현재 위치를 보여 줍니다. 호출자는 다음 항목을 선택할 수 있습니다.

- 현재 `Task`와 Change Unit
- 차단 사유, 대기 중인 사용자 판단, 사용할 수 있는 User Channel 답변 경로, 쓰기 티켓 상태
- 증거와 닫기 준비 상태 관찰 정보(`GuardHealthSummary`, `CoverageSummary` 포함)
- 프로젝트 연속성, 보장 표시, 다음 안전한 행동

성공한 결과에는 항상 간결한 `summary_card`도 포함됩니다.

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
- 쓰기 티켓 변경

## 성공 결과

아래 값을 담은 `StatusResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=read_only`
- `base.disclosure.guarantee_class=authority_record`

`include.close=true`일 때 `StatusResult.close_blockers`는 읽기 전용 관찰인 `CloseReadinessBlocker[]`입니다.

비주장: `StatusResult.close_blockers`는 저장된 `close_task` 결과, 정확성 증명, 테스트 충분성 증명, 인간 검토 대체가 아닙니다. `base.disclosure.non_guarantees`가 안정적인 기계 판독 가능 값을 담습니다.

`include` 상태 보기 계약:

- `include.task`는 선택된 `Task` 요약과 현재 Change Unit을 `active_task`로 반환합니다.
- `include.pending_user_judgments`는 현재 대기 판단 참조, 사용자 행동을 위한 `pending_judgment_inbox_items`, 현재 호출 맥락에서 지원되는 답변 경로를 나타내는 `user_channel_availability`를 반환합니다. 관련 있는 오래됨 또는 대체됨 판단 상태는 `blocker_refs`, `next_actions.required_refs` 같은 기존 결과 필드로 나타납니다.
- `include.write_ticket`는 활성, 만료, 오래됨, 소비됨 또는 그 밖의 관련 쓰기 티켓 상태를 `write_ticket_summary`로 반환합니다.
- `write_ticket_summary`는 호환성 요약일 뿐이며 파일시스템 접근, 셸 승인, 최종 수락, 일반 쓰기 승인, 쓰기가 실제로 일어났다는 증명이 아닙니다.
- `include.evidence`는 사용할 수 있을 때 현재 `EvidenceSummary`와 범위, 기준 `evidence_gate` 상태 보기를 반환합니다.
- `include.close`는 `CurrentCloseBasis | null`, 닫기 상태, 계산된 차단 사유, 위험 수락 범위, 호스트 훅 경로 안전성을 포함한 `GuardHealthSummary` 상태 정보, 도출된 `CoverageSummary`, 관련 다음 행동, 동일한 기준 `evidence_gate`를 반환합니다. 차단 사유는 `volicord.check_close`와 같은 닫기 준비 상태 계산을 사용합니다.
- 증거나 닫기 세부사항이 선택되면 `summary_card.evidence`는 정확히 `evidence_gate.state`입니다. `not_required`, `optional_none`, `required_missing`, `partial`, `sufficient`, `stale`, `blocked` 중 하나를 사용하며 증거 첨부 표시 상태에서 두 번째 gate를 파생하지 않습니다.
- `include.guarantees`는 프로젝트 강제 프로필, 확인된 호출 맥락, 활성화된 강제 메커니즘, 지원되는 기준 범위에서 파생된 보장만 반환합니다.
- `include.continuity`는 오래 유지하는 프로젝트 수준 맥락의 활성 `ProjectContinuitySummary[]` 항목을 반환합니다.
- `summary_card`는 성공한 `StatusResult` 응답에서 항상 반환됩니다. 담당 문서가 선택한 보기를 공개 표시 용어와, 알 수 있을 때 선택된 다음 행동 하나인 `next`로 요약합니다. 요약하는 구조화 필드 너머의 권한을 추가하지 않습니다.
- `include.evidence=false`이면 `evidence_summary`를 생략합니다. `include.close=true`이면 `evidence_gate`는 계속 반환합니다.
- `include.close=false`이면 `CurrentCloseBasis`, 닫기 상태, 닫기 차단 사유, `GuardHealthSummary` 상태 정보, `CoverageSummary`, 잔여 위험 범위, 닫기 전용 다음 행동을 생략합니다. `include.evidence=true`이면 Core는 증거 출처, 최신성, 아티팩트 차단 사유가 기준 gate에 반영되도록 내부에서 동일한 읽기 전용 닫기 근거를 평가하지만 닫기 전용 필드는 노출하지 않습니다.
- `include.guarantees=false`는 보장 표시를 파생하지도 반환하지도 않는다는 뜻입니다.
- `include.continuity=false`는 프로젝트 연속성 요약을 읽거나 반환하지 않는다는 뜻입니다.

정직한 상태 보기 규칙:
- 계산하지 않았거나 선택하지 않은 선택 상태 보기는 스키마가 허용하는 곳에서 생략합니다. 고정 형태의 최상위 필드는 해당 `include` 플래그가 `false`이면 `null` 또는 빈 배열로 남으므로 요청의 `include` 객체와 함께 해석해야 합니다.
- 상태 보기가 선택된 경우 `null`은 계산했지만 사용할 값이 없음을 뜻하고, 닫기 차단 사유를 포함한 빈 배열은 계산했지만 항목이 없음을 뜻합니다.
- 호스트 지침, 연결 모드, 생성된 텍스트만으로는 보장이 생기지 않습니다. 협력형 전용 배포는 `detective`를 주장하면 안 됩니다.
- `GuaranteeDisplay.capability_refs`는 해당 참조를 사용할 수 있을 때 호출 바인딩, Agent Connection, 관찰 사실을 식별해야 합니다.

`include.evidence=true` 또는 `include.close=true`와 [`volicord.check_close`](method-close-task.md#volicordcheck_close)는 같은 닫기 준비 상태 증거 gate 계산을 사용합니다. 따라서 같은 상태 버전의 증거 전용 status 결과와 check-close 결과는 같은 `evidence_gate`를 반환합니다. 닫기 선택 여부는 닫기 필드 노출만 제어하며 별도 gate 계산을 만들지 않습니다. `volicord.status`는 재실행 행, 이벤트, Core 상태 변경, 닫기 변경, 상태 버전 증가를 만들지 않습니다. 세션에 묶인 Agent Connection으로 호출되면 런타임은 이후 메서드 경계 확인에서 Product Repository 스냅샷을 비교할 수 있도록 `session-watch` 진단 기록을 초기화할 수 있습니다.

## 메서드 결과 필드

`StatusResult`는 성공적인 상태 조회에 대한 메서드별 결과 분기입니다. 이 결과는 `base: ToolResultBase`와 아래 메서드 소유 최상위 필드를 담습니다.

| 필드 | 결과 필드 의미 |
|---|---|
| `base` | 공통 결과 메타데이터입니다. `ToolResultBase` 형태는 [API 코어 스키마](schema-core.md#common-response)가 담당합니다. 읽기 전용 상태 조회 결과는 `events: []`와 권한 기록 공개를 사용합니다. 공통 응답 분기에 `EventRef.event_kind`가 있을 때 그 값은 불투명한 예시용 분류 문자열로 남습니다. |
| `summary_card` | 선택된 상태 조회 보기에 대한 `SummaryCard`입니다. 증거나 닫기 세부사항이 선택되면 증거 표시는 `evidence_gate.state`를 복사합니다. 형태는 [API 상태 스키마](schema-state.md#current-position-display-shapes)가 담당합니다. |
| `active_task` | 현재 선택된 `Task` 요약의 `StateSummary | null`입니다. |
| `status_summary` | 현재 상태 조회 보기를 요약하는 자유 형식 표시 문자열입니다. 닫기 준비 상태 보기가 선택되면 현재 닫기 준비 상태나 첫 번째 닫기 차단 사유 코드를 요약할 수 있습니다. 구조화된 권한 사실은 다른 결과 필드에 남습니다. |
| `next_actions` | 다음 안전한 API 단계를 설명하는 `NextActionSummary[]`입니다. 비어 있지 않은 목록에는 `presentation_role=primary`인 항목이 정확히 하나 있으며 `summary_card.next_action`은 배열 위치가 아니라 그 행동을 선택합니다. |
| `pending_user_judgments` | 상태 조회 보기에 선택된 대기 중 사용자 판단 기록의 `StateRecordRef[]`입니다. |
| `pending_judgment_inbox_items` | `include.pending_user_judgments=true`일 때 사용자 행동이 필요한 대기 판단의 `JudgmentInboxItem[]`입니다. 형태는 [API 판단 스키마](schema-judgment.md#judgmentinboxitem)가 담당합니다. |
| `user_channel_availability` | `include.pending_user_judgments=true`이고 작업 범위 판단 보기를 사용할 수 있을 때 지원되는 답변 경로를 나타내는 `UserChannelAvailability`입니다. 호스트 프롬프트 입력을 사용할 수 없다고 보고하면서도 사용할 수 있는 채팅 캡처, 로컬 consent, CLI inbox 경로를 함께 보고할 수 있습니다. 형태는 [API 판단 스키마](schema-judgment.md#judgmentinboxitem)가 담당합니다. |
| `blocker_refs` | 현재 상태 조회 보기에 보이는 차단 사유 기록의 `StateRecordRef[]`입니다. |
| `write_ticket_summary` | 쓰기 티켓 상태 보기의 `WriteTicketStateSummary | null`입니다. `include.write_ticket=true`인데 `null`이면 관련 쓰기 티켓을 사용할 수 없음을 뜻하고, 해당 상태 보기를 선택하지 않으면 이 고정 형태 필드는 `null`로 남습니다. 형태는 [API 상태 스키마](schema-state.md#current-position-display-shapes)가 담당합니다. |
| `evidence_summary` | `include.evidence=true`일 때의 `EvidenceSummary | null`입니다. 명시적 `null`은 선택한 상태 보기에서 현재 증거 요약을 찾지 못했음을 뜻하고, `include.evidence=false`이면 이 필드를 생략합니다. 형태는 [API 상태 스키마](schema-state.md#evidence-and-run-snapshot-shapes)가 담당합니다. |
| `evidence_gate` | `include.evidence=true` 또는 `include.close=true`일 때의 `EvidenceGateSummary | null`입니다. 명시적 `null`은 Task 범위 gate를 사용할 수 없다는 뜻이고, 두 상태 보기를 모두 선택하지 않으면 필드를 생략합니다. `active_task.evidence_gate`와 `summary_card.evidence`는 이 상태 보기를 복사합니다. |
| `close_state` | 현재 보기의 닫기 상태 값입니다. 현재 닫기 상태가 없을 때의 `none`을 포함한 지원 값은 [API 값 집합](schema-value-sets.md#task-lifecycle-values)이 담당합니다. |
| `current_close_basis` | 닫기 상태 조회 보기에 선택된 `CurrentCloseBasis | null`입니다. 형태는 [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)가 담당합니다. |
| `risk_acceptance_coverage` | 닫기 상태 조회 보기에서 현재 잔여 위험 수락 범위를 나타내는 `RiskAcceptanceCoverage[]`입니다. 형태는 [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)가 담당합니다. |
| `close_blockers` | 현재 보기에 대한 읽기 전용 `CloseReadinessBlocker[]` 관찰입니다. 저장된 `close_task` 결과가 아닙니다. |
| `guard_health` | 닫기 상태 조회 보기에 선택된 `GuardHealthSummary | null`입니다. 형태는 [API 상태 스키마](schema-state.md#guard-health-summary)가 담당합니다. |
| `coverage_summary` | 닫기 상태 조회 보기에 선택된 `CoverageSummary | null`입니다. 형태와 값 의미는 [API 상태 스키마](schema-state.md#guard-health-summary)가 담당합니다. `record` 권한 기록과 `detective` 관찰을 구분하고 관찰 범위의 비보장을 보고합니다. |
| `guarantee_display` | 현재 상태 조회 보기에 대한 `GuaranteeDisplay | null`입니다. |
| `continuity_summary` | `include.continuity=true`일 때의 `ProjectContinuitySummary[]`입니다. 이 상태 보기를 선택하지 않으면 생략합니다. 형태는 [API 상태 스키마](schema-state.md#project-continuity-shapes)가 담당합니다. |

중첩된 `UserChannelAvailability`와 `JudgmentInboxItem` 형태는 [API 판단 스키마](schema-judgment.md#judgmentinboxitem)가 담당합니다. 중첩된 `SummaryCard`, `StateSummary`, `StateRecordRef`, `WriteTicketStateSummary`, `EvidenceSummary`, `EvidenceGateSummary`, `ProjectContinuitySummary`, `CurrentCloseBasis`, `RiskAcceptanceCoverage`, `CloseReadinessBlocker`, `GuardHealthSummary`, `CoverageSummary`, `GuaranteeDisplay`, `NextActionSummary` 형태는 [API 상태 스키마](schema-state.md)가 담당합니다.

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

이 읽기형 메서드에서는 `dry_run=true`가 `ToolDryRunResponse` 분기를 만들지 않습니다.

유효한 요청은 같은 `StatusResult` 형태를 반환합니다.

- `base.dry_run=true`
- `base.effect_kind=read_only`

## 저장 효과

이 메서드는 Core 상태 변경, 이벤트, 재실행 행, 닫기 변경, 상태 버전 증가를 저장하지 않습니다. 세션에 묶인 Agent Connection으로 호출되면 위에서 설명한 것처럼 런타임이 `session-watch` 진단 기록을 초기화할 수 있습니다. 정확한 저장 의미는 아래 저장 담당 문서가 담당합니다.

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
    locale: ko-KR
  include:
    task: true
    pending_user_judgments: true
    write_ticket: false
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
    produced_at_state_version: 42
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "대시보드 합계의 CSV 요약 내보내기를 추가합니다."
  scope_summary: "CSV 내보내기 열 순서와 요약 합계."
  non_goals:
    - "대시보드 차트 렌더링 변경."
  acceptance_criteria:
    - acceptance_criterion_id: criterion_csv_columns_001
      statement: "CSV 내보내기에 선택한 열이 지정된 순서로 포함됩니다."
      evidence_requirement: not_required
  autonomy_boundary: "CSV 요약 내보내기 동작만 다룹니다."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_export_001
    project_id: proj_export_001
    task_id: task_export_001
    produced_at_state_version: 42
  baseline_ref: baseline_export_001
  shaping_readiness: null
  pending_user_judgment_refs:
    - record_kind: user_judgment
      record_id: uj_export_columns_001
      project_id: proj_export_001
      task_id: task_export_001
      produced_at_state_version: 42
  blocker_refs: []
  write_ticket_summary: null
  evidence_summary: null
  evidence_gate:
    state: not_required
  close_state: blocked
  close_blockers:
    - category: pending_user_judgment
      code: pending_user_judgment
      message: "CSV 열 순서에 대한 사용자 소유 제품 결정이 아직 대기 중입니다."
      can_resolve_in_chat: false
      outside_chat_action_required: false
      related_refs:
        - record_kind: user_judgment
          record_id: uj_export_columns_001
          project_id: proj_export_001
          task_id: task_export_001
          produced_at_state_version: 42
      next_actions:
        - presentation_role: primary
          action_kind: record_user_judgment
          owner_method: volicord.record_user_judgment
          allowed_operation_categories: [user_only]
          label: "사용자가 User Channel을 통해 대기 중인 CSV 열 순서 결정에 답해야 합니다."
          blocking_question: "대기 중인 CSV 열 순서 결정에 사용자가 어떻게 답했습니까?"
          expected_state_version: null
          required_refs:
            - record_kind: user_judgment
              record_id: uj_export_columns_001
              project_id: proj_export_001
              task_id: task_export_001
              produced_at_state_version: 42
  guarantee_display:
    level: cooperative
    basis: "현재 적용된 더 강한 로컬 보장은 없습니다."
    capability_refs: []
status_summary: "닫기 준비 상태가 pending_user_judgment 때문에 차단되었습니다."
next_actions:
  - presentation_role: primary
    action_kind: record_user_judgment
    owner_method: volicord.record_user_judgment
    allowed_operation_categories: [user_only]
    label: "사용자가 User Channel을 통해 대기 중인 CSV 열 순서 결정에 답해야 합니다."
    blocking_question: "대기 중인 CSV 열 순서 결정에 사용자가 어떻게 답했습니까?"
    expected_state_version: null
    required_refs:
      - record_kind: user_judgment
        record_id: uj_export_columns_001
        project_id: proj_export_001
        task_id: task_export_001
        produced_at_state_version: 42
pending_user_judgments:
  - record_kind: user_judgment
    record_id: uj_export_columns_001
    project_id: proj_export_001
    task_id: task_export_001
    produced_at_state_version: 42
user_channel_availability: &user_channel_availability_example
  paths:
    - kind: mcp_elicitation
      label: "호스트 프롬프트 입력"
      available: false
      status: unavailable
      capture_basis: mcp_elicitation_user_channel
      detail: "이 호출에서는 호스트 프롬프트 입력을 사용할 수 없습니다."
    - kind: prompt_capture
      label: "채팅 명령 캡처"
      available: false
      status: unavailable
      capture_basis: user_prompt_submit_hook
      detail: "현재 이 연결에서는 채팅 명령 캡처를 사용할 수 없습니다."
    - kind: local_web_consent
      label: "로컬 consent URL"
      available: false
      status: unavailable
      capture_basis: local_user_local_web
      detail: "이 호출에서 사용할 수 있는 로컬 consent URL이 없습니다."
    - kind: cli
      label: "CLI inbox"
      available: true
      status: available
      capture_basis: cli_direct_user_channel
      detail: "로컬 터미널에서 사용자로 답변합니다."
  recommended_path_kind: cli
  recommended_path_label: "CLI inbox"
  recommendation: "대기 중인 판단에는 CLI inbox로 답하세요."
pending_judgment_inbox_items:
  - judgment_id: uj_export_columns_001
    question: "어떤 CSV 열 순서를 사용할까요?"
    requirement_status: required
    choices:
      - choice_id: accept
        label: "제안된 CSV 열 순서 사용"
    answer_path_availability: *user_channel_availability_example
    preferred_capture_path:
      kind: cli
      label: "CLI inbox"
      available: true
      command: "volicord inbox answer uj_export_columns_001 --choice <choice>"
blocker_refs: []
evidence_gate:
  state: not_required
close_state: blocked
current_close_basis: null
risk_acceptance_coverage: []
close_blockers:
  - category: pending_user_judgment
    code: pending_user_judgment
    message: "CSV 열 순서에 대한 사용자 소유 제품 결정이 아직 대기 중입니다."
    can_resolve_in_chat: false
    outside_chat_action_required: false
    related_refs:
      - record_kind: user_judgment
        record_id: uj_export_columns_001
        project_id: proj_export_001
        task_id: task_export_001
        produced_at_state_version: 42
    next_actions:
      - presentation_role: primary
        action_kind: record_user_judgment
        owner_method: volicord.record_user_judgment
        allowed_operation_categories: [user_only]
        label: "사용자가 User Channel을 통해 대기 중인 CSV 열 순서 결정에 답해야 합니다."
        blocking_question: "대기 중인 CSV 열 순서 결정에 사용자가 어떻게 답했습니까?"
        expected_state_version: null
        required_refs:
          - record_kind: user_judgment
            record_id: uj_export_columns_001
            project_id: proj_export_001
            task_id: task_export_001
            produced_at_state_version: 42
guarantee_display:
  level: cooperative
  basis: "현재 적용된 더 강한 로컬 보장은 없습니다."
  capability_refs: []
```

## 담당 문서 링크

- 요청 래퍼와 응답 분기: [API 코어 스키마](schema-core.md).
- 상태, 현재 닫기 근거, 닫기 준비 상태 형태, 증거 요약, 보장 표시: [API 상태 스키마](schema-state.md).
- 지원되는 값과 작업 범주: [API 값 집합](schema-value-sets.md#operation-category-values).
- 공개 오류, 우선순위, 거절 응답 처리 경로: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md).
- 닫기 준비 상태 차단 사유 처리 경로: [API 차단 사유 처리 경로](blocker-routing.md).
- 저장 효과: [저장 효과](../storage-effects.md).
