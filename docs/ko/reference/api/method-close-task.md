<a id="harnessclose_task"></a>

# `harness.close_task` 참조

## 담당하는 것

이 문서는 기준 범위의 `harness.close_task` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- `CloseTaskResult.blockers`를 만드는 메서드별 차단 사유 분기
- 공유 계정 데이터 내보내기 확인 시나리오의 최소 요청과 대표 응답
- 메서드 수준 저장 효과 요약과 저장 담당 문서 링크

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`의 공통 스키마 본문
- 상태, 아티팩트, 사용자 판단, 값 집합, 오류의 중첩 스키마 정의
- 닫기 차단 사유와 API 응답 사이의 차단 사유 처리 경로 또는 차단 사유 범주 값 정의
- 저장 DDL, 저장 기록 레이아웃, 아티팩트 생명주기, 보안 보장, Core 제품 의미

## 목적

현재 적용 `Task`의 닫기 준비 상태를 평가합니다.

조건:

- 선택한 `intent`가 허용됩니다.
- 닫기 차단 사유가 없습니다.

결과:

- `intent=complete`, `intent=cancel`, `intent=supersede`를 커밋할 수 있습니다.
- `harness.close_task`는 닫기 차단 사유를 반환할 수 있습니다.

비주장:

- 닫기는 Core 상태 전이이며 보고서가 아닙니다.
- 대화, 상태 텍스트, 최종 수락만, 잔여 위험 수락만, 증거만, 렌더링된 보기에서 닫기를 추론하지 않습니다.

## 필수 입력

- `ToolEnvelope`: `project_id`, `surface_id`, `request_id`, `dry_run`이 필요합니다.
- `task_id`, `intent`, `close_reason`, `superseding_task_id`, `user_note`.
- `intent=complete`, `intent=cancel`, `intent=supersede`와 `dry_run=false`에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `intent=check`에서는 `idempotency_key`와 `expected_state_version`이 `null`일 수 있고, `close_reason`은 `null`이어야 합니다.

## `intent` 필드 규칙

`intent`, `close_reason`, `close_state`의 지원 값은 [API 값 집합](schema-value-sets.md#task-lifecycle-values)이 담당합니다.

| `intent` | `close_reason` | `superseding_task_id` | 메서드 규칙 |
|---|---|---|---|
| `check` | `null` | `null` | 읽기 전용 닫기 준비 상태 관찰입니다. |
| `complete` | `completed_self_checked` 또는 `completed_with_risk_accepted` | `null` | 완료 경로이며 닫기 준비 상태 평가가 필요합니다. |
| `cancel` | `cancelled` | `null` | 취소 경로이며 증거 충분성을 대신하지 않습니다. |
| `supersede` | `superseded` | `null`이 아닌 같은 프로젝트의 대체 `Task` 참조 | 대체 경로이며 증거 충분성을 대신하지 않습니다. |

## 접근 요구사항

| `intent` 종류 | 조건 |
|---|---|
| `intent=check` | 보호된 닫기 준비 상태 세부정보를 위해 `VerifiedSurfaceContext.access_class=read_status`가 필요합니다. |
| 상태 변경 `intent` | `core_mutation`, 확인된 접점 맥락, 호환되는 `Task` 상태, 닫기 관련 담당 기록이 필요합니다. |

## 메서드 흐름

구현은 `harness.close_task`를 아래 순서로 평가합니다.

1. 요청 래퍼, 메서드 필드, `intent` 필드 조합, 같은 프로젝트의 `Task` 식별자를 검증합니다. 형태 오류, 잘못된 프로젝트 식별자, 읽을 수 없는 `Task` 식별자는 `ToolRejectedResponse`를 반환합니다.
2. 접점 맥락, 접근 등급, 로컬 역량, 요청한 종료 경로의 선행조건을 확인합니다.
3. `dry_run=false`인 상태 변경 `intent`에서는 닫기 준비 상태 평가 전에 `idempotency_key`, 현재 `expected_state_version`, 멱등 요청 해시, 닫기 관련 `WriteAuthorization.basis_state_version`을 확인합니다. 오래되었거나 충돌하는 값은 `ToolRejectedResponse`를 반환합니다.
4. `intent=check`는 현재 닫기 준비 상태를 계산해 읽기 전용 `CloseTaskResult`를 반환합니다. 이 분기는 `close_state=ready` 또는 `close_state=blocked`를 보고할 수 있으며 커밋하지 않습니다.
5. 상태 변경 `intent`와 `dry_run=true` 조합은 유효한 사전 확인 뒤 공통 미리보기 분기를 반환합니다. 미리보기 차단 사유는 `PlannedBlocker` 데이터이며 저장된 `CloseReadinessBlocker` 객체가 아닙니다.
6. `intent=complete`는 전체 닫기 준비 상태 평가 순서를 실행합니다. 차단 사유가 남아 있으면 차단 분기를 반환하고, 없으면 `close_state=closed`를 커밋합니다.
7. `intent=cancel` 또는 `intent=supersede`는 요청한 종료 경로의 제약만 평가합니다. 두 종료 경로는 완료 증거나 최종 수락을 요구하지 않지만, 취소나 대체 자체가 정직하지 않은 경우 차단될 수 있습니다.

## 상태 버전 동작

| 경우 | 상태 버전 효과 |
|---|---|
| `intent=check` | `dry_run=true`여도 항상 읽기 전용이며 상태를 올리지 않습니다. |
| 상태 변경 `intent`의 커밋된 종료 닫기 또는 커밋된 차단 닫기 | `project_state.state_version`을 정확히 한 번 올립니다. |
| 커밋 전 실패 또는 `dry_run` 미리보기 | 아무것도 올리지 않습니다. |

커밋 전 실패에는 닫기 사전 확인 거절, 오래된 `expected_state_version`, 닫기 관련 `WriteAuthorization.basis_state_version` 오래됨, 멱등 요청 해시 충돌이 포함됩니다.

## 성공 결과

`base.response_kind=result`인 `CloseTaskResult`를 반환합니다.

| 경우 | 효과 | `close_state` |
|---|---|---|
| `intent=check` | `base.effect_kind=read_only` | 계산된 현재 닫기 상태. |
| 성공한 종료 상태 변경 | `base.effect_kind=core_committed` | `closed`, `cancelled`, `superseded` 중 하나. |

## 차단 결과

조건:

- 닫기 사전 확인이 성공했습니다.
- 메서드가 읽기 전용 닫기 준비 상태 관찰 또는 종료 경로 평가에 도달했습니다.
- 요청한 경로에 하나 이상의 닫기 차단 사유 또는 종료 차단 사유가 있습니다.

결과:

- `blockers: CloseReadinessBlocker[]`를 가진 `CloseTaskResult(close_state=blocked)`를 반환할 수 있습니다.
- 상태 변경 `intent`는 이 메서드의 상태 버전 규칙과 저장 효과 담당 문서가 그 커밋된 차단 결과를 허용할 때만 차단 사유 상태 효과를 저장할 수 있습니다.

메서드별 차단 사유 생성 분기:

| 분기 | 차단 사유 생성 |
|---|---|
| `intent=check` | 현재 닫기 차단 사유를 읽기 전용 관찰 데이터로 반환합니다. 차단 사유 행을 만들거나 상태를 증가시키지 않습니다. |
| `intent=complete` | 적용되는 담당 조건이 충족되지 않았을 때 `Task` 상태, 열린 실행 기록 호환성, 범위, 사용자 소유 판단, 민감 동작 승인, 쓰기 호환성, 기준 상태, 접점 역량, 증거, 아티팩트 가용성, 최종 수락, 잔여 위험 표시, 잔여 위험 수락, 복구 제약에 대한 차단 사유를 만들 수 있습니다. |
| `intent=cancel` | 호환되지 않는 `Task` 상태, 필요한 복구나 수리 제약, 담당 문서가 정의한 취소 제약처럼 취소 전용 종료 제약에 대해서만 차단 사유를 만듭니다. 완료 전용 증거 공백과 최종 수락 공백은 그 자체로 취소를 막지 않습니다. |
| `intent=supersede` | 호환되지 않는 `Task` 상태, 호환되지 않는 같은 프로젝트의 대체 `Task` 관계, 복구나 수리 제약처럼 대체 전용 종료 제약에 대해서만 차단 사유를 만듭니다. 완료 전용 증거 공백과 최종 수락 공백은 그 자체로 대체를 막지 않습니다. |

비주장:

- `CloseReadinessBlocker`가 있다는 사실만으로 저장을 뜻하지 않습니다.
- `STATE_VERSION_CONFLICT`는 절대 `CloseReadinessBlocker.code`가 아닙니다.
- 차단 사유 범주는 사용자 판단, 승인, 증거, 아티팩트 가용성, 수락, 위험 수락, 복구 상태 자체를 만들지 않습니다.

## 거절 결과

닫기 준비 상태 평가 전 사전 확인 실패가 있으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- 검증 실패.
- 로컬 접근 실패.
- 오래된 `expected_state_version`.
- 닫기 관련 `WriteAuthorization.basis_state_version` 오래됨.
- 멱등 요청 해시 충돌.
- 잘못된 프로젝트 또는 읽을 수 없는 `Task` 식별.
- Core 사용 불가.
- 역량 부족.

비주장:

- 거부 응답은 `CloseTaskResult.blockers`를 반환하지 않습니다.
- 거부 응답은 닫기 효과를 만들지 않습니다.

## `dry_run` 동작

`intent=check`와 `dry_run=true`는 읽기 전용 `CloseTaskResult` 분기에 남습니다. 상태 변경 `intent`의 `dry_run=true`는 유효할 때 공통 미리보기 분기를 사용합니다. 분기 형태는 [API 코어 스키마](schema-core.md)가 담당하고, 예상 차단 사유 응답 분기 경로는 [API 오류 처리 경로](error-routing.md)가 담당합니다. 닫기 차단 사유와 API 응답 사이의 차단 사유 처리 경로의 의미는 [API 차단 사유 처리 경로](blocker-routing.md)가 담당합니다.

## 저장 효과

`intent=check`에는 저장 효과가 없습니다. 상태 변경 닫기 `intent`는 메서드 결과에 따라 닫기 또는 차단 결과를 지속할 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당합니다.

닫기 준비 상태 시나리오 데이터:

리터럴 `intent=complete`는 완료 의도를 고르는 API 값입니다. 전체 닫기 준비 상태 평가 순서를 뜻하는 산문 표현이 아닙니다.

계정 데이터 내보내기 확인 시나리오에서 성공한 닫기 준비 상태 관찰 예시는 아래와 같습니다. 증거는 기존 실행 참조 `run_account_export_tests_001`, 승격된 아티팩트 `artifact_account_export_test_log_001`, 해결된 사용자 판단 `uj_001`에 의존합니다.

```yaml
close_readiness:
  ready: true
  evidence:
    - "계정 데이터 내보내기 확인 테스트가 통과했습니다."
    - "사용자가 계정 데이터 내보내기 확인 문구를 수락했습니다."
```

같은 시나리오에서 차단된 닫기 준비 상태 관찰 예시는 아래와 같습니다. 아래 대표 응답이 사용하는 `state_version: 21` 변형입니다. 테스트 증거는 기존 실행 참조 `run_account_export_tests_001`과 승격된 아티팩트 `artifact_account_export_test_log_001`에 기록되어 있지만 해결된 사용자 판단은 없습니다.

```yaml
close_readiness:
  ready: false
  blockers:
    - code: missing_user_judgment
      message: "계정 데이터 내보내기 확인 문구에 대한 사용자 수락이 없습니다."
```

## 최소 유효 요청

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

## 대표 응답

차단된 읽기 전용 결과 분기(`CloseTaskResult`, `intent=check`):

```yaml
base:
  response_kind: result
  effect_kind: read_only
  dry_run: false
  state_version: 21
  events: []
close_state: blocked
state:
  project_id: proj_123
  state_version: 21
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 21
blockers:
  - category: user_judgment
    code: missing_user_judgment
    message: "계정 데이터 내보내기 확인 문구에 대한 사용자 수락이 없습니다."
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
next_actions:
  - action: harness.request_user_judgment
    reason: "닫기를 시도하기 전에 계정 데이터 내보내기 확인 문구를 수락해 달라고 사용자에게 요청한다."
```

## 담당 문서 링크

- 요청 래퍼, 공통 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `CloseTaskResult.blockers`, `CloseReadinessBlocker`, `EvidenceSummary`, `StateSummary` 형태: [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes).
- 닫기 상태, 생명주기, 닫기 이유, `CloseReadinessBlocker.category` 값: [API 값 집합](schema-value-sets.md#state-and-blocker-values).
- 닫기 준비 상태 의미와 정직한 닫기: [Core 모델의 닫기 준비 상태](../core-model.md#close_task).
- 공개 `ErrorCode` 의미: [API 오류 코드](error-codes.md).
- 거부 응답 분기 경로: [API 오류 처리 경로](error-routing.md).
- 닫기 차단 사유와 API 응답 사이의 차단 사유 처리 경로의 의미: [API 차단 사유 처리 경로](blocker-routing.md).
- 저장 효과와 상태 버전 동작: [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).
