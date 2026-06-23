<a id="volicordrecord_run"></a>

# `volicord.record_run` 참조

## 담당하는 것

이 문서는 기준 범위의 `volicord.record_run` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 실행 기록, 현재 닫기 근거 갱신, 증거 갱신, 차단 사유 갱신, 아티팩트 승격 메서드 동작
- 실행 기록 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, `dry_run`, 거절 응답 스키마 본문
- 상태, 아티팩트, 값 집합, 오류의 중첩 스키마 정의
- Core의 증거 의미, Core 권한 의미, 저장 DDL, 저장 기록 레이아웃, 정확한 저장 효과, 아티팩트 생명주기, 보안 보장
- 공개 오류 코드 의미, 공개 오류 우선순위, 기계 판독용 오류 세부사항, 공통 응답 분기 처리 경로

## 목적

`volicord.record_run`은 아래 작업을 기록합니다.

- 구체화 작업
- 직접 응답 또는 결과
- 구현 작업

이 메서드는 현재 닫기 근거와 간결한 증거 범위를 갱신하고, 제품 쓰기를 기록할 때 호환되는 `Write Authorization`을 소비하며, 기존 아티팩트를 연결하고, 허용되는 경우 적격 스테이징 핸들을 지속 `ArtifactRef`로 승격할 수도 있습니다.

## 필수 입력

- 유효한 `ToolEnvelope`. 커밋되는 `dry_run`이 아닌 요청에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `task_id`, `change_unit_id`, `kind`, `run_id`, `baseline_ref`, `write_authorization_id`, `summary`, `observed_changes`, `artifact_inputs`, `evidence_updates`, `close_assessment`.
- 제품 쓰기 실행은 `volicord.prepare_write`가 만든 호환되는 `status=active` `Write Authorization`이 필요합니다.
- 새 아티팩트 바이트는 이미 유효한 `StagedArtifactHandle`로 표현되어 있어야 합니다. `volicord.record_run`은 새 바이트를 스테이징하지 않습니다.

## 요청 스키마

이 메서드는 아래 최상위 `params` 요청 형태를 담당합니다. `envelope`는 [API 코어 스키마](schema-core.md#tool-envelope)의 공통 `ToolEnvelope`이며, 이 블록은 `ToolEnvelope` 필드를 다시 정의하지 않습니다.

이 메서드 소유 요청 블록에 표시된 모든 필드는 필드 참고가 명시적으로 선택 필드라고 표시하지 않는 한 `params`의 필수 멤버입니다. `T | null`은 멤버가 반드시 있어야 하며 JSON `null`을 담을 수 있다는 뜻입니다.

```yaml
RecordRunRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string
  kind: string
  run_id: string | null
  baseline_ref: string
  write_authorization_id: string | null
  summary: string
  observed_changes: ObservedChanges
  artifact_inputs: ArtifactInput[]
  evidence_updates: EvidenceCoverageItem[]
  close_assessment: CloseAssessmentInput | null

CloseAssessmentInput:
  result_summary: string
  result_refs: StateRecordRef[]
  residual_risks: ResidualRiskInput[]
  sensitive_categories: string[]
  recovery_constraints: string[]

ResidualRiskInput:
  summary: string
  consequence: string
  acceptance_required: boolean
  source_refs: StateRecordRef[]
```

중첩 형태 담당 문서:
- `observed_changes`와 `evidence_updates`는 `ObservedChanges`와 `EvidenceCoverageItem`을 사용합니다. 이 형태는 [API 상태 스키마](schema-state.md#evidence-and-run-snapshot-shapes)가 담당합니다.
- `close_assessment.result_refs`와 `ResidualRiskInput.source_refs`는 [API 상태 스키마](schema-state.md#state-references)가 담당하는 `StateRecordRef`를 사용합니다.
- `CurrentCloseBasis`와 커밋된 `ResidualRisk` 출력 형태는 [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)가 담당합니다. `ResidualRiskInput`에는 호출자 권한의 `risk_id`가 없습니다. Core는 새 현재 닫기 근거를 커밋할 때 불투명 `risk_id` 값을 생성합니다.
- `artifact_inputs`는 `ArtifactInput[]`을 사용합니다. `ArtifactInput`, `StagedArtifactHandle`, `ArtifactRef` 형태는 [API 아티팩트 스키마](schema-artifacts.md#artifactinput)가 담당합니다.
- `kind`, 아티팩트 출처 값, `redaction_state`, 증거 범위 값은 [API 값 집합](schema-value-sets.md)이 담당합니다.

경로와 접근 참고:
- `observed_changes.changed_paths` 항목은 `Product Repository` API 제품 경로입니다. `Product Repository` 경로 정규화는 [런타임 경계](../runtime-boundaries.md#product-repository-api-path-normalization)가 담당합니다.
- `ArtifactInput[]`와 스테이징 핸들은 두 번째 요청 수준 접근 등급을 만들지 않습니다. 요청 수준 접근 등급은 파생된 `VerifiedSurfaceContext`의 접근 등급 하나로 유지됩니다.

닫기 평가 참조 규칙:
- 호출자가 제공한 `close_assessment.result_refs`와 `ResidualRiskInput.source_refs`는 담당 문서가 다른 종류를 명시적으로 추가하지 않는 한 `record_kind=run`, `artifact`, `evidence_summary`, `change_unit`으로 제한됩니다.
- 담당 문서가 명시적으로 추가하지 않는 한 이 메서드는 호출자가 제공한 `project_state`, `write_authorization`, `user_judgment`, `blocker`, `task_event`, `local_surface_registration`, `task` 참조를 닫기 근거에서 거절하거나 제외합니다.
- 받아들인 모든 참조는 존재해야 하고 같은 프로젝트와 `Task`에 속해야 합니다. 아티팩트 참조는 `Task`에 연결되어 있고 `integrity_status=verified`로 현재 바이트 검증을 통과해야 합니다. 증거 참조는 현재 `Task` 증거 요약을 식별해야 합니다. 현재 닫기 근거 결과 참조로 쓰이는 실행 기록 참조는 현재 `Task`, 현재 적용 Change Unit, 현재 범위 리비전, 호환되는 기준선, 기록된 상태와 호환되는 기록된 현재 실행 기록을 식별해야 합니다.
- 이력 실행 기록 참조는 이 새 현재 실행 기록이 이력의 `verified` 아티팩트나 증거를 명시적으로 재사용하고 그 재사용을 커밋된 증거나 닫기 평가에 기록하지 않는 한 닫기 근거 용도에서는 감사 기록입니다.
- Core는 `CurrentCloseBasis`에 기준 참조를 저장하며 호출자가 보낸 `state_version` 메타데이터를 권한으로 보존하지 않습니다.
- Core는 기준 닫기 근거를 만들면서 현재 실행 기록, 현재 Change Unit, 현재 EvidenceSummary 참조를 추가할 수 있습니다.

## 접근 요구사항

요구사항:

- `access_class=run_recording`인 서버 파생 `VerifiedSurfaceContext`

`source_kind=staged_artifact`인 경우:

- 현재 파생된 `VerifiedSurfaceContext.surface_id`가 스테이징 핸들의 기록된 출처와 일치해야 합니다.
- 현재 파생된 `VerifiedSurfaceContext.surface_instance_id`가 스테이징 핸들의 기록된 출처와 일치해야 합니다.

기록된 출처는 스테이징 시점의 파생된 `VerifiedSurfaceContext`에서 캡처된 것입니다. 이 메서드는 호출자가 제출한 출처를 권한 근거로 받아들이지 않고, 그 기록된 출처를 현재 파생된 맥락과 비교합니다.

비주장:

- `ArtifactInput[]`는 `artifact_registration`을 추가하지 않습니다.
- 접점 간 스테이징 핸들 전달은 기준 범위 밖입니다.

## 상태 버전 동작

호환되는 커밋 결과는 `project_state.state_version`을 정확히 한 번 올립니다.

호환되는 커밋 결과는 선택된 `Task.close_basis_revision`을 정확히 한 번 증가시킵니다. `close_assessment`가 `null`이 아니면 커밋은 커밋된 현재 실행 기록, 평가 필드, 생성된 잔여 위험 ID, 현재 Task, 현재 적용 Change Unit, 선택된 현재 범위 리비전, 호환되는 기준선에서 새 `CurrentCloseBasis`를 만듭니다. `close_assessment=null`이면 커밋된 실행 기록이 현재 닫기 근거를 만들지 않음을 명시하며, 기존 현재 닫기 근거는 오래되거나 없어집니다.

빈 `close_assessment.residual_risks` 목록은 현재 결과에 식별된 잔여 위험이 없다는 명시적 의미입니다. Core는 커밋된 `null`이 아닌 평가에 대해서만 불투명 `risk_id` 값을 생성합니다. `dry_run`은 지속 `risk_id` 값을 예약하지 않습니다.

결과 `CurrentCloseBasis` 안의 민감 동작 요구사항은 커밋된 실행 기록과 소비된 `Write Authorization`에서 Core가 파생합니다. `close_assessment.sensitive_categories` 안의 범주만 담은 호출자 입력은 표시 맥락에는 기여할 수 있지만 민감 승인 요구사항을 만들거나, 만족하거나, 지울 수 없습니다.

실행 기록, 현재 닫기 근거, 증거 갱신, 아티팩트 연결 또는 승격, `Write Authorization` 소비, 리비전 변경은 결과가 커밋될 때 원자적으로 커밋됩니다.

제품 쓰기 기록이 `Write Authorization`을 소비하려면 아래 조건을 모두 만족해야 합니다.

- 소비 직전 현재 `project_state.state_version`이 `WriteAuthorization.basis_state_version`과 같습니다.
- 권한이 유효 만료 규칙, 즉 저장된 `expires_at`과 `created_at + 15 minutes` 중 더 이른 시점에 따라 만료되지 않았습니다.
- `Product Repository` 경로 정규화 뒤의 관찰된 변경 경로가 권한 부여된 시도와 호환됩니다.

`volicord.prepare_write`가 만든 `Write Authorization`은 사이에 다른 프로젝트 상태 변경이 없으면 생성 직후 오래되지 않습니다. 예를 들어 `volicord.prepare_write`가 버전 `19`에서 버전 `20`으로 커밋하면 현재 `project_state.state_version`과 `WriteAuthorization.basis_state_version`이 모두 `20`인 동안 `volicord.record_run`이 그 권한을 소비할 수 있습니다.

오래된 `expected_state_version`과 오래된 `Write Authorization` 근거는 `Write Authorization`을 소비하기 전에 거절됩니다. 오래된 `WriteAuthorization.basis_state_version`은 같은 권한이 함께 만료되었더라도 더 높은 우선순위의 `STATE_VERSION_CONFLICT` 경로를 유지합니다.

만료는 문자열 사전식 비교가 아니라 파싱한 UTC 타임스탬프로 계산합니다. 만료된 권한은 절대 소비되지 않습니다. 만료된 권한 사용은 `ToolError.details.authorization_reason=expired`와 함께 `WRITE_AUTHORIZATION_INVALID`를 반환합니다.

## 메서드 결과 필드

`RecordRunResult`는 커밋된 실행 기록 작업에 대한 메서드별 결과 분기입니다. 이 결과는 `base: ToolResultBase`와 아래 메서드 소유 최상위 필드를 담습니다.

| 필드 | 결과 필드 의미 |
|---|---|
| `base` | 공통 결과 메타데이터입니다. `events`를 포함한 `ToolResultBase` 형태는 [API 코어 스키마](schema-core.md#common-response)가 담당합니다. 커밋된 `RecordRunResult` 분기는 `base.response_kind=result`와 `base.effect_kind=core_committed`를 사용합니다. `base.events[].event_kind`가 있을 때 그 값은 불투명한 예시용 분류 문자열입니다. |
| `run_summary` | 기록된 Run의 `RunSummary`입니다. `RunSummary.kind`는 요청의 `kind`와 대응하며, 지원되는 실행 종류 값은 [API 값 집합](schema-value-sets.md#method-local-values)이 담당합니다. |
| `registered_artifacts` | 이 실행 결과가 만들거나 연결한 지속 아티팩트 참조의 `ArtifactRef[]`입니다. `ArtifactRef` 형태는 [API 아티팩트 스키마](schema-artifacts.md#artifactref)가 담당하고, 승격과 연결 생명주기 세부사항은 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다. |
| `evidence_summary` | 이 실행 결과가 갱신한 증거 범위의 `EvidenceSummary | null`입니다. 실행이 증거 갱신을 기록하지 않으면 `null`입니다. 형태는 [API 상태 스키마](schema-state.md#evidence-and-run-snapshot-shapes)가 담당하고, 증거 권한 의미는 [Core 모델](../core-model.md#9-evidence-and-run-authority)이 담당합니다. |
| `current_close_basis` | 이 실행이 기록된 뒤의 `CurrentCloseBasis | null`입니다. `null`이 아니면 이 실행이 현재 닫기 근거를 만들었다는 뜻입니다. `null`이면 이 실행이 현재 닫기 근거를 만들지 않았다는 뜻입니다. 형태는 [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)가 담당합니다. |
| `blocker_refs` | 이 결과 때문에 커밋되었거나 계속 관련되는 실행 또는 증거 관련 차단 사유의 `StateRecordRef[]`입니다. |
| `state` | 실행이 기록된 뒤의 현재 `StateSummary`입니다. `Write Authorization` 소비 뒤의 `write_authority_summary`를 포함한 중첩 상태 필드는 [API 상태 스키마](schema-state.md)가 담당합니다. |

중첩된 `StateRecordRef`, `RunSummary`, `ObservedChanges`, `EvidenceSummary`, `EvidenceCoverageItem`, `StateSummary`, `ArtifactRef` 필드 본문은 위에 연결된 스키마 담당 문서에 둡니다. 스테이징 핸들 소비, 아티팩트 승격, 증거 갱신, 재실행 행, `Write Authorization` 소비를 포함한 정확한 지속 효과는 [저장 효과](../storage-effects.md)와 [아티팩트 저장소](../storage-artifacts.md)에 둡니다.

## 성공 결과

아래 값을 담은 `RecordRunResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `run_summary`
- 모든 `registered_artifacts`
- 갱신된 `evidence_summary`
- 만들어진 경우 `current_close_basis`, 아니면 `null`
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
- 만료된 `Write Authorization`
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

오래된 `Write Authorization` 근거에서는 소비 전에 거절되며 Run, 증거 갱신, 아티팩트 연결, 아티팩트 승격, 이벤트, 재실행 행, `project_state.state_version` 증가를 만들지 않습니다.

만료된 `Write Authorization`에서는 소비 전에 거절되며 Run, 이벤트, 재실행 행, 아티팩트 승격, 증거 갱신, 권한 소비, `project_state.state_version` 증가를 만들지 않습니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 미리보기:

- `ToolDryRunResponse`를 반환합니다.
- Run, 현재 닫기 근거, 잔여 위험 ID, 증거 갱신, 차단 사유 갱신, 아티팩트 연결, 아티팩트 승격, `Write Authorization` 소비를 만들지 않습니다.

## 저장 효과

커밋 시 실행, 현재 닫기 근거, 증거, 차단 사유, `Write Authorization` 소비, 아티팩트 연결 결과를 지속할 수 있습니다. 정확한 저장 효과와 아티팩트 승격 세부사항은 아래 저장 담당 문서가 담당합니다.

아래 예시는 메서드 안에서만 성립하도록 짧게 구성했습니다. 대표 응답은 커밋된 실행, 승격된 아티팩트 참조, 갱신된 증거 요약, 차단 사유 참조, 상태 버전, 현재 상태 스냅샷을 보여 주는 데 필요한 필드로 축약했습니다.

## 최소 유효 요청

이 예시는 이 메서드 문서 안에서 전제로 둔 스테이징된 핸들의 검증 출력을 기록합니다. 메서드 안의 전제: `staged_runprobe_001`은 만료되지 않았고 소비되지 않았으며 `proj_runprobe_001` / `task_runprobe_001`에 속합니다. 스테이징 시점에 캡처된 기록된 접점 출처는 `surface_run_probe`와 `surface_instance_run_probe_01`입니다. 이 전제는 이 문서의 예시 안에서만 성립하며 다른 메서드 예시를 재사용하지 않습니다.

```yaml
method: volicord.record_run
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
        sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
        size_bytes: 96
        redaction_state: none
        expires_at: "<future-expiration-timestamp>"
        consumed: false
      existing_artifact_ref: null
      relation_hint: "validation_report"
      claim: "Search-result count validation passed."
      expected_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
      expected_size_bytes: 96
      redaction_state: none
  evidence_updates:
    - claim: "Search-result count validation passed."
      required_for_close: true
      coverage_state: supported
      supporting_refs: []
      supporting_artifact_refs: []
      gap_refs: []
  close_assessment:
    result_summary: "Search-result count validation passed."
    result_refs: []
    residual_risks: []
    sensitive_categories: []
    recovery_constraints: []
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
      sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
      size_bytes: 96
      integrity_status: verified
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
    sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
    size_bytes: 96
    integrity_status: verified
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
          sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
          size_bytes: 96
          integrity_status: verified
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
      sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
      size_bytes: 96
      integrity_status: verified
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
current_close_basis:
  close_basis_revision: 4
  scope_revision: 2
  task_id: task_runprobe_001
  change_unit_id: cu_runprobe_001
  baseline_ref: baseline_runprobe_001
  result_summary: "Search-result count validation passed."
  result_refs:
    - record_kind: run
      record_id: run_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      state_version: 32
  evidence_summary_ref: null
  residual_risks: []
  sensitive_categories: []
  sensitive_action_requirements: []
  recovery_constraints: []
  source_run_ref:
    record_kind: run
    record_id: run_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    state_version: 32
  updated_at: "<example-updated-at>"
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
- `RunSummary`, `EvidenceSummary`, `EvidenceCoverageItem`, `CurrentCloseBasis`, `ResidualRisk`, `StateSummary`, 참조: [API 상태 스키마](schema-state.md).
- `ArtifactInput`, `StagedArtifactHandle`, `ArtifactRef`: [API 아티팩트 스키마](schema-artifacts.md).
- `Write Authorization`과 닫기 관련 증거 경계: [Core 모델](../core-model.md).
- `Product Repository` 경로 정규화: [런타임 경계](../runtime-boundaries.md#product-repository-api-path-normalization).
- 지원되는 값과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류, 우선순위, 응답 처리 경로, 아티팩트 입력 세부 값: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md), [아티팩트 입력 오류 세부사항](error-details.md#artifact-input-error-reason).
- 저장 효과와 아티팩트 승격: [저장 효과](../storage-effects.md), [아티팩트 저장소](../storage-artifacts.md).
