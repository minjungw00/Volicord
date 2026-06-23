# API 아티팩트 스키마

이 문서는 기준 범위의 아티팩트 형태 API 스키마를 담당합니다. 스키마는 요청과 응답 형태를 정의하지만 로컬 파일 접근 권한, 아티팩트 본문, 저장소 행, 증거 충분성을 정의하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- `ArtifactRef`
- `ArtifactInput`
- `StagedArtifactHandle`
- 스테이징된 아티팩트 입력과 기존 아티팩트 입력의 구분
- 스테이징, 연결, 본문 읽기 참조에 쓰이는 아티팩트 형태 요청/응답 필드
- 스키마 검증에 필요한 아티팩트 참조 제약
- 아티팩트 형태 API 응답에 나타나는 가림 처리, 가용성, 무결성, 체크섬, 크기 필드

이 문서는 담당하지 않습니다.

- 아티팩트 저장소 배치, 스테이징 기록, 승격 지속 효과, 보존, 본문 읽기 저장소 자격: [아티팩트 저장소](../storage-artifacts.md)
- `volicord.stage_artifact`, `volicord.record_run` 메서드 동작: [아티팩트 스테이징 메서드](method-stage-artifact.md), [실행 기록 메서드](method-record-run.md), [API 메서드](methods.md)
- 지원되는 아티팩트 값 집합: [API 값 집합](schema-value-sets.md)
- 증거 충분성: [Core 모델](../core-model.md), [API 상태 스키마](schema-state.md)
- 접근, 차단, 격리에 대한 보안 주장: [보안](../security.md)

## 경계

아티팩트 스키마는 호출자가 보낸 경로 문자열을 권한으로 만들지 않습니다.

이 문서는 아티팩트 관련 메서드와 담당 문서가 쓰는 요청/응답 형태를 설명합니다.

담당 문서:
- 메서드 검증, 스테이징, 승격, 연결 동작: [API 메서드](methods.md)가 안내하는 메서드 담당 문서
- 본문 읽기 자격과 아티팩트 생명주기: [아티팩트 저장소](../storage-artifacts.md)

## `ArtifactRef`

`ArtifactRef`는 공개 아티팩트 참조와 메타데이터 형태입니다.

```yaml
ArtifactRef:
  artifact_id: string
  project_id: string
  task_id: string
  display_name: string
  content_type: string | null
  sha256: string | null
  size_bytes: integer | null
  integrity_status: string
  redaction_state: string
  availability: string
  created_by_run_ref: StateRecordRef | null
  created_by_surface_id: string | null
  created_by_surface_instance_id: string | null
  storage_ref: string | null
```

`ArtifactRef`는 참조와 메타데이터 형태입니다. 이 값만으로 아티팩트 본문을 읽을 수 있는 것도 아니고, 그 본문이 닫기에 충분한 증거라는 뜻도 아닙니다.

`artifact_id`, `project_id`, `task_id`, `created_by_surface_id`, `created_by_surface_instance_id`, `storage_ref`는 불투명 식별자입니다. `display_name`은 자유 형식 표시 문자열입니다. `content_type`은 알 때의 미디어 타입 메타데이터이고, `sha256`은 알 때의 체크섬 문자열이며, `size_bytes`는 알 때의 바이트 크기 메타데이터입니다. `integrity_status`, `redaction_state`, `availability`는 [아티팩트 값](schema-value-sets.md#artifact-values)이 담당하는 제어 값 문자열입니다.

`integrity_status`는 필수입니다. `content_type`, `sha256`, `size_bytes`가 null이면 그 사실을 모른다는 뜻이며, 비어 있음, 0, 기본값이 아닙니다. 빠진 사실을 빈 해시, 0바이트 크기, 만들어 낸 콘텐츠 타입으로 표현하면 안 됩니다. 실제 0바이트 아티팩트는 `size_bytes: 0`과 빈 바이트의 SHA-256인 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`를 가집니다.

`integrity_status=verified`에서는 `content_type`이 비어 있지 않고, `sha256`이 유효한 소문자 16진수 SHA-256 문자열이며, `size_bytes`가 음수가 아니어야 합니다. 권한을 지니는 증거와 닫기 사용은 [아티팩트 저장소](../storage-artifacts.md)의 현재 바이트 검증도 요구합니다. `integrity_status=corrupt`는 알려진 불일치나 유효하지 않은 `verified` 사실 관계를 기록합니다. 본문 바이트가 없거나, 읽을 수 없거나, 사용할 수 없거나, 사용에 부적합한 상태는 세 번째 무결성 값이 아니라 `availability`로 표현합니다.

## `StagedArtifactHandle`

`StagedArtifactHandle`은 `volicord.stage_artifact` 결과와 연결되는 임시 핸들 형태입니다. 지속 아티팩트의 `ArtifactRef` 형태가 아닙니다.

```yaml
StagedArtifactHandle:
  handle_id: string
  project_id: string
  task_id: string
  created_by_surface_id: string
  created_by_surface_instance_id: string
  content_type: string
  sha256: string
  size_bytes: integer
  redaction_state: string
  expires_at: string
  consumed: boolean
```

호출자는 `created_by_surface_id`나 `created_by_surface_instance_id`를 권한 주장으로 제출하지 않습니다. 스테이징 핸들의 생명주기, 출처 검증, 만료, 승격은 [아티팩트 저장소](../storage-artifacts.md)와 메서드 담당 문서가 담당합니다.

`handle_id`, `project_id`, `task_id`, `created_by_surface_id`, `created_by_surface_instance_id`는 불투명 식별자입니다. `content_type`은 미디어 타입 메타데이터이고, `sha256`은 체크섬 문자열이며, `redaction_state`는 제어 값 문자열입니다.

## `ArtifactInput`

`ArtifactInput`은 실행 기록이나 증거 출력에 아티팩트 링크를 받는 메서드의 요청 측 형태입니다.

```yaml
ArtifactInput:
  artifact_input_id: string
  source_kind: string
  staged_artifact_handle: StagedArtifactHandle | null
  existing_artifact_ref: ArtifactRef | null
  relation_hint: string | null
  claim: string | null
  expected_sha256: string | null
  expected_size_bytes: integer | null
  redaction_state: string | null
```

각 입력에서는 출처 필드 하나만 채우고 다른 출처 필드는 `null`이어야 합니다. `ArtifactInput.source_kind`는 어느 출처 필드가 적용되는지 고르며, 지원되는 출처 종류 값과 값 의미는 [아티팩트 값](schema-value-sets.md#artifact-values)이 담당합니다.

`artifact_input_id`는 요청 안에서 유효한 불투명 입력 식별자입니다. `relation_hint`와 `claim`은 자유 형식 표시 또는 주장 문자열입니다. `expected_sha256`은 체크섬 문자열입니다. `redaction_state`는 값이 있을 때 제어 값 문자열입니다.

형태 규칙:
- `staged_artifact_handle`이 채워지면 `existing_artifact_ref`는 `null`입니다.
- `existing_artifact_ref`가 채워지면 `staged_artifact_handle`은 `null`입니다.

호출자가 준 경로, 로그, 캡처 주장, 로컬 파일 참조는 아티팩트 권한이 아닙니다.

## 참조 제약

`ArtifactInput[]`은 입력마다 아티팩트 출처 형태 하나를 고릅니다. 공개 API 요청에 두 번째 요청 수준 접근 등급을 더하지 않습니다.

출처 필드 형태 오류의 공개 오류 의미와 응답 처리 경로는 [API 오류 코드](error-codes.md)와 [API 오류 처리 경로](error-routing.md)가 담당합니다. 스테이징된 아티팩트 핸들 검증, 승격, 본문 읽기 자격, 지속 연결은 [아티팩트 저장소](../storage-artifacts.md)와 메서드 담당 문서가 담당합니다.

## 관련 담당 문서

- [아티팩트 스테이징 메서드](method-stage-artifact.md), [실행 기록 메서드](method-record-run.md), [API 메서드](methods.md): 아티팩트 관련 메서드 동작.
- [아티팩트 저장소](../storage-artifacts.md): 스테이징, 승격, 지속 연결, 본문 읽기 생명주기.
- [API 값 집합](schema-value-sets.md): `ArtifactInput.source_kind`, `redaction_state`, 가용성, 관련 값.
- [API 상태 스키마](schema-state.md): `ArtifactRef`를 언급하는 증거 요약.
- [런타임 경계](../runtime-boundaries.md)와 [보안](../security.md): 로컬 접근과 비주장 경계.
