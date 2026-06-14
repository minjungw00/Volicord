# 용어집

이 용어집은 하네스 문서의 사람이 읽는 간결한 용어 뜻 안내입니다. 용어의 뜻과 그 용어의 주 담당 문서를 빠르게 확인할 때 사용합니다.

이 문서는 참조 담당 문서 전체 색인이 아닙니다. 주제별 담당 문서 찾기는 [참조 색인](README.md)을 사용합니다. `doc_id` 기준의 정확한 기계 판독 경로는 [`docs/doc-index.yaml`](../../doc-index.yaml)을 사용합니다.

구조화된 용어 정보, 식별자 보존 규칙, 한국어 혼합어 규칙은 [docs/terminology-map.yaml](../../terminology-map.yaml)에 있습니다. 정확한 API 동작, 스키마, 저장 효과, 보안 보장, 닫기 준비 상태 동작, 오류 처리 경로는 연결된 담당 문서를 따릅니다.

## 용어

| 용어 | 한국어 용어 | 짧은 뜻 | 주 담당 문서 |
|---|---|---|---|
| Harness | 하네스 | AI 지원 제품 작업을 위한 로컬 작업 권한 서버입니다. | [기준 범위](scope.md) |
| `Product Repository` | `Product Repository`; 제품 저장소 | 하네스 런타임 상태와 구분되는 사용자의 프로젝트 작업 공간입니다. | [런타임 경계](runtime-boundaries.md) |
| `Harness Runtime Home` | `Harness Runtime Home`; 런타임 홈 | 하네스 기록과 아티팩트를 담는 운영 데이터 공간입니다. | [런타임 경계](runtime-boundaries.md) |
| documentation | 문서 | 유지되는 원천 자료입니다. 런타임 출력, 구현, 수락 상태와 구분합니다. | [작성 가이드](../maintain/authoring-guide.md) |
| semantic skeleton | 의미 골격 | 중요한 참조 섹션을 다듬기 전에 정하는 의미 단위 구조입니다. | [작성 가이드](../maintain/authoring-guide.md) |
| baseline scope | 기준 범위 | 하네스가 안정적으로 지원한다고 문서화한 경계입니다. | [기준 범위](scope.md) |
| supported scope | 지원 범위 | 지원된다고 문서화된 동작이나 역량입니다. | [기준 범위](scope.md) |
| supported behavior | 지원 동작 | 기준 범위와 영향받는 담당 문서가 지원한다고 문서화한 동작입니다. | [기준 범위](scope.md) |
| supported API method | 지원되는 API 메서드 | 지원된다고 문서화된 공개 API 메서드입니다. | [API 메서드](api/methods.md) |
| supported API value | 지원되는 API 값 | 이름만 있거나 예약된 값이 아니라 지원된다고 문서화된 API 값입니다. | [API 값 집합](api/schema-value-sets.md) |
| out-of-scope capability | 지원 범위 밖 기능 | 기준 지원 경계 밖에 있는 유예된 기능입니다. | [기준 범위](scope.md) |
| evidence collection workflow | 증거 수집 흐름 | 지원 여부를 기준 범위가 담당하는 증거 흐름 표현입니다. | [기준 범위](scope.md) |
| expanded or additional evidence collection workflows | 확장 또는 추가 증거 수집 흐름 | 지원 범위 밖인 증거 수집 흐름 묶음입니다. | [기준 범위](scope.md) |
| owner document | 담당 문서 | 용어, 제품 개념, 계약을 정의하는 기준 문서입니다. | [작성 가이드](../maintain/authoring-guide.md) |
| owner contract | 담당 계약 | 관련 담당 문서가 정의한 계약입니다. | [작성 가이드](../maintain/authoring-guide.md) |
| applicable owner path | 적용되는 담당 경로 | 질문이나 개념에 맞는 집중 담당 문서로 가는 문서 경로입니다. | [작성 가이드](../maintain/authoring-guide.md) |
| applicable reference | 적용되는 참조 문서 | 관련 계약을 정의하는 참조 문서입니다. | [참조 색인](README.md) |
| existing owner | 기존 담당 문서 | 이미 존재하고 규범 의미를 담을 수 있는 담당 문서입니다. | [작성 가이드](../maintain/authoring-guide.md) |
| promotion-time owner update | 승격 시점의 담당 문서 갱신 | 지원 범위 밖 기능을 지원 범위로 승격할 때 필요한 담당 문서 변경입니다. | [기준 범위](scope.md) |
| owner placeholder | 담당 문서 자리표시자 | 지원 범위 밖 기능에 담당 문서가 아직 필요함을 나타내는 표시입니다. | [작성 가이드](../maintain/authoring-guide.md) |
| `Task` | `Task` | 범위, 권한 맥락, 판단, 증거, 닫기 준비 상태를 묶는 하네스 개체입니다. | [Core 모델](core-model.md) |
| scope | 범위 | `Task` 또는 Change Unit 맥락에 붙는 작업 또는 권한 경계입니다. | [Core 모델](core-model.md) |
| active scope | 현재 적용 범위 | `Task` 또는 Change Unit 맥락에서 현재 적용되는 범위입니다. | [Core 모델](core-model.md) |
| active Change Unit | 현재 적용 Change Unit | 권한 모델에서 현재 적용되는 Change Unit입니다. | [Core 모델](core-model.md) |
| user-owned judgment | 사용자 소유 판단 | 하네스가 기록하지만 Core 소유 사실로 바꾸지 않는 사용자 결정이나 평가입니다. | [Core 모델](core-model.md) |
| `UserJudgment` | `UserJudgment` | 사용자 소유 판단 데이터를 나타내는 API 스키마 식별자입니다. | [API 판단 스키마](api/schema-judgment.md) |
| close readiness | 닫기 준비 상태 | 현재 상태에서 `Task`를 닫을 준비가 되었는지를 나타내는 Core 개념입니다. | [Core 모델](core-model.md) |
| close readiness evaluation | 닫기 준비 상태 평가 | `Task` 닫기 평가를 가리키는 용어입니다. | [Task 닫기 메서드](api/method-close-task.md) |
| close task | `Task` 닫기 | 사용자 또는 API가 `Task` 닫기를 시도하는 동작입니다. | [Task 닫기 메서드](api/method-close-task.md) |
| close task behavior | `Task` 닫기 동작 | `Task` 닫기 API 동작 영역을 가리키는 표현입니다. | [Task 닫기 메서드](api/method-close-task.md) |
| `harness.close_task` | `harness.close_task` | `Task` 닫기의 공개 API 메서드 식별자입니다. | [Task 닫기 메서드](api/method-close-task.md) |
| close-readiness blocker | 닫기 차단 사유 | 닫기 준비 상태가 진행되지 못하는 사유입니다. | [API 차단 사유 처리 경로](api/blocker-routing.md) |
| `CloseReadinessBlocker` | `CloseReadinessBlocker` | 닫기 차단 사유 데이터를 나타내는 스키마 식별자입니다. | [API 상태 스키마](api/schema-state.md) |
| blocker category | 차단 사유 범주 | 닫기 차단 사유의 범주 개념입니다. | [API 값 집합](api/schema-value-sets.md) |
| blocker | 차단 사유 | 막힘의 이유를 가리키는 일반 산문 용어입니다. | [용어 지도](../../terminology-map.yaml) |
| complete intent | `complete` | 일반 산문의 "전체"와 구분되는 `complete` API 값입니다. | [API 값 집합](api/schema-value-sets.md) |
| full evaluation order | 전체 평가 순서 | `complete` API 값과 구분되는 일반 산문의 전체 평가 순서입니다. | [용어 지도](../../terminology-map.yaml) |
| artifact | 아티팩트 | 하네스 아티팩트 개념으로 참조되거나 스테이징되는 작업 자료입니다. | [API 아티팩트 스키마](api/schema-artifacts.md) |
| evidence | 증거 | 주장, 검증 결과, 사용자 판단 맥락을 뒷받침하는 기록 자료입니다. | [Core 모델](core-model.md) |
| `ArtifactRef` | `ArtifactRef` | 지속된 아티팩트 참조를 나타내는 스키마 식별자입니다. | [API 아티팩트 스키마](api/schema-artifacts.md) |
| `ArtifactInput` | `ArtifactInput` | 아티팩트 입력 데이터를 나타내는 스키마 식별자입니다. | [API 아티팩트 스키마](api/schema-artifacts.md) |
| `StagedArtifactHandle` | `StagedArtifactHandle` | 스테이징된 아티팩트 핸들을 나타내는 식별자입니다. | [API 아티팩트 스키마](api/schema-artifacts.md) |
| projection | 상태 보기 | 읽기 전용 상태 보기입니다. | [상태 보기 권한 참조](projection-and-templates.md) |
| `Projection` | `Projection` | 읽기 전용 상태 보기의 정확한 제품 라벨입니다. | [상태 보기 권한 참조](projection-and-templates.md) |
| surface | 접점 | 맥락이 드러나는 통합 또는 상호작용 접점입니다. | [에이전트 통합](agent-integration.md) |
| `surface_id` | `surface_id` | 접점을 나타내는 정확한 식별자입니다. | [에이전트 통합](agent-integration.md) |
| active surface context | 현재 적용 접점 맥락 | 요청이나 상호작용에 현재 적용되는 접점 맥락입니다. | [에이전트 통합](agent-integration.md) |
| `state_version` | `state_version` | 저장된 프로젝트 상태의 순서를 나타내는 식별자입니다. | [저장소 버전 관리](storage-versioning.md) |
| runtime | 런타임 | 하네스의 운영 실행과 데이터 맥락입니다. | [런타임 경계](runtime-boundaries.md) |
| `Write Authorization` | 쓰기 권한 부여 | 하네스 쓰기 권한 부여 개념의 정확한 제품 라벨입니다. | [Core 모델](core-model.md) |
| sensitive approval | 민감 동작 승인 | `Write Authorization`과 구분되는 민감 동작에 대한 사용자 승인입니다. | [Core 모델](core-model.md) |
| access class | 접근 등급 | 접근 맥락을 분류하는 값 범주입니다. | [API 값 집합](api/schema-value-sets.md) |
| baseline guarantee | 기준 범위 보장 | 기준 범위 보장에 쓰는 보안 용어입니다. | [보안](security.md) |
| cooperative guarantee | 협력형 보장 | 협력형 보장 유형에 쓰는 보안 용어입니다. | [보안](security.md) |
| detective guarantee | 탐지형 보장 | 탐지형 보장 유형에 쓰는 보안 용어입니다. | [보안](security.md) |
| design-quality owner boundary | 설계 품질 담당 경계 | 설계 품질 관찰을 관련 담당 문서로 보내는 경계입니다. | [설계 품질](design-quality.md) |
| reserved value | 예약된 값 | 그 자체만으로 기준 동작을 뜻하지 않는 예약 어휘나 노출 지점입니다. | [기준 범위](scope.md) |
| profile-gated value | 프로필 조건부 값 | 문서화된 프로필이나 게이트가 지원할 때만 사용할 수 있는 값입니다. | [기준 범위](scope.md) |
| `ErrorCode` | `ErrorCode` | 공개 API 오류 코드 식별자입니다. | [API 오류 코드](api/error-codes.md) |
| error code meanings | 공개 오류 코드 의미 | 공개 API 오류 코드의 의미 영역입니다. | [API 오류 코드](api/error-codes.md) |
| error precedence | 오류 우선순위 | API 오류 선택과 정렬 영역입니다. | [API 오류 우선순위](api/error-precedence.md) |
| error routing | 오류 처리 경로 | API 응답 분기 처리 경로 영역입니다. | [API 오류 처리 경로](api/error-routing.md) |
| blocker routing | 차단 사유 처리 경로 | 닫기 차단 사유와 API 응답 분기 사이의 경계입니다. | [API 차단 사유 처리 경로](api/blocker-routing.md) |
| error/blocker boundary | 오류와 차단 사유의 경계 | 공개 API 오류와 닫기 차단 사유 데이터의 구분입니다. | [API 차단 사유 처리 경로](api/blocker-routing.md) |
| public error as blocker | 공개 오류 코드가 차단 사유로 표현되는 경우 | 공개 오류 코드 표현이 차단 사유 데이터에 나타나는 경우를 가리키는 경계 용어입니다. | [API 차단 사유 처리 경로](api/blocker-routing.md) |
| `ToolError.details` | `ToolError.details` | 기계 판독용 오류 세부사항 필드입니다. | [API 오류 세부사항](api/error-details.md) |
| error detail helper values | 오류 세부사항 보조 값 | 기계 판독용 오류 세부사항 아래의 보조 값입니다. | [API 오류 세부사항](api/error-details.md) |
| dry-run | dry-run 미리보기 | `dry_run`을 사용하는 API 미리보기 모드입니다. | [API 코어 스키마](api/schema-core.md) |
| dry-run preview routing | dry-run 미리보기 처리 경로 | `dry_run` 미리보기 응답의 처리 경로 용어입니다. | [API 오류 처리 경로](api/error-routing.md) |
| blocked result | 차단 결과 | 차단을 보고하는 API 결과 분기입니다. | [API 오류 처리 경로](api/error-routing.md) |
| rejected response | 거부 응답 | 작업이 진행되기 전에 요청이 거부되었음을 나타내는 API 응답입니다. | [API 오류 처리 경로](api/error-routing.md) |
| migration | 마이그레이션 | 스키마, 저장소, 데이터, 문서에 적용되는 기술적 마이그레이션입니다. | [저장소 버전 관리](storage-versioning.md) |
| lifecycle | 생명주기 | 개체나 아티팩트가 시간에 따라 거치는 단계입니다. | [Core 모델](core-model.md) |
