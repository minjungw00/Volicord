<a id="harnessstatus"></a>

# `harness.status` 참조

## 담당하는 것

이 문서는 현재 MVP의 `harness.status` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 공유 계정 데이터 내보내기 확인 시나리오의 최소 요청과 대표 응답
- 저장 담당 문서가 기록 단위 세부사항을 정의하기 전의 메서드 수준 저장 효과 기대치

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`의 공통 스키마 본문
- 상태, 아티팩트, 사용자 판단, 값 집합, 오류의 중첩 스키마 정의
- 저장 DDL, 저장 기록 레이아웃, 아티팩트 생명주기, 보안 보장, Core 제품 의미

## 목적

Core 상태의 읽기 전용 현재 위치 보기를 반환합니다. 활성 Task 요약, 차단 사유, 대기 중인 사용자 판단, 쓰기 승인 요약, 증거 요약, 닫기 상태, 닫기 준비 상태 발견 사항, 보장 표시, 다음 안전한 행동을 포함할 수 있습니다.

## 필수 입력

- `ToolEnvelope`: `project_id`, `surface_id`, `request_id`, `dry_run`이 필요합니다. `idempotency_key`와 `expected_state_version`은 `null`일 수 있습니다.
- 호출자가 필요한 요약을 고르는 `include` 플래그.

## 접근 요구사항

조건:

- 보호된 Core 세부정보를 반환합니다.
- 같은 프로젝트의 활성 로컬 접점이 있습니다.
- `VerifiedSurfaceContext.access_class=read_status`입니다.

비주장: 오래된 상태 보기, 대화 요약, 생성된 Markdown 파일, 캐시된 텍스트는 상태 권한 근거가 아닙니다.

## 상태 버전 동작

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

## 성공 결과

`base.response_kind=result`, `base.effect_kind=read_only`인 `StatusResult`를 반환합니다. `include.close=true`일 때 `StatusResult.close_blockers`는 읽기 전용 관찰인 `CloseReadinessBlocker[]`입니다. 저장된 `close_task` 결과가 아닙니다.

## 차단 결과

커밋된 차단 분기는 없습니다. `StatusResult`의 차단 사유와 닫기 차단 사유는 계산된 응답 필드일 뿐입니다.

## 거절 결과

읽기를 안전하게 제공할 수 없으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- Core 사용 불가.
- 로컬 접근 불일치.
- 요청한 보호 세부정보에 대한 역량 부족.
- Task 범위 읽기에 필요한 활성 Task 없음.
- 요청한 상태 보기가 오래되었거나 사용 불가.

공개 오류 코드 의미와 우선순위는 [API 오류](errors.md)가 담당합니다.

## `dry_run` 동작

이 읽기 전용 메서드에서는 `dry_run=true`가 `ToolDryRunResponse` 분기를 만들지 않습니다. 유효한 요청은 같은 `StatusResult` 형태를 반환하며 `base.dry_run=true`, `base.effect_kind=read_only`를 사용합니다. 분기 규칙은 [API 코어 스키마](schema-core.md)가 담당합니다.

## 저장 효과

이 메서드는 읽기 전용입니다. 정확한 저장 효과 없음 의미는 [저장 효과](../storage-effects.md)가 담당합니다.

## 최소 유효 요청

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

## 대표 응답

결과 분기(`StatusResult`, 읽기 전용). 이 상태 스냅샷은 `harness.record_run`이 `run_account_export_tests_001`을 만들고 `artifact_account_export_test_log_001`을 증거 아티팩트로 승격한 뒤에 관찰된 응답입니다.

```yaml
base:
  response_kind: result
  effect_kind: read_only
  dry_run: false
  state_version: 21
  events: []
active_task:
  project_id: proj_123
  state_version: 21
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 21
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "계정 데이터 내보내기 전에 명시적 확인 단계를 추가한다."
  scope_summary: "계정 내보내기 흐름과 계정 내보내기 확인 테스트."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_001
    project_id: proj_123
    task_id: task_456
    state_version: 21
status_summary: "계정 내보내기 확인 테스트가 기록되었습니다. 계정 내보내기 확인 문구에 대한 사용자 수락은 아직 대기 중입니다."
next_actions:
  - action: harness.request_user_judgment
    reason: "닫기 전에 계정 내보내기 확인 문구를 수락해 달라고 사용자에게 요청합니다."
pending_user_judgments: []
write_authority_summary:
  status: stale
  write_authorization_ref:
    record_kind: write_authorization
    record_id: wa_001
    project_id: proj_123
    task_id: task_456
    state_version: 20
  basis_state_version: 19
  intended_paths:
    - src/account/export.ts
    - src/account/export-confirmation.ts
    - tests/account-export.test.ts
  guarantee_display:
    level: cooperative
    notes:
      - "쓰기 승인(`Write Authorization`)은 하네스 호환성 기록이며 OS 권한이 아닙니다."
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
blocker_refs: []
close_readiness:
  ready: false
  blockers:
    - code: missing_user_judgment
      message: "사용자가 계정 내보내기 확인 문구를 아직 수락하지 않았습니다."
guarantee_display:
  level: cooperative
  notes:
    - "더 강한 로컬 보장이 활성화되지 않았습니다."
```

## 담당 문서 링크

- 요청 래퍼와 응답 분기: [API 코어 스키마](schema-core.md).
- 상태, 닫기 준비 상태 형태, 증거 요약, 보장 표시: [API 상태 스키마](schema-state.md).
- 활성 값과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류와 닫기 차단 사유 경로: [API 오류](errors.md), [`close_task` 차단 사유 매핑](errors.md#harnessclose_task-close-blockers).
- 저장 효과: [저장 효과](../storage-effects.md).
