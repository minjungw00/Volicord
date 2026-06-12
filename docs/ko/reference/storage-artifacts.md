# 아티팩트 저장 참조

규칙:

- 이 문서는 현재 MVP 원천 설계에서 아티팩트 저장 생명주기를 담당합니다.

허용되지 않는 것:

- 이 문서는 아티팩트 바이트, 아티팩트 디렉터리, 런타임 저장소, 증거 기록, QA 기록, 수락 기록, 닫기 기록을 만들지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 아티팩트 스테이징의 저장 생명주기.
- 저장소가 보관한 스테이징 기록에 대한 `StagedArtifactHandle` 검증.
- 호환되는 스테이징 핸들을 지속 `ArtifactRef`로 승격하는 저장소 경계.
- 기존 지속 아티팩트를 새 담당 관계에 연결할 수 있는 조건.
- 아티팩트 본문 읽기의 저장소 자격, 가용성, 가림 처리, 보존, 무결성 경계.

이 문서는 담당하지 않습니다.

| 주제 | 담당 문서 |
|---|---|
| `ArtifactRef`, `ArtifactInput`, `StagedArtifactHandle` 형태 | [API 아티팩트 스키마](api/schema-artifacts.md) |
| 아티팩트 관련 API 동작 | [아티팩트 스테이징 메서드](api/method-stage-artifact.md), [실행 기록 메서드](api/method-record-run.md), [MVP API 경로 문서](api/mvp-api.md) |
| 일반 기록 배치와 DDL | [저장소 기록](storage-records.md) |
| 메서드별 저장 효과와 상태 버전 영향 | [저장 효과](storage-effects.md), [저장소 버전 관리](storage-versioning.md) |
| 로컬 접근, 접근 등급, 보안 보장 수준 | [보안](security.md), [런타임 경계](runtime-boundaries.md) |

## 아티팩트 생명주기 요약

아티팩트 저장은 네 단계를 구분합니다. 아래 요약 표는 단계만 보여 주고, 세부 블록은 의미와 증거와의 관계를 분리합니다.

| 단계 | 세부 블록 |
|---|---|
| 스테이징 | [생명주기: 스테이징](#artifact-lifecycle-staging) |
| 승격 | [생명주기: 승격](#artifact-lifecycle-promotion) |
| 기존 아티팩트 연결 | [생명주기: 기존 아티팩트 연결](#artifact-lifecycle-existing-artifact-link) |
| 아티팩트 본문 읽기 | [생명주기: 아티팩트 본문 읽기](#artifact-lifecycle-body-read) |

<a id="artifact-lifecycle-staging"></a>
**생명주기: 스테이징**

의미:

- `harness.stage_artifact`가 임시 아티팩트 바이트나 안전한 알림을 저장하고 스테이징 핸들을 반환하는 단계입니다.

증거와의 관계:

- 스테이징 자체는 정식 증거를 만들지 않습니다.

<a id="artifact-lifecycle-promotion"></a>
**생명주기: 승격**

의미:

- 담당 메서드가 호환되는 스테이징 핸들을 받아 지속 `ArtifactRef`와 필요한 `artifact_links`로 등록하는 단계입니다.

증거와의 관계:

- 담당 메서드 계약이 허용할 때만 증거 범위가 갱신될 수 있습니다.

<a id="artifact-lifecycle-existing-artifact-link"></a>
**생명주기: 기존 아티팩트 연결**

의미:

- 이미 지속되는 아티팩트를 새 담당 관계에 연결하는 단계입니다.

증거와의 관계:

- 담당 메서드가 증거로 기록하지 않으면 새 증거를 뜻하지 않습니다.

<a id="artifact-lifecycle-body-read"></a>
**생명주기: 아티팩트 본문 읽기**

의미:

- 등록된 `ArtifactRef`의 메타데이터나 아티팩트 바이트를 읽는 단계입니다.

조건:

- 접근 등급, 역량, 가림 처리, 가용성, 담당 관계를 통과해야 합니다.

규칙:

- `ArtifactRef`는 등록된 지속 아티팩트를 가리키는 공개 API 포인터입니다.
- 저장소는 `artifacts`와 `artifact_links`를 통해 지속 아티팩트 권한을 표현합니다.

허용되는 것:

- `StagedArtifactHandle`은 임시 입력을 찾기 위한 값입니다.

허용되지 않는 것:

- `StagedArtifactHandle`은 `ArtifactRef`가 아닙니다.
- 스테이징 핸들은 지속 아티팩트 포인터나 증거 권한이 아닙니다.
- 호출자가 준 원시 파일시스템 경로, 임의 로컬 경로 문자열, 원시 로그, `captured_artifact` 핸들, 원시 캡처 어댑터 출력, 접점 자체 캡처 주장은 현재 MVP의 아티팩트 등록 권한이 아닙니다.

허용되는 것:

- 소비되지 않았거나 만료된 `artifact_staging` 행과 `artifacts/tmp/` 아래 임시 바이트 또는 알림은 `expired` 또는 `discarded`로 표시할 수 있습니다.
- 등록 전 임시 바이트는 정리할 수 있습니다.

예외:

- 이 임시 자료는 증거 권한이 아니므로 위 정리가 허용됩니다.

허용되지 않는 것:

- `artifacts` 행이 커밋된 뒤의 보존 삭제, 프로젝트 해체, 파괴적 정리는 일반적인 현재 MVP 변경 동작 밖입니다.
- 향후 보존 또는 마이그레이션 경로는 현재 기록이 아직 이름 붙인 증거 지원을 조용히 삭제하면 안 됩니다.

규칙:

- 향후 보존 또는 마이그레이션 경로는 아티팩트 해시, 담당 연결, 이벤트, 재실행 행을 보존하거나 영향을 받은 참조를 복구 대상으로 유효하지 않게 표시해야 합니다.

## 스테이징

규칙:

- `harness.stage_artifact`는 아티팩트를 스테이징합니다.
- 이 메서드는 데이터를 임시 저장할 뿐, 정식 증거를 만들지 않습니다.

허용되는 것:

- 성공한 `harness.stage_artifact`는 `base.effect_kind=staging_created`인 `StageArtifactResult`를 반환합니다.
- 저장소는 `artifacts/tmp/` 아래 안전한 아티팩트 바이트 또는 안전한 알림을 둘 수 있습니다.
- 저장소는 `artifact_staging` 또는 동등한 저장소 소유 스테이징 기록을 만들 수 있습니다.

스테이징할 아티팩트 데이터 예시는 아래와 같습니다.

```yaml
artifact:
  kind: test_log
  name: account_export_confirmation_test.log
  description: "계정 내보내기 확인 동작 테스트 출력."
staged_artifact_handle: staged_artifact_account_export_test_log_001
expires_at: "<future-expiration-timestamp>"
```

규칙:

- 이 예시는 제품 테스트 출력을 스테이징하는 입력을 나타냅니다.

허용되지 않는 것:

- 이 예시는 지속 `ArtifactRef`가 아닙니다.
- 호환되는 담당 메서드가 계약에 따라 기록하고 승격하기 전에는 정식 증거가 아닙니다.

스테이징 기록은 적어도 아래 사실을 추적합니다.

| 분류 | 저장되는 사실 |
|---|---|
| 식별 | `handle_id`, `project_id`, `task_id` |
| 출처 | `created_by_surface_id`, `created_by_surface_instance_id` |
| 무결성 | `sha256`, `size_bytes`, `content_type` |
| 가림 처리 | `redaction_state` |
| 생명주기 | `status`, `expires_at` |
| 소비 결과 | `consumed_by_run_id`, `promoted_artifact_id`, `consumed_at` 같은 소비 사실 |

규칙:

- `created_by_surface_*` 필드는 성공한 `harness.stage_artifact` 요청의 `VerifiedSurfaceContext`에서 향후 서버가 기록하는 값입니다.
- 저장소에 있는 스테이징 기록과 대조해야 합니다.

허용되지 않는 것:

- 이 필드는 호출자가 제출한 권한 주장이 아닙니다.
- 제출된 핸들의 형태가 맞다는 이유만으로 신뢰하면 안 됩니다.

스테이징이 만들지 않는 것은 아래와 같습니다.

- 지속 `ArtifactRef`.
- 정식 증거.
- 증거 충분성.
- QA 결과.
- 최종 수락.
- 잔여 위험 수락.
- Task 닫기.

증거 생성, 재실행 행, 상태 버전 증가 같은 메서드 저장 효과는 [저장 효과](storage-effects.md)가 담당합니다.

## 스테이징 핸들

`StagedArtifactHandle`은 성공한 `harness.stage_artifact`가 반환하는 임시 스테이징 핸들입니다. 이 값은 저장소가 보관한 호환 스테이징 기록으로 해석될 때만 소비 후보가 됩니다.

스테이징 핸들은 아래와 같이 다룹니다.

| 구분 | 규칙 |
|---|---|
| `StagedArtifactHandle` | 임시 스테이징 입력을 가리킵니다. |
| `ArtifactRef` | 등록된 지속 아티팩트를 가리킵니다. |
| 관계 | 스테이징 핸들은 승격 전까지 `ArtifactRef`가 아닙니다. |
| 권한 | 스테이징 핸들은 아무 로컬 호출자나 사용할 수 있는 베어러 토큰이 아닙니다. |

`artifact_staging.status`는 저장소 소유의 임시 핸들 생명주기입니다. 요약 표는 값만 짧게 보여 주고, 세부 블록은 조건과 허용되는 효과를 나눕니다.

| 값 | 요약 | 세부사항 |
|---|---|---|
| `staged` | 소비 후보 | [`staged`](#artifact-staging-status-staged) |
| `consumed` | 담당 메서드가 소비함 | [`consumed`](#artifact-staging-status-consumed) |
| `expired` | 사용 가능 시간 지남 | [`expired`](#artifact-staging-status-expired) |
| `discarded` | 임시 객체 버림 | [`discarded`](#artifact-staging-status-discarded) |

<a id="artifact-staging-status-staged"></a>
**`artifact_staging.status=staged`**

조건:

- 핸들이 만료되지 않았습니다.
- 핸들이 아직 소비되지 않았습니다.

허용되는 효과:

- 호환되는 담당 메서드가 핸들을 소비할 수 있습니다.

<a id="artifact-staging-status-consumed"></a>
**`artifact_staging.status=consumed`**

조건:

- 호환되는 담당 메서드가 핸들을 소비했습니다.

저장 효과:

- 소비한 Run과 승격된 아티팩트 식별자를 기록합니다.

<a id="artifact-staging-status-expired"></a>
**`artifact_staging.status=expired`**

조건:

- 핸들의 사용 가능 시간이 지났습니다.

허용되지 않는 효과:

- 핸들을 소비할 수 없습니다.

<a id="artifact-staging-status-discarded"></a>
**`artifact_staging.status=discarded`**

조건:

- 지속 등록 전에 임시 스테이징 객체를 버렸습니다.

허용되지 않는 효과:

- 핸들을 소비할 수 없습니다.

소비할 수 있는 값은 `staged`뿐입니다. `consumed`, `expired`, `discarded`는 `staged`로 돌아갈 수 없습니다.

## 기존 아티팩트 참조

규칙:

- `existing_artifact`는 이미 지속되는 아티팩트 행을 재사용하는 입력입니다.

허용되지 않는 것:

- `existing_artifact`는 새 아티팩트 바이트를 등록하거나 새 본문을 복제하는 경로가 아닙니다.

기존 아티팩트 참조는 아래 조건이 새 사용과 계속 호환될 때만 연결될 수 있습니다.

- 같은 프로젝트.
- 허용된 Task 범위.
- 가용성 상태.
- 무결성 사실.
- `redaction_state`.
- 필요한 담당 관계.

호환되는 경우 새 담당 관계를 위해 `artifact_links` 행을 추가할 수 있습니다. 이 연결은 고유성 규칙, 같은 프로젝트 규칙, 같은 Task 규칙을 따라야 합니다.

기존 아티팩트 참조가 해서는 안 되는 일은 아래와 같습니다.

- 아티팩트 바이트 복제.
- 새 아티팩트 본문 등록.
- 체크섬과 크기 검증 생략.
- 원시 아티팩트 경로를 권한으로 사용.
- 담당 메서드가 증거로 기록하지 않았는데 새 증거가 생긴 것처럼 암시.

## 승격

규칙:

- 승격은 스테이징 핸들을 지속 `ArtifactRef`로 바꾸는 저장소 단계입니다.
- 승격에는 스테이징 핸들을 받아들이는 담당 메서드가 필요합니다.

현재 MVP에서 대표적인 담당 메서드는 `harness.record_run`입니다. 호환되는 `harness.record_run`만 아래 조건을 모두 만족하는 스테이징 핸들을 소비할 수 있습니다.

- `artifact_staging.status=staged`.
- 만료되지 않음.
- 같은 `project_id`.
- 같은 `task_id`.
- 현재 확인된 `surface_id`가 `created_by_surface_id`와 일치함.
- 현재 확인된 `surface_instance_id`가 `created_by_surface_instance_id`와 일치함.
- `sha256`, `size_bytes`, `redaction_state`가 저장된 스테이징 기록과 일치함.

허용되지 않는 것:

- 현재 MVP는 접점 간 스테이징 핸들 인계를 지원하지 않습니다.
- 스테이징을 만든 접점과 승격하려는 접점이 다르면 승격은 거절되어야 합니다.

소비 트랜잭션은 아래 일을 커밋해야 합니다.

- 검증된 스테이징 핸들만 승격합니다.
- 승격된 핸들을 `consumed`로 표시합니다.
- 소비한 Run과 승격된 아티팩트 식별자를 기록합니다.
- 지속 `artifacts` 행을 커밋합니다.
- 필요한 `artifact_links` 행을 커밋합니다.
- 메서드 담당 문서가 허용한 경우에만 증거 범위를 갱신합니다.

허용되는 것:

- 승격은 스테이징 핸들을 지속 아티팩트로 등록할 수 있습니다.
- 증거 범위 갱신은 `harness.record_run` 같은 담당 메서드의 계약 안에서만 일어납니다.

허용되지 않는 것:

- 승격 자체가 모든 증거 관문을 충족하지는 않습니다.

## 증거와의 관계

스테이징, 아티팩트 가용성, 증거 자격, 증거 충분성은 서로 다릅니다.

| 개념 | 뜻 |
|---|---|
| 스테이징 | 임시 아티팩트 바이트나 안전한 알림을 보관합니다. 정식 증거가 아닙니다. |
| 아티팩트 가용성 | 등록된 지속 아티팩트가 읽을 수 있는 상태인지 나타냅니다. |
| 증거 자격 | 저장소 사실과 담당 연결이 있어 증거 범위 항목을 뒷받침할 수 있는 상태입니다. |
| 증거 충분성 | 필요한 증거 범위가 실제 주장에 연결되어 충분하다고 평가된 상태입니다. |

아티팩트가 증거로 쓰일 수 있으려면 저장소에 아래 항목이 있어야 합니다.

- 아티팩트 저장소 아래 등록된 아티팩트 바이트 또는 안전한 메타데이터 알림.
- `sha256`, `size_bytes`, `content_type` 같은 무결성 사실.
- `redaction_state`.
- 생산자와 보존 사실.
- 가용성 `status`.
- `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` 같은 활성 기록에 대한 담당 연결.

허용되는 것:

- 유효한 담당 연결이 있는 `artifacts.status=available` 행은 증거 범위 항목을 뒷받침할 수 있습니다.
- 필수 범위 항목이 그 아티팩트를 주장에 연결하고 항목 상태가 `supported` 또는 `not_applicable`일 때만 `EvidenceSummary.status=sufficient`가 될 수 있습니다.

허용되지 않는 것:

- 없거나, 사용할 수 없거나, 무결성 실패이거나, 그 밖에 쓸 수 없는 아티팩트는 아티팩트 가용성 문제로 남습니다.
- 유효한 담당 연결이 없는 원시 `artifact_id`는 증거 지원이 아닙니다.

규칙:

- `artifact_links`가 다형 담당 테이블이어도 담당 관계 무결성은 필요합니다.
- 이런 가용성 문제는 필수 증거 범위를 충분하지 않게 만들 수도 있습니다.

필수 검증:

- `owner_record_kind`가 `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` 중 하나인지 확인합니다.
- `owner_record_id`가 해당 활성 테이블에 존재하는지 확인합니다.
- 담당 기록이 같은 `project_id`에 속하는지 확인합니다.
- 담당 기록이 같은 `task_id`에 속하는지 확인합니다.
- 관계가 아티팩트 사용 방식과 호환되는지 확인합니다.

아티팩트 연결은 아래 항목을 만들거나 증명하지 않습니다.

- 담당 기록 생성.
- 관문 충족.
- 증거 충분성 증명.
- QA 수행.
- 최종 수락 생성.
- 잔여 위험 수락.
- Task 닫기.

기존 아티팩트 참조도 마찬가지입니다. `existing_artifact`가 새 `artifact_links` 행을 추가할 수는 있지만, 담당 메서드가 그 연결을 증거로 기록하지 않으면 새 증거가 생겼다고 볼 수 없습니다.

## 아티팩트 본문 읽기

규칙:

- 아티팩트 본문 읽기는 스테이징 핸들 승격과 별개입니다.

허용되지 않는 것:

- 원시 아티팩트 경로 읽기는 기본으로 부여되지 않습니다.

아티팩트 메타데이터나 아티팩트 바이트를 읽으려면 아래 조건이 필요합니다.

| 조건 | 의미 |
|---|---|
| 등록된 참조 | 읽기 대상은 등록된 `ArtifactRef`여야 합니다. |
| 같은 범위 | 요청의 `project_id`와 `task_id`가 아티팩트 범위와 맞아야 합니다. |
| 담당 관계 | 필요한 `artifact_links` 담당 관계가 있어야 합니다. |
| 가용성과 가림 처리 | 호출자의 접근 등급에 맞는 `artifacts.status`와 `redaction_state`여야 합니다. |
| 접근 등급 | `access_class=artifact_read`에 대한 API/보안 담당 문서 요구사항을 통과해야 합니다. |
| 역량 경계 | 접점이나 커넥터가 노출한 아티팩트 읽기 역량 경계를 넘어 읽을 수 없습니다. |

아래 값만으로는 아티팩트 바이트를 읽거나 신뢰하기에 충분하지 않습니다.

- 아티팩트 저장소 아래 로컬 경로.
- 아티팩트 `uri`.
- 스테이징 경로.
- 복사된 파일.
- 스테이징 핸들.
- 원시 `artifact_id`.

규칙:

- `uri`는 보통 `harness-artifact://{project_id}/{artifact_id}`처럼 하네스 저장소를 통해 해석됩니다.

허용되는 것:

- 가림 처리된 아티팩트 바이트, `secret_omitted` 또는 `blocked` 알림, 안전 핸들, 담당 문서가 승인한 다른 안전 표현을 저장합니다.

허용되지 않는 것:

- `uri`는 호출자가 제공한 임의 파일시스템 경로가 아닙니다.
- 원시 비밀값, 토큰, 민감한 전체 로그는 증거로 쓰일 아티팩트 바이트로 저장하면 안 됩니다.

## 검증과 실패

`harness.record_run`에서 아티팩트 입력을 처리할 때 저장 효과는 API가 담당하는 아래 순서를 따릅니다.

1. 요청 수준 `VerifiedSurfaceContext.access_class=run_recording`.
2. 프로젝트 전체 `ToolEnvelope.expected_state_version`.
3. 참조된 Task와 Change Unit.
4. 제품 파일 쓰기를 기록할 때 호환되는 `Write Authorization`.
5. 스테이징 핸들 검증.
6. 스테이징 핸들 필드 확인.
7. 스테이징 승격.
8. 스테이징 소비.
9. 기존 아티팩트 연결 검증.
10. 아티팩트 본문 읽기 없음.

아래 스테이징 핸들은 변경 전에 API 담당 검증 오류 경로로 거절해야 합니다. 요약 표는 실패 유형만 보여 주고, 세부 블록은 예를 분리합니다.

| 실패 유형 | 세부사항 |
|---|---|
| 존재 또는 생명주기 문제 | [존재 또는 생명주기 문제](#staged-handle-failure-existence-lifecycle) |
| 범위 불일치 | [범위 불일치](#staged-handle-failure-scope) |
| 접점 불일치 | [접점 불일치](#staged-handle-failure-surface) |
| 무결성 불일치 | [무결성 불일치](#staged-handle-failure-integrity) |

<a id="staged-handle-failure-existence-lifecycle"></a>
**존재 또는 생명주기 문제**

예:

- 존재하지 않음.
- 만료됨.
- 이미 소비됨.
- 버려짐.

<a id="staged-handle-failure-scope"></a>
**범위 불일치**

예:

- 일치하지 않음.
- Task가 다름.
- 프로젝트가 다름.

<a id="staged-handle-failure-surface"></a>
**접점 불일치**

예:

- 접점이 다름.
- `created_by_surface_id` 불일치.
- `created_by_surface_instance_id` 불일치.

<a id="staged-handle-failure-integrity"></a>
**무결성 불일치**

예:

- `sha256` 불일치.
- `size_bytes` 불일치.
- `redaction_state` 불일치.
- 무결성에 맞지 않음.

이 오류를 증거 충분성, 로컬 접근 불일치, 역량 부족으로 숨기면 안 됩니다.

이 순서의 어떤 검증이든 커밋 전에 실패하면 저장소는 아래 항목을 바꾸면 안 됩니다.

- `artifact_staging.status`.
- `consumed_by_run_id`.
- `promoted_artifact_id`.
- `artifacts`.
- `artifact_links`.
- `evidence_summaries`.
- `write_authorizations.status`.
- `task_events`.
- `tool_invocations`.
- `project_state.state_version`.

`artifacts.status`는 가용성 상태입니다. 요약 표는 값만 짧게 보여 주고, 세부 블록은 저장소 의미를 나눕니다.

| 값 | 요약 | 세부사항 |
|---|---|---|
| `available` | 존재하고 무결성 일치 | [`available`](#artifacts-status-available) |
| `missing` | 행은 남았지만 본문 없음 | [`missing`](#artifacts-status-missing) |
| `integrity_failed` | 무결성 사실 불일치 | [`integrity_failed`](#artifacts-status-integrity_failed) |
| `unavailable` | 조회 경로 사용 불가 | [`unavailable`](#artifacts-status-unavailable) |

<a id="artifacts-status-available"></a>
**`artifacts.status=available`**

저장소 의미:

- 등록된 안전 바이트 또는 안전한 메타데이터 알림이 존재합니다.
- 저장된 무결성 메타데이터와 맞습니다.

<a id="artifacts-status-missing"></a>
**`artifacts.status=missing`**

저장소 의미:

- 아티팩트 행은 남아 있습니다.
- 등록된 바이트 또는 안전한 메타데이터 알림을 찾을 수 없습니다.

<a id="artifacts-status-integrity_failed"></a>
**`artifacts.status=integrity_failed`**

저장소 의미:

- 사용할 수 있는 바이트 또는 메타데이터가 `sha256`이나 `size_bytes` 같은 저장된 무결성 사실과 맞지 않습니다.

<a id="artifacts-status-unavailable"></a>
**`artifacts.status=unavailable`**

저장소 의미:

- 아티팩트 저장소 또는 필요한 조회 경로가 현재 등록된 바이트 또는 안전한 메타데이터 알림을 제공할 수 없습니다.

규칙:

- `artifacts.redaction_state`는 [API 아티팩트 스키마](api/schema-artifacts.md)의 활성 `ArtifactRef.redaction_state` 값을 사용합니다.
- 체크섬과 크기 검증은 아티팩트 바이트와 저장된 메타데이터가 맞는지 확인합니다.
- `sha256`, `size_bytes`, `content_type`은 저장된 바이트 비교와 가용성 처리를 위한 무결성 사실입니다.

허용되는 것:

- 커밋된 안전 알림이나 가림 처리된 아티팩트 바이트가 존재하고 무결성 확인이 가능하면 `blocked`, `secret_omitted`, `redacted` 아티팩트도 `artifacts.status=available`일 수 있습니다.

허용되지 않는 것:

- `blocked`는 가림 또는 생략 상태이지 아티팩트 가용성 상태가 아닙니다.
- 체크섬과 크기 검증은 아티팩트의 의미 내용이 맞는지, 로그가 주장을 실제로 뒷받침하는지, 테스트가 성공했는지, 증거가 충분한지를 증명하지 않습니다.
- `sha256`, `size_bytes`, `content_type`은 보안 보장 주장이 아닙니다.

담당 문서 링크:

- 보안 보장과 로컬 접근 비주장은 [보안](security.md)이 담당합니다.

## 관련 담당 문서

- [API 아티팩트 스키마](api/schema-artifacts.md): `ArtifactRef`, `ArtifactInput`, `StagedArtifactHandle` 형태.
- [아티팩트 스테이징 메서드](api/method-stage-artifact.md), [실행 기록 메서드](api/method-record-run.md), [MVP API 경로 문서](api/mvp-api.md): `harness.stage_artifact`, `harness.record_run`, 아티팩트 읽기 API 동작.
- [저장 효과](storage-effects.md): 응답 분기가 저장 효과를 만드는지 여부와 상태 버전 영향.
- [저장소 기록](storage-records.md): `artifact_staging`, `artifacts`, `artifact_links` 테이블 개요.
- [저장소 버전 관리](storage-versioning.md): 저장소 상태 버전, 잠금, 버전 관리 경계.
- [보안](security.md): 접근 등급, 역량 경계, 보장 비주장.
