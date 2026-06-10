# 용어집 참조

이 문서는 하네스 문서의 공식 용어를 담당합니다. 독자와 번역자를 위한 용어 의미를 정리하지만, 정확한 스키마, 값 집합, DDL, 저장 효과, 보안 메커니즘, API 동작, 구현 순서를 정의하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 제품, Core, API, 저장소, 보안, 에이전트, 상태 보기, 이후 후보 개념의 공식 한국어/영어 용어
- 문서 산문에서 쓰는 용어 수준 의미
- 용어에서 기준 기술 담당 문서로 가는 링크

이 문서는 담당하지 않습니다.

- 정확한 API 필드 형태나 enum 형태 값: API 스키마 담당 문서와 [API 값 집합](api/schema-value-sets.md)
- 공개 오류 코드: [API Errors](api/errors.md)
- 저장소 기록, 효과, 아티팩트, 버전 관리, 잠금, 마이그레이션: [참조 색인](README.md)의 저장소 담당 문서
- 템플릿 본문: [템플릿 본문](template-bodies.md)
- 구현 준비: [MVP 계획](../build/mvp-plan.md)

## 제품 용어

| 용어 | 의미 | 담당 문서 |
|---|---|---|
| 하네스 | AI 지원 제품 작업을 위한 향후 로컬 작업 권한 서버입니다. | [현재 MVP 범위](active-mvp-scope.md), [런타임 경계](runtime-boundaries.md) |
| Product Repository | 사용자의 프로젝트 작업 공간입니다. 제품 파일은 하네스 런타임 상태가 아닙니다. | [런타임 경계](runtime-boundaries.md) |
| Harness Runtime Home | 향후 하네스 기록과 아티팩트를 담는 운영 데이터 공간입니다. 이 문서 저장소는 Runtime Home이 아닙니다. | [런타임 경계](runtime-boundaries.md), 저장소 담당 문서 |
| 문서 전용 | 현재 저장소와 편집 범위가 문서 작업뿐이라는 뜻입니다. 런타임 구현이나 생성된 런타임 기록을 승인하지 않습니다. | [작성 가이드](../maintain/authoring-guide.md) |
| 현재 MVP | 첫 로컬 작업 루프를 위한 활성 제품 범위 경계입니다. | [현재 MVP 범위](active-mvp-scope.md) |
| 이후 후보 | 담당 문서가 승격하기 전까지 활성 범위가 아닌 미뤄 둔 자료입니다. | [이후 후보 색인](../later/index.md) |

## Core 용어

| 용어 | 의미 | 담당 문서 |
|---|---|---|
| Core가 소유한 상태 | 작업 권한을 담는 하네스 소유 기록입니다. | [Core Model](core-model.md), 저장소 담당 문서 |
| 사용자 소유 판단 | 하네스가 추론하지 않고 사용자에게 묻거나 사용자의 선택으로 보존해야 하는 결정입니다. | [Core Model](core-model.md), [API 판단 스키마](api/schema-judgment.md) |
| 민감 동작 승인 | 이름 붙은 민감한 행동에 대한 사용자 판단입니다. Write Authorization이나 최종 수락이 아닙니다. | [Core Model](core-model.md), [보안](security.md) |
| 최종 수락 | 담당 경로가 요구할 때 결과를 받아들이는 사용자 판단입니다. | [Core Model](core-model.md) |
| 잔여 위험 수락 | 요구될 때 보이는 잔여 위험을 받아들이는 사용자 판단입니다. | [Core Model](core-model.md) |
| 닫기 준비 상태 | 현재 작업을 정직하게 닫을 수 있는지와 남은 차단 사유를 보여주는 상태입니다. | [Core Model](core-model.md), [API 상태 스키마](api/schema-state.md) |
| 닫기 준비 상태 평가 | 닫기 준비 상태와 남은 닫기 차단 사유를 도출하는 확인입니다. | [Core Model](core-model.md), [API 상태 스키마](api/schema-state.md) |
| 닫기 차단 사유 | 담당 경로에서 처리하기 전까지 작업을 정직하게 닫을 수 없게 만드는 이유입니다. | [Core Model](core-model.md), [API 상태 스키마](api/schema-state.md) |
| 차단 사유 | 작업 진행이나 닫기를 막는 일반적인 이유입니다. 닫기 준비 상태에서 나온 이유는 닫기 차단 사유라고 씁니다. | 차단된 개념의 담당 문서 |

## API와 스키마 용어

| 용어 | 의미 | 담당 문서 |
|---|---|---|
| `ToolEnvelope` | 공개 메서드가 사용하는 공통 요청 래퍼입니다. | [API Schema Core](api/schema-core.md) |
| 응답 분기 | 메서드 결과, `ToolRejectedResponse`, `ToolDryRunResponse` 중 하나입니다. | [API Schema Core](api/schema-core.md), [MVP API](api/mvp-api.md) |
| `ErrorCode` | 공개 API 오류 식별자입니다. | [API Errors](api/errors.md) |
| `StateSummary` | API 상태 형태 요약입니다. | [API 상태 스키마](api/schema-state.md) |
| `UserJudgment` | 사용자 소유 판단 기록 또는 후보의 API 형태입니다. | [API 판단 스키마](api/schema-judgment.md) |
| `CloseReadinessBlocker` | 닫기 차단 사유의 API 스키마 식별자입니다. | [API 상태 스키마](api/schema-state.md) |
| `ArtifactRef` | 지속 아티팩트를 가리키는 공개 포인터입니다. | [API 아티팩트 스키마](api/schema-artifacts.md), [아티팩트 저장소](storage-artifacts.md) |
| `ArtifactInput` | 아티팩트 입력 API 스키마 식별자입니다. | [API 아티팩트 스키마](api/schema-artifacts.md) |
| `StagedArtifactHandle` | 스테이징된 아티팩트 핸들의 API 스키마 식별자입니다. | [API 아티팩트 스키마](api/schema-artifacts.md), [아티팩트 저장소](storage-artifacts.md) |
| API 값 집합 | 활성 enum 형태 API 값의 기준 목록입니다. | [API 값 집합](api/schema-value-sets.md) |

## 저장소 용어

| 용어 | 의미 | 담당 문서 |
|---|---|---|
| 저장소 기록 | 향후 지속 저장되는 하네스 기록 형태입니다. | [저장소 기록](storage-records.md) |
| 저장 효과 | 메서드 분기가 저장소를 바꾸는지, 효과가 없는지를 나타냅니다. | [저장 효과](storage-effects.md) |
| 아티팩트 | 아티팩트 담당 문서가 추적하는 하네스 관련 자료입니다. 정확한 저장 동작은 아티팩트 계약이 담당합니다. | [API 아티팩트 스키마](api/schema-artifacts.md), [아티팩트 저장소](storage-artifacts.md) |
| 아티팩트 저장소 생명주기 | 스테이징, 승격, 지속 연결, 본문 읽기 자격, 보존, 무결성 경계입니다. | [아티팩트 저장소](storage-artifacts.md) |
| 상태 버전 관리 | 공개 상태 시계, 멱등성, 잠금, 마이그레이션 의미입니다. | [저장소 버전 관리](storage-versioning.md) |

## 보안과 에이전트 용어

| 용어 | 의미 | 담당 문서 |
|---|---|---|
| 협력형 보장 | 접점이 절차를 따를 때 하네스가 담당 경로의 상태 변경을 안내하거나, 기록하거나, 비교하거나, 거절할 수 있다는 뜻입니다. | [보안](security.md) |
| 탐지형 보장 | 관련 역량 확인이 통과한 뒤 지원되는 관찰 사실을 보고할 수 있다는 뜻입니다. | [보안](security.md), [에이전트 통합](agent-integration.md) |
| 예방형 보장 | 정확한 예방 메커니즘과 증명 경로가 문서화되었을 때만 하네스가 행동을 막을 수 있다는 뜻입니다. | [보안](security.md) |
| 쓰기 권한 부여(`Write Authorization`) | 제품 파일 쓰기 범위를 위한 이름 붙은 권한 부여입니다. 민감 동작 승인이 아닙니다. | [보안](security.md), [Core Model](core-model.md) |
| 접근 등급 | 담당 문서가 접근 기대나 경계를 설명할 때 쓰는 분류입니다. | [보안](security.md), [에이전트 통합](agent-integration.md) |
| 접점 | 하네스가 쓰이거나 관찰되는 사용자, 에이전트, 도구, 커넥터 맥락입니다. | [에이전트 통합](agent-integration.md), [보안](security.md) |
| `surface_id` | 접점 식별자입니다. 그 자체로 권한 증거가 아닙니다. | [에이전트 통합](agent-integration.md), [보안](security.md) |
| `capability_profile` | 접점이 지원하는 역량을 설명하는 커넥터 담당 정보입니다. | [에이전트 통합](agent-integration.md) |
| 접점별 사용 레시피 | 이름 붙은 접점에서 쓰는 실무 안내입니다. | [접점별 사용 레시피](../use/surface-recipes.md) |

## 상태 보기 및 템플릿 본문 용어

| 용어 | 의미 | 담당 문서 |
|---|---|---|
| 상태 보기 | 담당 기록에서 만든 읽기 전용 파생 표시 또는 지원 맥락입니다. 참조 문서에서 영어 라벨이 필요하면 첫 언급에 `Projection`(읽기 전용 상태 보기)처럼 씁니다. | [상태 보기 권한 참조](projection-and-templates.md) |
| 렌더링된 라벨 | 사용자에게 보이는 표시 문구입니다. 기준 스키마 값이 아닙니다. | [상태 보기 권한 참조](projection-and-templates.md), [템플릿 본문](template-bodies.md) |
| 템플릿 본문 | 상태 보기 권한과 분리되어 관리되는 정확한 렌더링 문구입니다. | [템플릿 본문](template-bodies.md) |

## 용어 통제

[docs/terminology-map.yaml](../../terminology-map.yaml)은 한영 용어 선택, 식별자 보존, 피해야 할 혼합 표현을 관리하는 기준 기계 판독 용어 통제 파일입니다. 이 용어집은 독자용 의미를 설명하며, 용어가 바뀌면 용어집과 용어 지도를 함께 맞춥니다.
