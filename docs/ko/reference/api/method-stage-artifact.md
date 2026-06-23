<a id="volicordstage_artifact"></a>

# `volicord.stage_artifact` 참조

## 담당하는 것

이 문서는 기준 범위의 `volicord.stage_artifact` 메서드 동작을 담당합니다.

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

`volicord.stage_artifact`는 호출자가 제공한 안전한 아티팩트 바이트 또는 안전한 알림을 같은 프로젝트와 `Task`에 대한 임시 `StagedArtifactHandle`로 스테이징합니다.

스테이징은 입력 준비일 뿐입니다. 증거, 지속 아티팩트 연결, 수락, 잔여 위험, 닫기 준비 상태 효과는 관련 메서드와 저장소 담당 문서가 담당합니다.

## 필수 입력

- 유효한 `ToolEnvelope`. `idempotency_key`와 `expected_state_version`은 `null`일 수 있습니다.
- `task_id`, `display_name`, `content_type`, `redaction_state`, `safe_bytes_or_notice`, `expected_sha256`, `expected_size_bytes`, `relation_hint`.

## 요청 스키마

이 메서드는 아래 최상위 `params` 요청 형태를 담당합니다. `envelope`는 [API 코어 스키마](schema-core.md#tool-envelope)의 공통 `ToolEnvelope`이며, 이 블록은 `ToolEnvelope` 필드를 다시 정의하지 않습니다.

이 메서드 소유 요청 블록에 표시된 모든 필드는 필드 참고가 명시적으로 선택 필드라고 표시하지 않는 한 `params`의 필수 멤버입니다. `T | null`은 멤버가 반드시 있어야 하며 JSON `null`을 담을 수 있다는 뜻입니다.

```yaml
StageArtifactRequest:
  envelope: ToolEnvelope
  task_id: string
  display_name: string
  content_type: string
  redaction_state: string
  safe_bytes_or_notice: string
  expected_sha256: string | null
  expected_size_bytes: integer | null
  relation_hint: string | null
```

중첩 형태 담당 문서:
- `redaction_state` 값은 [API 값 집합의 아티팩트 값](schema-value-sets.md#artifact-values)이 담당합니다.
- 결과 측 `StagedArtifactHandle` 형태는 [API 아티팩트 스키마](schema-artifacts.md#stagedartifacthandle)에 남습니다.

## 스테이징 입력 허용 기본값

이 메서드는 [아티팩트 저장소](../storage-artifacts.md)가 담당하는 기준 스테이징 기본값을 적용합니다.

- 반환되는 핸들은 스테이징 생성 24시간 뒤 만료됩니다.
- 저장되는 스테이징 아티팩트 본문 또는 안전한 알림은 10 MiB(10,485,760 bytes)로 제한됩니다.
- 저장되는 본문 바이트는 안전한 텍스트, JSON, Markdown, XML 또는 동등한 텍스트성 미디어 타입으로 제한됩니다.
- 바이너리 입력은 나중에 담당 문서가 프로필 조건부 안전 바이너리 본문 경로를 정의하기 전까지 안전한 텍스트 알림으로만 표현됩니다.
- 원시 비밀값은 저장하면 안 됩니다. 적용 가능한 경우 `redaction_state=secret_omitted` 또는 `redaction_state=blocked`인 안전한 알림을 사용합니다.

이 입력 허용 요구사항을 통과하지 못한 요청은 유효하지 않거나 안전하지 않은 아티팩트 입력에 대한 기존 거절 결과 동작을 사용합니다. 이 절은 저장소 담당 기본값을 적용하고 안내할 뿐입니다. 아티팩트 생명주기, 보존, `redaction_state` 값 의미, 본문 읽기 자격은 [아티팩트 저장소](../storage-artifacts.md)와 [API 값 집합](schema-value-sets.md#artifact-values)이 담당합니다.

## 접근 요구사항

요구사항:

- `access_class=artifact_registration`인 서버 파생 `VerifiedSurfaceContext`
- 호환되는 `project_id`와 `task_id`
- `manual_artifact_attachment_supported=true`

서버는 파생된 `VerifiedSurfaceContext`에서 `created_by_surface_id`와 `created_by_surface_instance_id`를 기록합니다. 호출자는 이 값을 권한 근거로 제출하지 않습니다.

## 상태 버전 동작

성공한 스테이징 결과:

- Core 상태를 바꾸지 않습니다.
- `project_state.state_version`을 증가시키지 않습니다.
- 호출이 관찰한 현재 프로젝트 전체 `project_state.state_version`을 `base.state_version`에 보고합니다.
- `tool_invocations` 재실행 행을 만들지 않습니다.

거절과 `dry_run` 요청은 저장 효과가 없습니다.

## 메서드 결과 필드

`StageArtifactResult`는 성공한 스테이징 작업에 대한 메서드별 결과 분기입니다. 이 결과는 `base: ToolResultBase`와 아래 메서드 소유 최상위 필드를 담습니다.

| 필드 | 결과 필드 의미 |
|---|---|
| `base` | 공통 결과 메타데이터입니다. `events`를 포함한 `ToolResultBase` 형태는 [API 코어 스키마](schema-core.md#common-response)가 담당합니다. 성공한 스테이징은 `base.response_kind=result`, `base.effect_kind=staging_created`, 호출이 관찰한 현재 프로젝트 전체 `project_state.state_version`을 담은 `base.state_version`, `events: []`를 사용합니다. |
| `staged_artifact_handle` | 스테이징된 안전한 바이트 또는 안전한 알림에 대한 임시 `StagedArtifactHandle`입니다. 형태는 [API 아티팩트 스키마](schema-artifacts.md#stagedartifacthandle)가 담당합니다. |
| `expires_at` | 임시 핸들의 만료 시각입니다. `staged_artifact_handle.expires_at`과 같은 값을 나타내며, 생명주기, 만료, 소비 세부사항은 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다. |

`StageArtifactResult`에는 지속 `ArtifactRef`, 실행 요약, 증거 요약, 차단 사유 참조, 현재 상태 스냅샷이 포함되지 않습니다.

## 성공 결과

아래 값을 담은 `StageArtifactResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=staging_created`
- 호출이 관찰한 현재 프로젝트 전체 버전으로 설정된 `base.state_version`
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

아래 예시는 메서드 안에서만 성립하도록 짧게 구성했습니다. 대표 응답은 전체 `StageArtifactResult` 최상위 형태와 하나의 `StagedArtifactHandle`을 보여 줍니다.

## 최소 유효 요청

```yaml
method: volicord.stage_artifact
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
  state_version: 42
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
