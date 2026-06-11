# 용어집 참조

이 문서는 하네스 문서의 공식 용어를 담당합니다. 제품 용어의 산문 의미, 한국어 기준 표현, 사용자용 표현, 식별자 보존, 피할 표현, 담당 문서 경로를 함께 정리합니다. 정확한 스키마, 값 집합, DDL, 저장 효과, 보안 메커니즘, API 동작, 구현 순서는 이 문서가 정의하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 제품, Core, API, 저장소, 보안, 에이전트, 상태 보기, 이후 후보 개념의 공식 영어/한국어 용어
- 한국어 기준 용어와 사용자용 대체 표현
- 문서 산문에서 쓰는 용어 수준 의미
- 한국어 하네스 문서에서 피할 표현
- 용어에서 기준 기술 담당 문서로 가는 링크

이 문서는 담당하지 않습니다.

- 정확한 API 필드 형태나 enum 형태 값: API 스키마 담당 문서와 [API 값 집합](api/schema-value-sets.md)
- 공개 오류 코드: [API 오류](api/errors.md)
- 저장소 기록, 효과, 아티팩트, 버전 관리, 잠금, 마이그레이션: [참조 색인](README.md)의 저장소 담당 문서
- 템플릿 본문: [템플릿 본문](template-bodies.md)
- 구현 준비: [MVP 계획](../build/mvp-plan.md)

## 용어 통제 표

이 표는 [docs/terminology-map.yaml](../../terminology-map.yaml)과 함께 사용합니다. 용어집은 독자용 의미와 담당 문서 경로를 설명하고, 용어 지도는 기계 판독 가능한 통제 파일입니다. 정확한 식별자는 영어와 한국어 모두에서 backtick으로 보존합니다.

| 영어 용어 | 의미 | 한국어 기준 용어 | 한국어 사용자 표현 | 식별자로 보존 | 피할 표현 | 담당 문서 |
|---|---|---|---|---|---|---|
| Harness | AI 지원 제품 작업을 위한 향후 로컬 작업 권한 서버입니다. | 하네스 | 하네스 | - | 이 문서 저장소를 작동 중인 서버로 보는 표현 | [현재 MVP 범위](active-mvp-scope.md), [런타임 경계](runtime-boundaries.md) |
| Product Repository | 사용자의 프로젝트 작업 공간입니다. 제품 파일은 하네스 런타임 상태가 아닙니다. | Product Repository | 제품 저장소 | 경계를 이름 붙일 때 `Product Repository` | 제품 파일을 하네스 기록으로 보는 표현 | [런타임 경계](runtime-boundaries.md) |
| Harness Runtime Home | 향후 하네스 기록과 아티팩트를 담는 운영 데이터 공간입니다. 이 문서 저장소는 Runtime Home이 아닙니다. | Harness Runtime Home | 런타임 홈 | 경계를 이름 붙일 때 `Harness Runtime Home` | 이 저장소나 Product Repository를 Runtime Home으로 보는 표현 | [런타임 경계](runtime-boundaries.md), [참조 색인](README.md)의 저장소 담당 문서 |
| documentation-only | 현재 저장소와 편집 범위가 문서 전용이라는 뜻입니다. 런타임 구현이나 생성된 런타임 기록을 승인하지 않습니다. | 문서 전용 | 문서 전용 | 파일 경로와 담당 문서 라벨 | 구현 완료, 런타임 준비 완료, 생성된 운영 기록 | [작성 가이드](../maintain/authoring-guide.md), [런타임 경계](runtime-boundaries.md), [MVP 계획](../build/mvp-plan.md) |
| active MVP | 첫 로컬 작업 루프를 위한 활성 제품 범위 경계입니다. | 현재 MVP | 현재 MVP | 담당 문서 제목과 정확한 값 | 이후 후보나 profile-gated 값을 현재 요구사항처럼 쓰는 표현 | [현재 MVP 범위](active-mvp-scope.md), [API 값 집합](api/schema-value-sets.md) |
| reserved value | 향후 또는 profile-gated 사용을 위해 남겨 둔 값 이름입니다. 이름이 있다는 사실만으로 동작이 활성화되지는 않습니다. | 예약된 값 | 예약된 값 | 정확한 값 문자열 | 기본값, 필수값, 지원됨, 강제됨, 수락됨, 검증됨, 닫기 준비 상태, 현재 활성 보장처럼 쓰는 표현 | [현재 MVP 범위](active-mvp-scope.md), [API 값 집합](api/schema-value-sets.md) |
| profile-gated value | 관련 프로필이 지원할 때만 쓸 수 있는 값입니다. 값이 존재한다고 해서 어디서나 사용할 수 있다는 뜻은 아닙니다. | profile-gated 값 | profile-gated 값 | 정확한 값 문자열 | 값 집합에 있다는 이유만으로 profile-gated 값을 현재 MVP 동작처럼 쓰는 표현 | [현재 MVP 범위](active-mvp-scope.md), [API 값 집합](api/schema-value-sets.md) |
| later candidate | 아직 활성화되지 않은 향후 아이디어입니다. 관련 담당 문서가 승격하기 전까지 현재 요구사항을 만들지 않습니다. | 이후 후보 | 이후 후보 | - | 미뤄 둔 자료를 현재 MVP 요구사항처럼 부르는 표현 | [이후 후보 색인](../later/index.md), [현재 MVP 범위](active-mvp-scope.md) |
| owner document | 제품 개념, 계약, 스키마 묶음, 경로, 용어 규칙의 기준 의미를 정의하는 담당 문서입니다. | 담당 문서 | 담당 문서 | 파일 경로와 앵커 | 두 번째 기준 문서, 복사된 계약 담당 문서 | [작성 가이드](../maintain/authoring-guide.md), [참조 색인](README.md) |
| current owner | 현재 존재하며 규범 의미의 기준으로 연결할 수 있는 담당 문서입니다. | 현재 담당 문서 | 현재 담당 문서 | 파일 경로와 앵커 | 향후 담당 문서 자리표시자를 현재 담당 문서처럼 이름 붙이는 표현 | [작성 가이드](../maintain/authoring-guide.md), [참조 색인](README.md), [doc-index.yaml](../../doc-index.yaml) |
| promotion-time owner update | 이후 후보가 승격될 때에만 기준 담당 문서를 갱신하거나 새로 만드는 작업입니다. | 승격 시점의 담당 문서 갱신 | 승격 시점의 담당 문서 갱신 | 파일 경로와 앵커 | 향후 담당 문서가 이미 현재 기준 담당 문서인 것처럼 이름 붙이는 표현 | [작성 가이드](../maintain/authoring-guide.md), [이후 후보 색인](../later/index.md), [현재 MVP 범위](active-mvp-scope.md) |
| future owner placeholder | 향후 담당 문서를 만들거나 지정해야 할 수 있음을 알리는 계획 표현입니다. 현재 담당 문서가 아닙니다. | 향후 담당 문서 자리표시자 | 향후 담당 문서 자리표시자 | - | 자리표시자를 현재 기준 담당 문서처럼 독자에게 안내하는 표현 | [작성 가이드](../maintain/authoring-guide.md), [이후 후보 색인](../later/index.md) |
| `Task` | 구체화, 실행, 차단, 닫기의 대상이 되는 사용자 가치 단위입니다. | `Task` | 정확한 엔티티가 필요 없을 때는 작업 | `Task`, `task_id`, `active_task_id` | 식별자 번역, 하네스 엔티티가 필요한 자리에서 일반 할 일처럼 쓰는 표현 | [Core 모델](core-model.md), [API 상태 스키마](api/schema-state.md), [API 값 집합](api/schema-value-sets.md) |
| scope | 현재 Task나 Change Unit이 포함하고 제외하는 합의된 경계입니다. | 범위 | 범위 | `scope`, `scope_decision`, `AuthorizedAttemptScope`, `SensitiveActionScope` | 스코프, 조용한 범위 확장, 광범위한 승인 | [Core 모델](core-model.md), [MVP API](api/mvp-api.md), [API 판단 스키마](api/schema-judgment.md) |
| user-owned judgment | 하네스가 추론하지 않고 사용자에게 묻거나 사용자 선택으로 보존해야 하는 결정입니다. | 사용자 소유 판단 | 사용자 판단 | `user_judgment`, `UserJudgment`, `judgment_kind` 값 | 광범위한 승인을 수락, 잔여 위험 수락, 범위 변경, Write Authorization으로 보는 표현 | [Core 모델](core-model.md), [API 판단 스키마](api/schema-judgment.md) |
| close readiness | 현재 담당 상태를 기준으로 Task를 정직하게 닫을 수 있는지의 의미입니다. | 닫기 준비 상태 | 닫기 가능 여부 | 스키마를 이름 붙일 때 `CloseReadinessBlocker` | 영어와 한국어가 섞인 평가 표현 | [Core 모델](core-model.md), [MVP API](api/mvp-api.md), [API 오류](api/errors.md) |
| close readiness evaluation | 닫기 준비 상태와 남은 닫기 차단 사유를 도출하는 담당 경로의 확인입니다. | 닫기 준비 상태 평가 | 닫기 준비 상태 평가 | `harness.close_task`, `CloseTaskResult`, `CloseReadinessBlocker` | 영어와 한국어가 섞인 평가 표현 | [Core 모델](core-model.md), [MVP API](api/mvp-api.md), [API 오류](api/errors.md) |
| close blocker | 담당 경로에서 처리하기 전까지 정직한 닫기를 막는 닫기 관련 이유입니다. | 닫기 차단 사유 | 닫기 차단 사유 | `close_blockers`, API 데이터를 이름 붙일 때 `CloseReadinessBlocker` | 닫기 차단 사유를 영어로 남기는 표현 | [Core 모델](core-model.md), [API 상태 스키마](api/schema-state.md), [API 오류](api/errors.md) |
| `CloseReadinessBlocker` | 닫기 준비 상태의 차단 데이터를 나타내는 API 스키마 식별자입니다. 닫기 준비 상태 전체 개념은 아닙니다. | `CloseReadinessBlocker` | 스키마를 말하지 않을 때는 닫기 차단 사유 | `CloseReadinessBlocker`, `CloseReadinessBlocker.code` | 식별자 번역, prepare-write 판단 사유처럼 쓰는 표현 | [API 상태 스키마](api/schema-state.md), [API 값 집합](api/schema-value-sets.md), [API 오류](api/errors.md) |
| artifact | 아티팩트 담당 문서가 추적하는 하네스 관련 자료입니다. 정확한 저장 동작은 아티팩트 계약이 담당합니다. | 아티팩트 | 아티팩트 | `ArtifactRef`, `ArtifactInput`, `StagedArtifactHandle`, `artifact_id` | 아티팩트를 영어로 남긴 저장/바이트 표현, 원시 경로를 권한 근거로 쓰는 표현 | [API 아티팩트 스키마](api/schema-artifacts.md), [아티팩트 저장소](storage-artifacts.md) |
| `ArtifactRef` | 등록된 지속 아티팩트를 가리키는 공개 포인터입니다. | `ArtifactRef` | 스키마를 말하지 않을 때는 아티팩트 참조 | `ArtifactRef`, `existing_artifact_ref` | 식별자 번역, 표시된 참조를 본문 읽기 권한이나 증거 충분성으로 보는 표현 | [API 아티팩트 스키마](api/schema-artifacts.md), [아티팩트 저장소](storage-artifacts.md) |
| `StagedArtifactHandle` | 성공한 아티팩트 스테이징이 반환하는 임시 핸들입니다. 그 자체로 지속 아티팩트 권한이 아닙니다. | `StagedArtifactHandle` | 스테이징된 아티팩트 핸들 | `StagedArtifactHandle`, `staged_artifact_handle` | 스테이징 핸들을 영어로 남긴 표현, 베어러 토큰, 지속 아티팩트 | [API 아티팩트 스키마](api/schema-artifacts.md), [아티팩트 저장소](storage-artifacts.md) |
| projection | 담당 기록에서 만든 읽기 전용 파생 표시 또는 지원 맥락입니다. | 상태 보기 | 상태 보기 | 정확한 라벨을 말할 때 `Projection`, `ProjectionKind` | 렌더링된 표시를 Core 상태, 증거, 수락, 권한으로 보는 표현 | [상태 보기 권한 참조](projection-and-templates.md), [템플릿 본문](template-bodies.md) |
| surface | 하네스가 쓰이거나 관찰되는 사용자, 에이전트, 도구, 커넥터, 로컬 맥락입니다. | 접점 | 접점 | `surface_id`, `surface_instance_id`, `VerifiedSurfaceContext` | 접점 정보를 영어로 남긴 표현, 접점 권한처럼 쓰는 표현, `surface_id`를 권한 증거로 보는 표현 | [에이전트 통합](agent-integration.md), [보안](security.md) |
| runtime | 향후 실행되는 하네스 서버/런타임 동작과 런타임 데이터 공간입니다. | 런타임 | 런타임 | `Harness Runtime Home` | Markdown 원천 문서를 런타임 상태나 생성된 런타임 출력으로 보는 표현 | [런타임 경계](runtime-boundaries.md), [보안](security.md) |
| `Write Authorization` | 호환되는 제품 파일 쓰기 시도 하나를 위한 Core 권한 부여입니다. OS 권한이나 민감 동작 승인이 아닙니다. | 쓰기 권한 부여 | 쓰기 권한 부여 | `Write Authorization`, `AuthorizedAttemptScope`, `WriteAuthorization.basis_state_version` | write permission, command approval, 민감 동작 승인 대체 표현 | [Core 모델](core-model.md), [보안](security.md), [MVP API](api/mvp-api.md) |
| sensitive approval / sensitive-action approval | 이름 붙은 민감 동작에 대한 사용자 판단입니다. 영어 산문에서는 "sensitive-action approval"을 기본으로 씁니다. | 민감 동작 승인 | 민감 동작 승인 | `sensitive_approval`, `SensitiveActionScope` | Write Authorization, 최종 수락, 광범위한 승인처럼 쓰는 표현 | [Core 모델](core-model.md), [API 판단 스키마](api/schema-judgment.md), [보안](security.md) |
| access class | API와 보안 담당 문서가 보호된 읽기, 변경, 아티팩트, 커넥터 접근 기대를 설명할 때 쓰는 분류입니다. | 접근 등급 | 접근 등급 | `access_class`, `VerifiedSurfaceContext.access_class` | 접근 등급을 OS 권한이나 광범위한 권한으로 보는 표현 | [API 값 집합](api/schema-value-sets.md), [MVP API](api/mvp-api.md), [보안](security.md) |
| active guarantee | 현재 MVP가 실제로 주장하는 보장입니다. 현재 범위와 보안 담당 문서가 모두 현재 동작으로 문서화해야 합니다. | 현재 활성 보장 | 현재 활성 보장 | 정확한 보장 라벨 값 | 예약된 값이나 profile-gated 값을 현재 활성 보장처럼 쓰는 표현 | [보안](security.md), [현재 MVP 범위](active-mvp-scope.md), [API 값 집합](api/schema-value-sets.md) |
| cooperative guarantee | 접점이 문서화된 절차를 따를 때 하네스가 담당 경로의 상태 변경을 안내, 기록, 비교, 거부할 수 있다는 보장 수준입니다. | 협력형 보장 | 협력형 보장 | 값 이름을 말할 때 `cooperative` | `detective`, `preventive`, `isolated`, 샌드박스, 강제 차단처럼 강화하는 표현 | [보안](security.md) |
| detective guarantee | 관련 역량 확인이 통과한 뒤 지원되는 관찰 사실을 보고할 수 있다는 보장 수준입니다. | 탐지형 보장 | 탐지형 보장 | 값 이름을 말할 때 `detective` | 전체 모니터링이나 예방처럼 말하는 표현 | [보안](security.md), [에이전트 통합](agent-integration.md) |
| preventive guarantee | 정확한 예방 메커니즘과 증명 경로가 문서화되었을 때만 동작을 막을 수 있다는 보장 수준입니다. | 예방형 보장 | 예방형 보장 | 값 이름을 말할 때 `preventive` | 현재 MVP 샌드박싱이나 권한 제어처럼 말하는 표현 | [보안](security.md), [이후 후보 색인](../later/index.md) |
| `isolated` | 이후/profile-gated 보장 라벨로 예약된 값입니다. 현재 MVP의 현재 활성 보장이 아닙니다. | `isolated` | `isolated` | `isolated` | 격리 보장이 제공됩니다, 현재 격리됩니다, 현재 MVP가 isolated 보장을 제공합니다 | [보안](security.md), [API 값 집합](api/schema-value-sets.md) |
| dry-run | 선택된 동작의 유효한 미리보기 경로입니다. 쓰기를 커밋하거나 담당 기록을 만들지 않습니다. | dry-run 미리보기 | 미리보기 | `dry_run`, `ToolDryRunResponse`, `DryRunSummary`, `PlannedBlocker` | dry-run 출력을 커밋된 상태, 저장된 차단 사유, `CloseReadinessBlocker`로 보는 표현 | [API 코어 스키마](api/schema-core.md), [MVP API](api/mvp-api.md), [API 오류](api/errors.md), [저장 효과](storage-effects.md) |
| blocked result | 공개 전송 오류나 스키마 거부가 아니라 메서드가 반환하는 동작별 차단 결과입니다. | 차단 결과 | 차단 결과 | `CloseTaskResult(close_state=blocked)`, `decision=blocked`, `WriteDecisionReason`, `CloseReadinessBlocker` | 거부 응답, 공개 오류, `STATE_VERSION_CONFLICT`를 차단 코드로 쓰는 표현 | [API 오류](api/errors.md), [MVP API](api/mvp-api.md), [저장 효과](storage-effects.md) |
| rejected response | 메서드가 커밋 동작으로 진행하기 전 실패했음을 나타내는 `ToolRejectedResponse` 분기입니다. | 거부 응답 | 거부 응답 | `ToolRejectedResponse`, `ToolError`, `ErrorCode` | 차단 결과, 닫기 차단 사유, 커밋된 결과 | [API 코어 스키마](api/schema-core.md), [API 오류](api/errors.md), [저장 효과](storage-effects.md) |
| lifecycle | Task나 아티팩트 핸들 같은 개념에서 허용되는 단계 진행입니다. | 생명주기 | 생명주기 | `Task.lifecycle_phase`, `artifact_staging.status` | 생명주기를 영어로 남긴 표현 | 개념별 담당 문서: [Core 모델](core-model.md), [API 값 집합](api/schema-value-sets.md), [아티팩트 저장소](storage-artifacts.md) |

## 용어 통제

[docs/terminology-map.yaml](../../terminology-map.yaml)은 한영 용어 선택, 식별자 보존, 피해야 할 혼합 표현을 관리하는 기준 기계 판독 용어 통제 파일입니다. 용어가 바뀌면 같은 문서 전용 작업 묶음에서 이 용어집과 용어 지도를 함께 맞춥니다.
