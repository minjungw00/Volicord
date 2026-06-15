<a id="harnessstage_artifact"></a>

# `harness.stage_artifact` 참조

## 담당하는 것

이 문서는 기준 범위의 `harness.stage_artifact` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 임시 스테이징 핸들을 만드는 동작
- 아티팩트 스테이징 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, `dry_run`, 거절 응답 스키마 본문
- `ArtifactInput`, `ArtifactRef`, `StagedArtifactHandle`, 값 집합, 오류 스키마 정의
- 저장 DDL, 저장 기록 레이아웃, 정확한 저장 효과, 아티팩트 생명주기, 보안 보장, Core 권한 의미
- 공개 오류 코드 의미, 공개 오류 우선순위, 공통 응답 분기 처리 경로

## 목적

`harness.stage_artifact`는 호출자가 제공한 안전한 아티팩트 바이트 또는 안전한 알림을 같은 프로젝트와 `Task`에 대한 임시 `StagedArtifactHandle`로 스테이징합니다.

스테이징은 입력 준비일 뿐입니다. 증거, 지속 아티팩트 연결, 수락, 잔여 위험, 닫기 준비 상태 효과는 관련 메서드와 저장소 담당 문서가 담당합니다.

## 필수 입력

- 유효한 `ToolEnvelope`. `idempotency_key`와 `expected_state_version`은 `null`일 수 있습니다.
- `task_id`, `display_name`, `content_type`, `redaction_state`, `safe_bytes_or_notice`, `expected_sha256`, `expected_size_bytes`, `relation_hint`.

## 접근 요구사항

요구사항:

- `VerifiedSurfaceContext.access_class=artifact_registration`
- `verified=true`
- 호환되는 `project_id`와 `task_id`
- `manual_artifact_attachment_supported=true`

서버는 확인된 로컬 접점에서 `created_by_surface_id`와 `created_by_surface_instance_id`를 기록합니다. 호출자는 이 값을 권한 근거로 제출하지 않습니다.

## 상태 버전 동작

성공한 스테이징 결과:

- Core 상태를 바꾸지 않습니다.
- `project_state.state_version`을 올리지 않습니다.
- `tool_invocations` 재실행 행을 만들지 않습니다.

거절과 `dry_run` 요청은 저장 효과가 없습니다.

## 성공 결과

아래 값을 담은 `StageArtifactResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=staging_created`
- 임시 `staged_artifact_handle`
- `expires_at`

결과에는 임시 핸들이 포함되며 지속 `ArtifactRef`는 포함되지 않습니다.

## 차단 결과

커밋된 차단 분기는 없습니다.

- 유효하지 않은 스테이징 요청은 Core 변경 전에 거절됩니다.
- 스테이징 가용성이나 역량 문제는 차단 사유를 만들지 않습니다.

## 거절 결과

아래 경우는 `ToolRejectedResponse`를 반환합니다.

- 유효하지 않은 요청 형태
- 체크섬 또는 크기 불일치
- 안전하지 않은 아티팩트 입력
- 지원하지 않는 가림 처리 상태
- Core 또는 로컬 접점 사용 불가
- 로컬 접근 불일치
- 아티팩트 등록 역량 부족

공개 오류 코드 의미, 우선순위, 거절 응답 처리 경로는 아래 오류 담당 문서가 담당합니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 스테이징 미리보기:

- `ToolDryRunResponse`를 반환합니다.
- `StageArtifactResult`를 반환하지 않습니다.
- 스테이징 핸들을 만들지 않습니다.

## 저장 효과

성공 시 임시 스테이징 결과만 만듭니다. 정확한 저장 효과와 아티팩트 생명주기 세부사항은 아래 저장 담당 문서가 담당합니다.

## 최소 유효 요청

```yaml
method: harness.stage_artifact
params:
  envelope:
    project_id: proj_trace_001
    task_id: task_trace_001
    actor_kind: agent
    surface_id: surface_artifact
    request_id: req_stage_trace_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: en-US
  task_id: task_trace_001
  display_name: "diagnostic_trace.log"
  content_type: text/plain
  redaction_state: none
  safe_bytes_or_notice: "Local trace sample captured for debugging."
  expected_sha256: null
  expected_size_bytes: null
  relation_hint: "diagnostic_log"
```

## 대표 응답

결과 분기(`StageArtifactResult`, 스테이징 생성):

```yaml
base:
  response_kind: result
  effect_kind: staging_created
  dry_run: false
  state_version: null
  events: []
staged_artifact_handle:
  handle_id: staged_trace_log_001
  project_id: proj_trace_001
  task_id: task_trace_001
  created_by_surface_id: surface_artifact
  created_by_surface_instance_id: surface_instance_trace_01
  content_type: text/plain
  sha256: sha256:example-trace
  size_bytes: 42
  redaction_state: none
  expires_at: "<future-expiration-timestamp>"
  consumed: false
expires_at: "<future-expiration-timestamp>"
```

## 담당 문서 링크

- 요청 래퍼, 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `StagedArtifactHandle`, `ArtifactInput`, `ArtifactRef`: [API 아티팩트 스키마](schema-artifacts.md).
- 지원되는 아티팩트 값과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류, 우선순위, 거절 응답 처리 경로: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md).
- 저장 효과와 아티팩트 생명주기: [저장 효과](../storage-effects.md), [아티팩트 저장소](../storage-artifacts.md).
