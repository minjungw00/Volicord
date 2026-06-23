# 용어집

이 용어집은 선별된 Volicord 핵심 용어를 사람이 읽기 쉽게 설명하는 간결한 안내입니다. 주요 개념을 이해하고, 포함된 각 용어의 주 담당 문서를 찾을 때 사용합니다.

완전한 구조화 용어 메타데이터는 [`docs/terminology-map.yaml`](../../terminology-map.yaml)에 있습니다. 이 용어집은 독자를 위한 선별 부분집합일 뿐입니다.

이 용어집은 일부 항목만 반복합니다.

- 용어 라벨
- 한국어 용어
- 간결한 뜻
- 주 담당 문서

선호 표현, 피할 표현, 식별자 보존 통제, 주변 참조는 용어 지도에 남깁니다.

주제별 담당 문서 찾기는 [참조 색인](README.md)을 사용합니다. `doc_id` 기준의 정확한 기계 판독 경로는 [`docs/doc-index.yaml`](../../doc-index.yaml)을 사용합니다.

계약 세부사항은 각각의 집중 담당 문서에서 확인합니다. 번역과 문체 규칙은 [번역 정책](../maintain/translation-policy.md)에 둡니다.

## 용어

| 용어 | 한국어 용어 | 짧은 의미 | 주 담당 문서 |
|---|---|---|---|
| Volicord | Volicord | AI 지원 제품 작업에서 로컬 작업 권한을 다루는 제품이자 시스템입니다. | [기준 범위](scope.md) |
| Core | Core | Volicord 상태를 위한 로컬 기준 기록입니다. | [Core 모델](core-model.md) |
| Volicord 구현 | Volicord 구현 | 이 저장소가 유지하는 서버 구현 집합입니다. 소스 수준의 크레이트, 실행 파일 역할, 테스트, 문서, 검증 도구, 저장소 설정을 포함하며 Volicord 전체와 같은 말은 아닙니다. | [런타임 경계](runtime-boundaries.md) |
| `Product Repository` | 제품 저장소 | Volicord 런타임 상태와 구분되는 사용자의 프로젝트 작업 공간과 제품 파일입니다. | [런타임 경계](runtime-boundaries.md) |
| `Volicord Runtime Home` | 런타임 홈 | 저장소/런타임 담당 문서가 정의한 Volicord 운영 데이터의 로컬 런타임 데이터 공간입니다. | [런타임 경계](runtime-boundaries.md) |
| runtime | 런타임 | Volicord의 운영 실행과 데이터 맥락입니다. | [런타임 경계](runtime-boundaries.md) |
| baseline scope | 기준 범위 | Volicord가 안정적으로 지원한다고 문서화한 경계입니다. | [기준 범위](scope.md) |
| out-of-scope capability | 지원 범위 밖 기능 | 기준 지원 경계 밖에 있는 유예된 기능입니다. | [기준 범위](scope.md) |
| owner document | 담당 문서 | 용어, 제품 개념, 계약을 정의하는 기준 문서입니다. | [문서 정책](../maintain/documentation-policy.md) |
| applicable owner path | 적용되는 담당 경로 | 질문이나 개념에 맞는 집중 담당 문서로 가는 문서 경로입니다. | [문서 정책](../maintain/documentation-policy.md) |
| `Task` | `Task` | 범위, 권한 맥락, 판단, 증거, 닫기 준비 상태를 묶는 Volicord 개체입니다. | [Core 모델](core-model.md) |
| scope | 범위 | `Task` 또는 Change Unit 맥락에 붙는 작업 또는 권한 경계입니다. | [Core 모델](core-model.md) |
| current scope | 현재 적용 범위 | `Task` 또는 Change Unit 맥락 안에서 현재 적용되는 범위입니다. | [Core 모델](core-model.md) |
| current Change Unit | 현재 적용 Change Unit | 권한 모델 안에서 현재 적용되는 Change Unit입니다. | [Core 모델](core-model.md) |
| user-owned judgment | 사용자 소유 판단 | 기록되지만 Core 소유 사실이 되지 않는 사용자 결정이나 평가입니다. | [Core 모델](core-model.md) |
| evidence | 증거 | 특정 범위에서 특정 주장을 뒷받침하는 기록입니다. | [Core 모델](core-model.md) |
| verification criteria | 검증 기준 | 작업을 확인하기 위해 사용자가 볼 수 있는 기준입니다. | [Core 모델](core-model.md) |
| artifact | 아티팩트 | Volicord 아티팩트 개념으로 참조되거나 스테이징되는 작업 자료입니다. | [API 아티팩트 스키마](api/schema-artifacts.md) |
| `Write Authorization` | 쓰기 권한 부여 | 호환되는 제품 파일 쓰기 시도 하나에 대한 `Core` 권한을 가리키는 정확한 Volicord 제품 라벨입니다. | [Core 모델](core-model.md) |
| write approval | 쓰기 승인 | 쓰기를 승인한다는 일반 사용자 승인이나 산문 표현입니다. `Write Authorization`과 구분됩니다. | [Core 모델](core-model.md) |
| sensitive-action approval | 민감 동작 승인 | 이름 붙은 민감 단계에 대한 사용자 승인이며, `Write Authorization`과 최종 수락과 구분됩니다. | [Core 모델](core-model.md) |
| final acceptance | 최종 수락 | 보이는 닫기 근거를 받아들일 수 있는지에 대한 사용자 소유 판단입니다. | [Core 모델](core-model.md) |
| residual-risk acceptance | 잔여 위험 수락 | 이름 붙은 보이는 잔여 위험에 대한 사용자 소유 판단입니다. | [Core 모델](core-model.md) |
| close readiness | 닫기 준비 상태 | 현재 상태에서 `Task`를 닫을 준비가 되었는지를 나타내는 Core 개념입니다. | [Core 모델](core-model.md) |
| close-readiness blocker | 닫기 차단 사유 | 닫기 준비 상태가 진행되지 못할 때 드러나는 닫기 관련 사유입니다. | [API 차단 사유 처리 경로](api/blocker-routing.md) |
| `Projection` | 상태 보기 | 읽기 전용 상태 보기를 뜻하는 정확한 제품 라벨입니다. 상태 보기 출력은 표시이지 `Core` 권한이 아닙니다. | [상태 보기 권한 참조](projection-and-templates.md) |
| `Agent Integration Profile` | 에이전트 통합 프로필 | 코딩 에이전트 통합 하나와 묶인 접점 맥락을 위한 지속되는 레지스트리 식별 정보입니다. | [에이전트 통합](agent-integration.md) |
| integration project membership | 통합 프로젝트 멤버십 | Agent Integration Profile과 등록된 프로젝트 사이의 명시적 허용 목록 관계입니다. | [에이전트 통합](agent-integration.md) |
| `Host Installation` | 호스트 설치 | 코딩 에이전트 통합을 위해 관리되는 호스트 설정 인벤토리입니다. 외부 호스트가 서버를 신뢰하거나 로드했다는 증명은 아닙니다. | [에이전트 통합](agent-integration.md) |
| `volicord.list_projects` | `volicord.list_projects` | 묶인 통합에 허용된 프로젝트를 나열하는 MCP 어댑터 유틸리티입니다. 공개 Core API 메서드가 아닙니다. | [MCP 전송](mcp-transport.md) |
| surface | 접점 | 맥락이 드러나는 통합 또는 상호작용 접점입니다. | [에이전트 통합](agent-integration.md) |
| access class | 접근 등급 | 검증된 접점과 접근 맥락을 분류하는 값 범주입니다. | [API 값 집합](api/schema-value-sets.md) |
| baseline guarantee | 기준 범위 보장 | 기준 범위에서 지원되는 보장을 말할 때 쓰는 보안 표현입니다. | [보안](security.md) |
| `ErrorCode` | 공개 오류 코드 | 공개 API 오류 코드 식별자입니다. | [API 오류 코드](api/error-codes.md) |
