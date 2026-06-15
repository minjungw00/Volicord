<a id="harnessrecord_run"></a>

# `harness.record_run` 참조

## 담당하는 것

이 문서는 기준 범위의 `harness.record_run` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 실행 기록, 증거 갱신, 차단 사유 갱신, 아티팩트 승격 메서드 동작
- record-run 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, `dry_run`, 거절 응답 스키마 본문
- 상태, 아티팩트, 값 집합, 오류의 중첩 스키마 정의
- Core의 증거 의미, Core 권한 의미, 저장 DDL, 저장 기록 레이아웃, 정확한 저장 효과, 아티팩트 생명주기, 보안 보장
- 공개 오류 코드 의미, 공개 오류 우선순위, 기계 판독용 오류 세부사항, 공통 응답 분기 처리 경로

## 목적

`harness.record_run`은 아래 작업을 기록합니다.

- 구체화 작업
- 직접 응답 또는 결과
- 구현 작업

이 메서드는 간결한 증거 범위를 갱신하고, 제품 쓰기를 기록할 때 호환되는 `Write Authorization`을 소비하며, 기존 아티팩트를 연결하고, 허용되는 경우 적격 스테이징 핸들을 지속 `ArtifactRef`로 승격할 수도 있습니다.

## 필수 입력

- 유효한 `ToolEnvelope`. 커밋되는 `dry_run`이 아닌 요청에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `task_id`, `change_unit_id`, `kind`, `run_id`, `baseline_ref`, `write_authorization_id`, `summary`, `observed_changes`, `artifact_inputs`, `evidence_updates`.
- 제품 쓰기 실행은 `harness.prepare_write`가 만든 호환되는 `status=active` `Write Authorization`이 필요합니다.
- 새 아티팩트 바이트는 이미 유효한 `StagedArtifactHandle`로 표현되어 있어야 합니다. `harness.record_run`은 새 바이트를 스테이징하지 않습니다.

## 접근 요구사항

요구사항:

- `VerifiedSurfaceContext.access_class=run_recording`
- `verified=true`

`source_kind=staged_artifact`인 경우:

- 현재 확인된 `surface_id`가 스테이징 핸들의 기록된 출처와 일치해야 합니다.
- 현재 확인된 `surface_instance_id`가 스테이징 핸들의 기록된 출처와 일치해야 합니다.

비주장:

- `ArtifactInput[]`는 `artifact_registration`을 추가하지 않습니다.
- 접점 간 스테이징 핸들 전달은 기준 범위 밖입니다.

## 상태 버전 동작

호환되는 커밋 결과는 `project_state.state_version`을 정확히 한 번 올립니다.

제품 쓰기 기록이 `Write Authorization`을 소비하려면 아래 조건을 모두 만족해야 합니다.

- 현재 상태 버전이 `Write Authorization`의 근거 상태와 여전히 맞습니다.
- 관찰된 변경 경로가 권한 부여된 시도와 호환됩니다.

오래된 `expected_state_version`과 오래된 `Write Authorization` 근거는 `Write Authorization`을 소비하기 전에 거절됩니다.

## 성공 결과

아래 값을 담은 `RecordRunResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `run_summary`
- 모든 `registered_artifacts`
- 갱신된 `evidence_summary`
- `blocker_refs`
- 현재 `state`

## 차단 결과

실행 자체는 기록 가능하지만 결과가 증거 공백 같은 차단 사유를 만들거나 유지할 때 호환되는 실행 관련 차단 사유 상태를 커밋할 수 있습니다.

허용되지 않는 것:

- 커밋된 차단 결과는 유효하지 않은 스테이징 핸들, 누락된 `Write Authorization`, 오래된 상태, 오래된 `Write Authorization` 근거, 로컬 접근 실패를 숨기면 안 됩니다.

위 경우는 커밋 전에 거절됩니다.

## 거절 결과

아래 경우는 `ToolRejectedResponse`를 반환합니다.

- 오래된 `expected_state_version`
- 오래된 `Write Authorization` 기준
- 제품 쓰기에 필요한 `Write Authorization` 누락 또는 무효
- 유효하지 않은 스테이징 핸들
- 스테이징 핸들 출처 불일치
- 누락된 아티팩트
- 범위 위반
- 오래된 기준선
- 로컬 접근 실패
- 역량 부족
- 검증기 실패

비주장: 유효하지 않은 스테이징 핸들은 [API 오류 세부사항](error-details.md#artifact-input-error-reason)이 담당하는 아티팩트 입력 세부정보가 있는 검증 실패입니다. 요청 수준 로컬 접근 자체가 실패한 경우가 아니라면 로컬 접근 불일치가 아닙니다.

공개 오류 코드 의미, 우선순위, 세부사항, 거절 응답 처리 경로는 아래 오류 담당 문서가 담당합니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 미리보기:

- `ToolDryRunResponse`를 반환합니다.
- Run, 증거 갱신, 차단 사유 갱신, 아티팩트 연결, 아티팩트 승격, `Write Authorization` 소비를 만들지 않습니다.

## 저장 효과

커밋 시 실행, 증거, 차단 사유, `Write Authorization` 소비, 아티팩트 연결 결과를 지속할 수 있습니다. 정확한 저장 효과와 아티팩트 승격 세부사항은 아래 저장 담당 문서가 담당합니다.

## 최소 유효 요청

이 예시는 이 메서드 문서 안에서 전제로 둔 스테이징된 핸들의 검증 출력을 기록합니다. 메서드 안의 전제: `staged_runprobe_001`은 만료되지 않았고 소비되지 않았으며 `proj_runprobe_001` / `task_runprobe_001`에 속합니다. 기록된 접점 출처는 `surface_run_probe`와 `surface_instance_run_probe_01`입니다. 이 전제는 이 문서의 예시 안에서만 성립하며 다른 메서드 예시를 재사용하지 않습니다.

```yaml
method: harness.record_run
params:
  envelope:
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    actor_kind: agent
    surface_id: surface_run_probe
    request_id: req_runprobe_001
    idempotency_key: idem_runprobe_001
    expected_state_version: 31
    dry_run: false
    locale: en-US
  task_id: task_runprobe_001
  change_unit_id: cu_runprobe_001
  kind: implementation
  run_id: null
  baseline_ref: baseline_runprobe_001
  write_authorization_id: null
  summary: "Search-result count validation passed."
  observed_changes:
    changed_paths: []
    product_file_write_observed: false
    sensitive_categories: []
    baseline_ref: baseline_runprobe_001
  artifact_inputs:
    - artifact_input_id: artifact_input_runprobe_001
      source_kind: staged_artifact
      staged_artifact_handle:
        handle_id: staged_runprobe_001
        project_id: proj_runprobe_001
        task_id: task_runprobe_001
        created_by_surface_id: surface_run_probe
        created_by_surface_instance_id: surface_instance_run_probe_01
        content_type: application/json
        sha256: sha256:example-runprobe
        size_bytes: 96
        redaction_state: none
        expires_at: "<future-expiration-timestamp>"
        consumed: false
      existing_artifact_ref: null
      relation_hint: "validation_report"
      claim: "Search-result count validation passed."
      expected_sha256: "sha256:example-runprobe"
      expected_size_bytes: 96
      redaction_state: none
  evidence_updates:
    - claim: "Search-result count validation passed."
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
  state_version: 32
  events:
    - event_id: evt_runprobe_001
      event_kind: run_recorded
run_summary:
  run_ref:
    record_kind: run
    record_id: run_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    state_version: 32
  kind: implementation
  summary: "Search-result count validation passed."
  observed_changes:
    changed_paths: []
    product_file_write_observed: false
    sensitive_categories: []
    baseline_ref: baseline_runprobe_001
  artifact_refs:
    - artifact_id: artifact_runprobe_report_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      display_name: "search-result-count-validation.json"
      content_type: application/json
      sha256: sha256:example-runprobe
      size_bytes: 96
      redaction_state: none
      availability: available
      created_by_run_ref:
        record_kind: run
        record_id: run_runprobe_001
        project_id: proj_runprobe_001
        task_id: task_runprobe_001
        state_version: 32
      created_by_surface_id: surface_run_probe
      created_by_surface_instance_id: surface_instance_run_probe_01
      storage_ref: "artifact-storage://search-result-count-validation"
registered_artifacts:
  - artifact_id: artifact_runprobe_report_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    display_name: "search-result-count-validation.json"
    content_type: application/json
    sha256: sha256:example-runprobe
    size_bytes: 96
    redaction_state: none
    availability: available
    created_by_run_ref:
      record_kind: run
      record_id: run_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      state_version: 32
    created_by_surface_id: surface_run_probe
    created_by_surface_instance_id: surface_instance_run_probe_01
    storage_ref: "artifact-storage://search-result-count-validation"
evidence_summary:
  status: sufficient
  completion_policy:
    evidence_required: true
    required_claims:
      - "Search-result count validation passed."
  coverage_items:
    - claim: "Search-result count validation passed."
      required_for_close: true
      coverage_state: supported
      supporting_refs:
        - record_kind: run
          record_id: run_runprobe_001
          project_id: proj_runprobe_001
          task_id: task_runprobe_001
          state_version: 32
      supporting_artifact_refs:
        - artifact_id: artifact_runprobe_report_001
          project_id: proj_runprobe_001
          task_id: task_runprobe_001
          display_name: "search-result-count-validation.json"
          content_type: application/json
          sha256: sha256:example-runprobe
          size_bytes: 96
          redaction_state: none
          availability: available
          created_by_run_ref:
            record_kind: run
            record_id: run_runprobe_001
            project_id: proj_runprobe_001
            task_id: task_runprobe_001
            state_version: 32
          created_by_surface_id: surface_run_probe
          created_by_surface_instance_id: surface_instance_run_probe_01
          storage_ref: "artifact-storage://search-result-count-validation"
      gap_refs: []
  artifact_refs:
    - artifact_id: artifact_runprobe_report_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      display_name: "search-result-count-validation.json"
      content_type: application/json
      sha256: sha256:example-runprobe
      size_bytes: 96
      redaction_state: none
      availability: available
      created_by_run_ref:
        record_kind: run
        record_id: run_runprobe_001
        project_id: proj_runprobe_001
        task_id: task_runprobe_001
        state_version: 32
      created_by_surface_id: surface_run_probe
      created_by_surface_instance_id: surface_instance_run_probe_01
      storage_ref: "artifact-storage://search-result-count-validation"
  updated_by_run_ref:
    record_kind: run
    record_id: run_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    state_version: 32
blocker_refs: []
state:
  project_id: proj_runprobe_001
  state_version: 32
  task_ref:
    record_kind: task
    record_id: task_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    state_version: 32
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Validate search-result count display."
  scope_summary: "Search-result count validation."
  non_goals:
    - "Changing search ranking."
  acceptance_criteria:
    - "Search results show the expected count."
  autonomy_boundary: "Stay within validation recording for search-result counts."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    state_version: 31
  baseline_ref: baseline_runprobe_001
  shaping_readiness: null
  pending_user_judgment_refs: []
  blocker_refs: []
  write_authority_summary: null
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display: null
```

## 담당 문서 링크

- 요청 래퍼, 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `RunSummary`, `EvidenceSummary`, `EvidenceCoverageItem`, `StateSummary`, 참조: [API 상태 스키마](schema-state.md).
- `ArtifactInput`, `StagedArtifactHandle`, `ArtifactRef`: [API 아티팩트 스키마](schema-artifacts.md).
- `Write Authorization`과 닫기 관련 증거 경계: [Core 모델](../core-model.md).
- 지원되는 값과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류, 우선순위, 응답 처리 경로, 아티팩트 입력 세부 값: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md), [아티팩트 입력 오류 세부사항](error-details.md#artifact-input-error-reason).
- 저장 효과와 아티팩트 승격: [저장 효과](../storage-effects.md), [아티팩트 저장소](../storage-artifacts.md).
