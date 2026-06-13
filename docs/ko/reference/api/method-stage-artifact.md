<a id="harnessstage_artifact"></a>

# `harness.stage_artifact` 참조

## 담당하는 것

이 문서는 기준 범위의 `harness.stage_artifact` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 공유 계정 데이터 내보내기 확인 시나리오의 최소 요청과 대표 응답
- 메서드 수준 저장 효과 요약과 저장 담당 문서 링크

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`의 공통 스키마 본문
- 상태, 아티팩트, 사용자 판단, 값 집합, 오류의 중첩 스키마 정의
- 저장 DDL, 저장 기록 레이아웃, 아티팩트 생명주기, 보안 보장, Core 제품 의미

## 목적

호출자가 제공한 안전한 아티팩트 바이트 또는 안전한 알림을 같은 프로젝트와 `Task`에 대한 임시 `StagedArtifactHandle`로 스테이징합니다.

스테이징은 입력 준비일 뿐입니다. 증거, 지속 아티팩트 연결, 수락, 잔여 위험, 닫기 준비 상태 효과는 관련 메서드와 저장소 담당 문서가 담당합니다.

## 필수 입력

- `ToolEnvelope`: `project_id`, `task_id`, `surface_id`, `request_id`, `dry_run`이 필요합니다. `idempotency_key`와 `expected_state_version`은 `null`일 수 있습니다.
- `task_id`, `display_name`, `content_type`, `redaction_state`, `safe_bytes_or_notice`, `expected_sha256`, `expected_size_bytes`, `relation_hint`.

## 접근 요구사항

조건:

- `VerifiedSurfaceContext.access_class=artifact_registration`입니다.
- `verified=true`입니다.
- `project_id`와 `task_id`가 호환됩니다.
- `manual_artifact_attachment_supported=true`입니다.

결과:

- 서버는 확인된 로컬 접점에서 `created_by_surface_id`와 `created_by_surface_instance_id`를 기록합니다.

비주장:

- 호출자는 이 값을 권한 근거로 제출하지 않습니다.

## 상태 버전 동작

성공한 스테이징 결과의 효과:

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

- 유효하지 않은 요청 형태.
- 체크섬 또는 크기 불일치.
- 안전하지 않은 아티팩트 입력.
- 지원하지 않는 가림 처리 상태.
- Core 또는 로컬 접점 사용 불가.
- 로컬 접근 불일치.
- 아티팩트 등록 역량 부족.

공개 오류 코드 의미는 [API 오류 코드](error-codes.md)가 담당합니다. 공개 오류 우선순위는 [API 오류 우선순위](error-precedence.md)가 담당합니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 스테이징 미리보기:

- `ToolDryRunResponse`를 반환합니다.
- `StageArtifactResult`를 반환하지 않습니다.

분기 형태는 [API 코어 스키마](schema-core.md)가 담당하고, 스테이징 효과 없음 의미는 [저장 효과](../storage-effects.md)와 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다.

## 저장 효과

성공 시 임시 스테이징 결과만 만듭니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당하고, 아티팩트 생명주기 세부사항은 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다.

아티팩트 데이터 예시:

스테이징할 아티팩트는 안정적인 제품 테스트 출력입니다. `harness.record_run`은 증거를 기록할 때 임시 스테이징 핸들을 소비할 수 있습니다. 증거 생성은 `harness.record_run`이 담당하고, 아티팩트 승격은 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다.

```yaml
artifact:
  kind: test_log
  name: account_export_confirmation_test.log
  description: "계정 데이터 내보내기 확인 테스트 출력."
staged_artifact_handle: staged_artifact_account_export_test_log_001
expires_at: "<future-expiration-timestamp>"
```

## 최소 유효 요청

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
expires_at: "<future-expiration-timestamp>"
```

## 담당 문서 링크

- 요청 래퍼, 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `StagedArtifactHandle`, `ArtifactInput`, `ArtifactRef`: [API 아티팩트 스키마](schema-artifacts.md).
- 지원되는 아티팩트 값과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md).
- 저장 효과와 아티팩트 생명주기: [저장 효과](../storage-effects.md), [아티팩트 저장소](../storage-artifacts.md).
