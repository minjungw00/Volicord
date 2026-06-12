<a id="harnessrecord_run"></a>

# `harness.record_run` 참조

## 담당하는 것

이 문서는 현재 MVP의 `harness.record_run` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 공유 계정 데이터 내보내기 확인 시나리오의 최소 요청과 대표 응답
- 저장 담당 문서가 기록 단위 세부사항을 정의하기 전의 메서드 수준 저장 효과 기대치

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`의 공통 스키마 본문
- 상태, 아티팩트, 사용자 판단, 값 집합, 오류의 중첩 스키마 정의
- 저장 DDL, 저장 기록 레이아웃, 아티팩트 생명주기, 보안 보장, Core 제품 의미

## 목적

`harness.record_run`은 아래 작업을 기록합니다.

- 구체화 작업.
- 직접 응답 또는 결과.
- 구현 작업.

추가 결과:

- 간결한 증거 범위를 갱신합니다.
- 제품 쓰기를 기록할 때 호환되는 쓰기 승인을 소비합니다.
- 기존 아티팩트를 연결합니다.
- 허용되는 경우 적격 스테이징 핸들을 지속 `ArtifactRef`로 승격합니다.

## 필수 입력

- `ToolEnvelope`: `dry_run=false` 커밋에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `task_id`, `change_unit_id`, `kind`, `run_id`, `baseline_ref`, `write_authorization_id`, `summary`, `observed_changes`, `artifact_inputs`, `evidence_updates`.
- 제품 쓰기 실행은 `harness.prepare_write`가 만든 호환되는 활성 쓰기 승인이 필요합니다.
- 새 아티팩트 바이트는 이미 유효한 `StagedArtifactHandle`로 표현되어 있어야 합니다. `record_run`은 새 바이트를 스테이징하지 않습니다.

## 접근 요구사항

조건:

- `VerifiedSurfaceContext.access_class=run_recording`입니다.
- `verified=true`입니다.
- `source_kind=staged_artifact`에서는 현재 확인된 `surface_id`와 `surface_instance_id`가 스테이징 핸들의 기록된 출처와 일치해야 합니다.

비주장:

- `ArtifactInput[]`는 `artifact_registration`을 추가하지 않습니다.
- 현재 MVP에는 접점 간 스테이징 핸들 인계가 없습니다.

## 상태 버전 동작

호환되는 커밋 결과:

- `project_state.state_version`을 정확히 한 번 올립니다.

제품 쓰기 기록이 활성 쓰기 승인을 소비하려면 아래 조건을 모두 만족해야 합니다.

- 현재 상태 버전이 승인 기준 상태와 여전히 맞습니다.
- 관찰된 변경 경로가 승인된 시도와 호환됩니다.

예외:

- 오래된 `expected_state_version`은 소비 전에 거절됩니다.
- 승인 기준 상태가 오래되었으면 소비 전에 거절됩니다.

## 성공 결과

`base.response_kind=result`, `base.effect_kind=core_committed`인 `RecordRunResult`를 반환합니다. 결과에는 `run_summary`, `registered_artifacts`, 갱신된 `evidence_summary`, `blocker_refs`, 현재 `state`가 들어갑니다.

## 차단 결과

실행 자체는 기록 가능하지만 결과가 증거 공백 같은 차단 사유를 만들거나 유지할 때 호환되는 실행 관련 차단 사유 상태를 커밋할 수 있습니다.

비주장: 아래 실패를 숨기기 위해 커밋된 차단 결과를 사용하면 안 됩니다.

- 유효하지 않은 스테이징 핸들.
- 누락된 쓰기 승인.
- 상태가 오래되었습니다.
- 승인 기준 상태가 오래되었습니다.
- 로컬 접근 실패.

위 경우는 커밋 전에 거절됩니다.

## 거절 결과

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

## `dry_run` 동작

`dry_run=true`에서 유효한 미리보기는 `ToolDryRunResponse`를 반환합니다. 분기 형태는 [API 코어 스키마](schema-core.md)가 담당하고, 저장 및 승격 효과 없음 의미는 [저장 효과](../storage-effects.md)와 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다.

## 저장 효과

커밋 시 실행, 증거, 차단 사유, 쓰기 승인 소비, 아티팩트 연결 결과를 지속할 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당하고, 아티팩트 승격 세부사항은 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다.

실행 데이터 예시:

이 실행은 제품 테스트 실행을 기록하며, 공유 `harness.stage_artifact` 예시의 스테이징된 테스트 로그를 증거로 소비할 수 있습니다.

```yaml
command: "npm test -- account-export"
summary: "계정 내보내기 확인 테스트가 통과했습니다."
artifacts:
  - staged_artifact_account_export_test_log_001
run_ref: run_account_export_tests_001
state_version: 21
```

## 최소 유효 요청

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
  summary: "계정 내보내기 확인 테스트가 통과했습니다."
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
        expires_at: "<future-expiration-timestamp>"
        consumed: false
      existing_artifact_ref: null
      relation_hint: "test_log"
      claim: "계정 내보내기 확인 테스트 출력."
      expected_sha256: null
      expected_size_bytes: null
      redaction_state: none
  evidence_updates:
    - claim: "계정 내보내기 확인 테스트가 통과했습니다."
      required_for_close: true
      coverage_state: supported
      supporting_refs: []
      supporting_artifact_refs: []
      gap_refs: []
```

## 대표 응답

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
  summary: "계정 내보내기 확인 테스트가 통과했습니다."
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

## 담당 문서 링크

- 요청 래퍼, 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `RunSummary`, `EvidenceSummary`, `EvidenceCoverageItem`, `StateSummary`, 참조: [API 상태 스키마](schema-state.md).
- `ArtifactInput`, `StagedArtifactHandle`, `ArtifactRef`: [API 아티팩트 스키마](schema-artifacts.md).
- 쓰기 승인과 닫기 관련 증거 경계: [Core 모델](../core-model.md).
- 활성 값과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류: [API 오류](errors.md).
- 저장 효과와 아티팩트 승격: [저장 효과](../storage-effects.md), [아티팩트 저장소](../storage-artifacts.md).
