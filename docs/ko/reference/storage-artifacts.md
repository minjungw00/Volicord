# 아티팩트 저장소

이 문서는 현재 MVP 원천 설계의 아티팩트 저장 생명주기를 담당합니다. 문서 원천 자료일 뿐이며 아티팩트 바이트, 아티팩트 디렉터리, 런타임 저장소, 증거 기록, QA 기록, 수락 기록, 닫기 기록을 만들지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 아티팩트 스테이징 저장 생명주기.
- 저장된 스테이징 기록에 대한 `StagedArtifactHandle` 검증.
- 호환되는 스테이징 핸들에서 지속 `ArtifactRef`로 승격하는 경로.
- 지속 `existing_artifact` 연결 자격.
- 아티팩트 본문 읽기의 저장소 자격, 가용성, 가림 처리, 보존, 무결성 경계.

이 문서는 담당하지 않습니다.

- API 아티팩트 스키마: [API 아티팩트 스키마](api/schema-artifacts.md)
- API 메서드 동작: [MVP API](api/mvp-api.md)
- 일반 기록 배치나 DDL: [저장소 기록](storage-records.md)
- 일반 메서드 저장 효과: [저장 효과](storage-effects.md)
- 로컬 접근 보안 주장: [보안](security.md), [런타임 경계](runtime-boundaries.md)

## 생명주기 경계

아티팩트 저장소는 스테이징, 승격, 지속 연결, 본문 읽기를 구분합니다.

`ArtifactRef`는 등록된 지속 아티팩트를 가리키는 공개 API 포인터이지만, 그 형태는 [API 아티팩트 스키마](api/schema-artifacts.md)가 담당합니다. 저장소는 `artifacts`와 `artifact_links`를 통해 지속 아티팩트 권한을 구현합니다.

`StagedArtifactHandle`은 성공한 `harness.stage_artifact`가 반환하는 임시 핸들이지만, 그 형태만으로 권한이 생기지 않습니다. 호환되는 저장 `artifact_staging` 행 또는 동등한 저장소 소유 스테이징 기록으로 해석되어야 합니다. `existing_artifact`는 기존 지속 아티팩트를 연결하는 경로이지 새 아티팩트 본문을 등록하지 않습니다.

호출자가 임의로 준 파일시스템 경로, 임의 로컬 경로 문자열, 권한 주장으로서의 원시 로그, `captured_artifact` 핸들, 원시 캡처 어댑터 출력, 접점 자체 캡처 주장은 현재 MVP의 등록 권한이 아닙니다.

## 스테이징

임시 스테이징은 아티팩트 권한이 아닙니다. `artifact_staging` 또는 동등한 저장소 소유 스테이징 기록은 적어도 `handle_id`, `project_id`, `task_id`, `created_by_surface_id`, `created_by_surface_instance_id`, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `status`, `expires_at`, 그리고 `consumed_by_run_id`, `promoted_artifact_id`, `consumed_at` 같은 소비 사실을 추적합니다.

`created_by_surface_*` 필드는 성공한 `harness.stage_artifact` 요청의 `VerifiedSurfaceContext`에서 서버가 기록합니다. 호출자가 제출한 권한 주장이 아니며, 제출된 핸들의 형태가 맞다는 이유만으로 믿지 말고 스테이징 행과 비교해야 합니다.

성공한 `harness.stage_artifact`는 `base.effect_kind=staging_created`인 `StageArtifactResult`를 반환합니다. `artifacts/tmp/` 아래 안전한 바이트 또는 안전한 알림을 쓰고 임시 스테이징 행을 만들 수 있습니다. Core 기록, `artifacts` 행, `artifact_links` 행, `evidence_summaries` 행, `task_events` 행, `tool_invocations` 재실행 행, `project_state.state_version` 증가를 만들지 않습니다.

`artifact_staging.status`는 저장소 소유 임시 핸들 생명주기입니다.

| 값 | 저장소 의미 |
|---|---|
| `staged` | 핸들이 만료되지 않았고, 소비되지 않았으며, 호환되는 `harness.record_run`이 소비할 수 있습니다. |
| `consumed` | 호환되는 `harness.record_run`이 핸들을 소비했고 소비한 Run과 승격된 아티팩트 id를 기록했습니다. |
| `expired` | 핸들의 사용 가능 시간이 지났고 소비할 수 없습니다. |
| `discarded` | 지속 등록 전에 임시 스테이징 객체를 버렸습니다. |

소비할 수 있는 값은 `staged`뿐입니다. 종료 값은 `staged`로 돌아갈 수 없습니다.

## 승격

호환되는 `harness.record_run`만 `artifact_staging.status=staged`인, 만료되지 않은 같은 프로젝트 같은 Task 핸들을 소비해 지속 `ArtifactRef`로 승격할 수 있습니다. 현재 확인된 `surface_id`와 `surface_instance_id`는 `created_by_surface_id`, `created_by_surface_instance_id`와 일치해야 합니다. 현재 MVP에는 접점 간 스테이징 핸들 인계가 없고, `StagedArtifactHandle`은 어떤 로컬 호출자나 사용할 수 있는 베어러 토큰이 아닙니다.

소비 트랜잭션은 저장된 `project_id`, `task_id`, `created_by_surface_id`, `created_by_surface_instance_id`, 만료 여부, 소비 상태, `sha256`, `size_bytes`, `redaction_state`를 검증해야 합니다. 검증된 스테이징 핸들만 승격하고, 승격된 핸들을 `consumed`로 표시하며, 소비한 Run과 승격된 아티팩트 id를 설정하고, 지속 `artifacts` 행과 필요한 `artifact_links`를 커밋하며, 메서드 담당 문서가 허용한 경우에만 증거 범위를 갱신해야 합니다.

없거나, 만료되었거나, 일치하지 않거나, 이미 소비되었거나, 버려졌거나, 접점이 다르거나, `created_by_surface_id`가 다르거나, `created_by_surface_instance_id`가 다르거나, `sha256`이 다르거나, `size_bytes`가 다르거나, `redaction_state`가 다르거나, 무결성에 맞지 않거나, Task가 다른 스테이징 핸들은 변경 전에 API 담당 검증 오류 경로로 거절해야 합니다. 이를 증거 충분성, 로컬 접근 불일치, 역량 부족으로 숨기면 안 됩니다.

`harness.record_run`의 저장 효과는 API가 담당하는 검증 순서를 따릅니다. 요청 수준 `VerifiedSurfaceContext.access_class=run_recording`, 프로젝트 전체 `ToolEnvelope.expected_state_version`, 참조된 Task와 Change Unit, 제품 파일 쓰기를 기록할 때 호환되는 Write Authorization, 스테이징 핸들 검증, 스테이징 핸들 필드 확인, 스테이징 승격, 스테이징 소비, 기존 아티팩트 연결 검증, 아티팩트 본문 읽기 없음 순서입니다. 이 순서의 어떤 검증이든 커밋 전에 실패하면 저장소는 `artifact_staging.status`, `consumed_by_run_id`, `promoted_artifact_id`, `artifacts`, `artifact_links`, `evidence_summaries`, `write_authorizations.status`, `task_events`, `tool_invocations`, `project_state.state_version`을 바꾸면 안 됩니다.

## 기존 아티팩트

`existing_artifact`는 해당 지속 아티팩트 행의 가용성, 무결성 사실, 가림 처리 상태, 같은 프로젝트 식별성, 허용된 Task 범위가 새 사용과 계속 호환될 때만 재사용합니다. 고유성 규칙과 같은 프로젝트/같은 Task 규칙에 따라 새 담당 관계를 위한 `artifact_links` 행을 추가할 수 있습니다.

`existing_artifact`는 바이트를 복제하거나, 새 아티팩트 본문을 등록하거나, 무결성 확인을 건너뛰거나, 원시 아티팩트 경로를 권한으로 사용하면 안 됩니다.

## 증거 자격

아티팩트가 증거로 쓰일 수 있으려면 저장소에 아래 항목이 있어야 합니다.

- 아티팩트 저장소 아래 등록된 바이트 또는 안전한 메타데이터 알림
- `sha256`, `size_bytes`, `content_type` 같은 무결성 사실
- `redaction_state`
- 생산자와 보존 사실
- 가용성 `status`
- `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` 같은 활성 기록에 대한 담당 연결

증거 자격, 아티팩트 가용성, 증거 충분성은 서로 다릅니다. 유효한 담당 연결이 있는 `artifacts.status=available` 행은 범위 항목을 뒷받침할 수 있지만, 필수 범위 항목이 그 아티팩트를 주장에 연결하고 항목 상태가 `supported` 또는 `not_applicable`일 때만 `EvidenceSummary.status=sufficient`가 될 수 있습니다. 없거나, 사용할 수 없거나, 무결성 실패이거나, 그 밖에 쓸 수 없는 아티팩트는 아티팩트 가용성 문제로 남으며 필수 증거 범위를 충분하지 않게 만들 수도 있습니다.

`artifact_links`가 다형 담당 테이블이어도 아티팩트 담당 관계 무결성은 필요합니다. 저장소는 `owner_record_kind`가 `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` 중 하나인지, `owner_record_id`가 맞는 활성 테이블에 존재하는지, 담당 행이 같은 `project_id`와 `task_id`에 속하는지, 관계가 아티팩트 사용 방식과 호환되는지 검증해야 합니다. 유효한 담당 연결이 없는 원시 `artifact_id`는 증거 지원이 아닙니다.

아티팩트 연결은 담당 기록을 만들거나, 그 자체로 gate를 충족하거나, 증거 충분성을 증명하거나, QA를 수행하거나, 최종 수락을 만들거나, 잔여 위험을 수락하거나, Task를 닫지 않습니다.

## 가용성, 가림 처리, 무결성

`artifacts.status`는 가용성 상태입니다.

| 값 | 저장소 의미 |
|---|---|
| `available` | 등록된 안전 바이트 또는 안전한 메타데이터 알림이 존재하며 저장된 무결성 메타데이터와 맞습니다. |
| `missing` | 아티팩트 행은 남아 있지만 등록된 바이트 또는 안전한 메타데이터 알림을 찾을 수 없습니다. |
| `integrity_failed` | 사용할 수 있는 바이트 또는 메타데이터가 `sha256`이나 `size_bytes` 같은 저장된 무결성 사실과 맞지 않습니다. |
| `unavailable` | 아티팩트 저장소 또는 필요한 조회 경로가 현재 등록된 바이트 또는 안전한 메타데이터 알림을 제공할 수 없습니다. |

`artifacts.redaction_state`는 [API 아티팩트 스키마](api/schema-artifacts.md)의 활성 `ArtifactRef.redaction_state` 값을 사용합니다. `blocked`는 가림/생략 상태이지 아티팩트 가용성 상태가 아닙니다. 커밋된 안전 알림이나 가림 처리된 바이트가 존재하고 무결성 확인이 가능하면 `blocked`, `secret_omitted`, `redacted` 아티팩트도 `artifacts.status=available`일 수 있습니다.

`sha256`, `size_bytes`, `content_type`은 비교와 가용성 처리를 위한 아티팩트 무결성 사실입니다. 보안 보장 주장이 아닙니다. [보안](security.md)을 확인하세요.

`uri`는 보통 `harness-artifact://{project_id}/{artifact_id}`처럼 하네스 저장소를 통해 해석됩니다. 호출자가 제공한 임의 파일시스템 경로가 아닙니다. 원시 비밀값, 토큰, 민감한 전체 로그는 증거 바이트로 저장하면 안 됩니다. 대신 가림 처리된 바이트, `secret_omitted` 또는 `blocked` 알림, 안전 핸들, 담당 문서가 승인한 다른 안전 표현을 저장합니다.

## 본문 읽기

아티팩트 본문 읽기는 스테이징 핸들 승격과 별개입니다. 원시 아티팩트 경로 읽기는 기본으로 부여되지 않습니다.

아티팩트 메타데이터 또는 본문을 읽으려면 등록된 `ArtifactRef`, 일치하는 같은 프로젝트 `task_id`, 필요한 `artifact_links` 담당 관계, 호출자 접근 등급에 필요한 가림 처리/가용성 상태, `access_class=artifact_read`에 대한 API/보안 담당 문서 요구사항이 필요합니다. 아티팩트 저장소 아래 로컬 경로, 아티팩트 `uri`, 스테이징 경로, 복사된 파일만으로는 아티팩트 바이트를 읽거나 신뢰하기에 충분하지 않습니다.

## 보존 경계

소비되지 않았거나 만료된 `artifact_staging` 행과 `artifacts/tmp/` 스테이징 바이트 또는 알림은 `expired` 또는 `discarded`로 표시할 수 있고, 등록 전 임시 바이트는 정리할 수 있습니다. 이것들은 증거 권한이 아니기 때문입니다.

`artifacts` 행이 커밋된 뒤의 보존 삭제, 프로젝트 해체, 파괴적 정리는 일반적인 현재 MVP 변경 동작 밖이며 담당 문서가 정의한 경로가 필요합니다. 향후 보존 또는 마이그레이션 경로는 아티팩트 해시, 담당 연결, 이벤트, 재실행 행을 보존하거나 영향을 받은 참조를 복구 대상으로 유효하지 않게 표시해야 합니다. 현재 기록이 아직 이름 붙인 증거 지원을 조용히 삭제하면 안 됩니다.

## 관련 담당 문서

- [API 아티팩트 스키마](api/schema-artifacts.md): `ArtifactRef`, `ArtifactInput`, `StagedArtifactHandle` 형태.
- [MVP API](api/mvp-api.md): `harness.stage_artifact`, `harness.record_run`, 아티팩트 읽기 동작.
- [저장 효과](storage-effects.md): 응답 분기가 저장 효과를 만드는지 여부.
- [저장소 기록](storage-records.md): `artifact_staging`, `artifacts`, `artifact_links` 테이블 개요.
- [보안](security.md): 접근과 보장 비주장.
