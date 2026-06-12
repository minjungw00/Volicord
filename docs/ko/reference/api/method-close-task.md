<a id="harnessclose_task"></a>

# `harness.close_task` 참조

## 담당하는 것

이 문서는 현재 MVP의 `harness.close_task` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 공유 계정 데이터 내보내기 확인 시나리오의 최소 요청과 대표 응답
- 저장 담당 문서가 기록 단위 세부사항을 정의하기 전의 메서드 수준 저장 효과 기대치

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`의 공통 스키마 본문
- 상태, 아티팩트, 사용자 판단, 값 집합, 오류의 중첩 스키마 정의
- 저장 DDL, 저장 기록 레이아웃, 아티팩트 생명주기, 보안 보장, Core 제품 의미

## 목적

활성 Task의 닫기 준비 상태를 평가합니다.

조건:

- 선택한 `intent`가 허용됩니다.
- 차단 사유가 없습니다.

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

## 접근 요구사항

| `intent` 종류 | 조건 |
|---|---|
| `intent=check` | 보호된 닫기 준비 상태 세부정보를 위해 `VerifiedSurfaceContext.access_class=read_status`가 필요합니다. |
| 상태 변경 `intent` | `VerifiedSurfaceContext.access_class=core_mutation`, `verified=true`, 호환되는 Task 식별, 유효한 생명주기, 닫기 관련 담당 기록이 필요합니다. |

## 상태 버전 동작

| 경우 | 상태 버전 효과 |
|---|---|
| `intent=check` | `dry_run=true`여도 항상 읽기 전용이며 상태를 올리지 않습니다. |
| 상태 변경 `intent`의 커밋된 종료 닫기 또는 커밋된 차단 닫기 | `project_state.state_version`을 정확히 한 번 올립니다. |
| 닫기 사전 확인 거절, 오래된 `expected_state_version`, 닫기 관련 `WriteAuthorization.basis_state_version` 오래됨, 멱등 요청 해시 충돌, `dry_run` 미리보기 | 아무것도 올리지 않습니다. |

## 성공 결과

`base.response_kind=result`인 `CloseTaskResult`를 반환합니다.

| 경우 | 효과 | `close_state` |
|---|---|---|
| `intent=check` | `base.effect_kind=read_only` | 계산된 현재 닫기 상태. |
| 성공한 종료 상태 변경 | `base.effect_kind=core_committed` | `closed`, `cancelled`, `superseded` 중 하나. |

## 차단 결과

조건:

- 닫기 사전 확인이 성공했습니다.
- `intent=complete`입니다.

결과:

- `blockers: CloseReadinessBlocker[]`를 가진 `CloseTaskResult(close_state=blocked)`를 반환할 수 있습니다.
- 상태 변경 `intent`는 메서드 상태 효과 표가 그 커밋된 차단 결과를 허용할 때만 차단 사유 상태 효과를 저장할 수 있습니다.

비주장:

- `CloseReadinessBlocker`가 있다는 사실만으로 저장을 뜻하지 않습니다.
- `STATE_VERSION_CONFLICT`는 절대 `CloseReadinessBlocker.code`가 아닙니다.

## 거절 결과

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

## `dry_run` 동작

`intent=check`와 `dry_run=true`는 읽기 전용 `CloseTaskResult` 분기에 남습니다. 상태 변경 `intent`의 `dry_run=true`는 유효할 때 공통 미리보기 분기를 사용합니다. 분기 형태와 계획 차단 사유 표현은 [API 코어 스키마](schema-core.md)와 [API 오류](errors.md)가 담당합니다.

## 저장 효과

`intent=check`에는 저장 효과가 없습니다. 상태 변경 닫기 `intent`는 메서드 결과에 따라 닫기 또는 차단 결과를 지속할 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당합니다.

닫기 준비 상태 시나리오 데이터:

리터럴 `intent=complete`는 완료 의도를 고르는 API 값입니다. 전체 닫기 준비 상태 평가 순서를 뜻하는 산문 표현이 아닙니다.

계정 내보내기 확인 시나리오에서 성공한 닫기 준비 상태 관찰 예시는 아래와 같습니다. 증거는 기존 실행 참조 `run_account_export_tests_001`, 승격된 아티팩트 `artifact_account_export_test_log_001`, 해결된 사용자 판단 `uj_001`에 의존합니다.

```yaml
close_readiness:
  ready: true
  evidence:
    - "계정 내보내기 확인 테스트가 통과했습니다."
    - "사용자가 계정 내보내기 확인 문구를 수락했습니다."
```

같은 시나리오에서 차단된 닫기 준비 상태 관찰 예시는 아래와 같습니다. 아래 대표 응답이 사용하는 `state_version: 21` 변형입니다. 테스트 증거는 기존 실행 참조 `run_account_export_tests_001`과 승격된 아티팩트 `artifact_account_export_test_log_001`에 기록되어 있지만 해결된 사용자 판단은 없습니다.

```yaml
close_readiness:
  ready: false
  blockers:
    - code: missing_user_judgment
      message: "사용자가 계정 내보내기 확인 문구를 아직 수락하지 않았습니다."
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
    message: "사용자가 계정 내보내기 확인 문구를 아직 수락하지 않았습니다."
    related_refs: []
evidence_summary:
  status: sufficient
  coverage_items:
    - claim: "계정 내보내기 확인 테스트가 통과했습니다."
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
    reason: "닫기를 시도하기 전에 계정 내보내기 확인 문구를 수락해 달라고 사용자에게 요청한다."
```

## 담당 문서 링크

- 요청 래퍼, 공통 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- 닫기 준비 상태 형태, `CloseReadinessBlocker`, `EvidenceSummary`, `StateSummary`: [API 상태 스키마](schema-state.md).
- 닫기 상태, 생명주기, 닫기 이유, 차단 사유 값: [API 값 집합](schema-value-sets.md).
- 전체 닫기 준비 상태 평가 순서와 정직한 닫기: [Core 모델의 닫기 준비 상태](../core-model.md#close_task).
- 공개 오류와 닫기 차단 사유 경로: [API 오류](errors.md), [`close_task` 차단 사유 매핑](errors.md#harnessclose_task-close-blockers).
- 저장 효과와 상태 버전 동작: [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).
