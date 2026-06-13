# API 아티팩트 스키마

이 문서는 기준 범위의 아티팩트 형태 API 스키마를 담당합니다. 스키마는 요청과 응답 형태를 정의하지만 로컬 파일 접근 권한, 아티팩트 본문, 저장소 행, 증거 충분성을 만들지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- `ArtifactRef`
- `ArtifactInput`
- `StagedArtifactHandle`
- 스테이징된 아티팩트 입력과 기존 아티팩트 입력의 구분
- 스테이징, 연결, 본문 읽기 참조에 쓰이는 아티팩트 형태 요청/응답 필드
- 스키마 검증에 필요한 아티팩트 참조 제약
- 아티팩트 형태 API 응답에 나타나는 가림 처리, 가용성, 체크섬, 크기 필드

이 문서는 담당하지 않습니다.

- 아티팩트 저장소 배치, 스테이징 기록, 승격 지속 효과, 보존, 본문 읽기 저장소 자격: [아티팩트 저장소](../storage-artifacts.md)
- `harness.stage_artifact`, `harness.record_run` 메서드 동작: [아티팩트 스테이징 메서드](method-stage-artifact.md), [실행 기록 메서드](method-record-run.md), [API 메서드](methods.md)
- 지원되는 아티팩트 값 집합: [API 값 집합](schema-value-sets.md)
- 증거 충분성: [Core 모델](../core-model.md), [API 상태 스키마](schema-state.md)
- 접근, 차단, 격리에 대한 보안 주장: [보안](../security.md)

## 경계

아티팩트 스키마는 호출자가 보낸 경로 문자열을 권한으로 만들지 않습니다.

이 문서는 아티팩트 관련 메서드와 담당 문서가 쓰는 요청/응답 형태를 설명합니다.

담당 문서:
- 검증, 스테이징, 승격, 연결: [API 메서드](methods.md)가 안내하는 메서드 담당 문서
- 본문 읽기 자격과 아티팩트 생명주기: [아티팩트 저장소](../storage-artifacts.md)

## `ArtifactRef`

`ArtifactRef`는 아티팩트 담당 문서가 이미 등록한 지속 아티팩트를 가리키는 공개 포인터입니다.

```yaml
ArtifactRef:
  artifact_id: string
  project_id: string
  task_id: string
  display_name: string
  content_type: string
  sha256: string
  size_bytes: integer
  redaction_state: string
  availability: string
  created_by_run_ref: StateRecordRef | null
  created_by_surface_id: string | null
  created_by_surface_instance_id: string | null
  storage_ref: string | null
```

`ArtifactRef`는 참조와 메타데이터 형태입니다. 이 값만으로 아티팩트 본문을 읽을 수 있는 것도 아니고, 그 본문이 닫기에 충분한 증거라는 뜻도 아닙니다.

## `StagedArtifactHandle`

`StagedArtifactHandle`은 성공한 `harness.stage_artifact`가 반환하는 임시 핸들입니다. 지속 아티팩트가 아니라 저장소가 소유하는 임시 스테이징을 나타냅니다.

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

호출자는 `created_by_surface_id`나 `created_by_surface_instance_id`를 권한 주장으로 제출하지 않습니다. 스테이징 핸들의 생명주기, 출처 검증, 만료, 승격은 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다.

## `ArtifactInput`

`ArtifactInput`은 실행 기록이나 증거 출력에 아티팩트를 연결하는 메서드가 사용합니다.

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

형태 규칙:
- `staged_artifact_handle`이 채워지면 호환되는 임시 스테이징 핸들이어야 합니다.
- `existing_artifact_ref`가 채워지면 이미 지속되는 같은 프로젝트 아티팩트 참조여야 합니다.

호출자가 준 경로, 로그, 캡처 주장, 로컬 파일 참조는 아티팩트 권한이 아닙니다.

## 참조 제약

`ArtifactInput[]`은 입력마다 아티팩트 출처 형태 하나를 고릅니다. 공개 API 요청에 두 번째 요청 수준 접근 등급을 더하지 않습니다.

출처 필드 형태가 잘못되면 [API 오류 코드](error-codes.md)와 [API 오류 경로](error-routing.md)가 담당하는 공개 오류 의미에 따라 `ToolRejectedResponse`로 반환합니다. 스테이징된 아티팩트 핸들 검증, 승격, 본문 읽기 자격, 지속 연결은 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다.

## 관련 담당 문서

- [아티팩트 스테이징 메서드](method-stage-artifact.md), [실행 기록 메서드](method-record-run.md), [API 메서드](methods.md): 아티팩트 관련 메서드 동작.
- [아티팩트 저장소](../storage-artifacts.md): 스테이징, 승격, 지속 연결, 본문 읽기 생명주기.
- [API 값 집합](schema-value-sets.md): `ArtifactInput.source_kind`, `redaction_state`, 가용성, 관련 값.
- [API 상태 스키마](schema-state.md): `ArtifactRef`를 언급하는 증거 요약.
- [런타임 경계](../runtime-boundaries.md)와 [보안](../security.md): 로컬 접근과 비주장 경계.
