# 참조 색인

이 색인은 하네스 참조 질문의 담당 문서를 고르는 경로입니다. 이 README는 담당 문서로 안내할 뿐 API 계약, 스키마, 저장 효과, 보안 보장, 범위를 정의하지 않습니다.

`doc_id` 기준의 기계 판독 가능한 경로는 [`docs/doc-index.yaml`](../../doc-index.yaml)을 사용합니다.

## 제품과 시스템 담당 문서

| 주제 | 담당 문서 |
|---|---|
| 범위 질문 | [`scope.md`](scope.md) |
| Core 권한, 제품 개념, 사용자 소유 판단, 닫기 준비 상태 | [`core-model.md`](core-model.md) |
| 런타임과 제품 저장소 경계 | [`runtime-boundaries.md`](runtime-boundaries.md) |
| 보안 표현과 보장 의미 | [`security.md`](security.md) |
| 제품 용어 | [`glossary.md`](glossary.md), [`docs/terminology-map.yaml`](../../terminology-map.yaml) |
| 구현 진입 경로 | [`../build/implementation-guide.md`](../build/implementation-guide.md) |

## API와 스키마 담당 문서

| 주제 | 담당 문서 |
|---|---|
| 공개 API 메서드 목록과 메서드 경로 | [`api/methods.md`](api/methods.md) |
| 공통 요청 래퍼와 응답 분기 | [`api/schema-core.md`](api/schema-core.md) |
| 상태와 닫기 준비 상태 형태 | [`api/schema-state.md`](api/schema-state.md) |
| 아티팩트 참조 형태 | [`api/schema-artifacts.md`](api/schema-artifacts.md) |
| 사용자 판단과 민감 동작 승인 스키마 | [`api/schema-judgment.md`](api/schema-judgment.md) |
| API 값 집합 | [`api/schema-value-sets.md`](api/schema-value-sets.md) |
| API 오류 문서 묶음 경로 | [`api/errors.md`](api/errors.md) |
| 공개 `ErrorCode` 식별자와 의미 | [`api/error-codes.md`](api/error-codes.md) |
| API 오류 우선순위와 상태 충돌 동작 | [`api/error-precedence.md`](api/error-precedence.md) |
| API 오류와 차단 사유 경로 | [`api/error-routing.md`](api/error-routing.md) |
| 기계 판독용 `ToolError.details` | [`api/error-details.md`](api/error-details.md) |

## 저장소 담당 문서

| 주제 | 담당 문서 |
|---|---|
| 저장소 문서 묶음 경로 | [`storage.md`](storage.md) |
| 저장소 기록 | [`storage-records.md`](storage-records.md) |
| 저장 효과 | [`storage-effects.md`](storage-effects.md) |
| 아티팩트 저장소 | [`storage-artifacts.md`](storage-artifacts.md) |
| 상태 시계, 버전 관리, 마이그레이션 | [`storage-versioning.md`](storage-versioning.md) |
| 런타임 홈 분리 | [`runtime-boundaries.md`](runtime-boundaries.md) |

## 접점, 상태 보기, 품질 담당 문서

| 주제 | 담당 문서 |
|---|---|
| 에이전트 통합과 현재 적용 접점 맥락 | [`agent-integration.md`](agent-integration.md) |
| 접점별 사용 레시피 | [`../use/surface-recipes.md`](../use/surface-recipes.md) |
| 권한과 상태 보기/상태 카드/템플릿 보기의 구분 | [`projection-and-templates.md`](projection-and-templates.md) |
| 템플릿 본문, 라벨, 표시 문구 | [`template-bodies.md`](template-bodies.md) |
| 적합성 참조 | [`conformance.md`](conformance.md) |
| 설계 품질 참조 | [`design-quality.md`](design-quality.md) |

## 사용자 판단과 닫기 준비 상태 담당 문서

| 주제 | 담당 문서 |
|---|---|
| 사용자 소유 판단 의미 | [`core-model.md`](core-model.md) |
| 사용자 판단 메서드 | [`api/method-user-judgment.md`](api/method-user-judgment.md) |
| 사용자 판단 스키마 | [`api/schema-judgment.md`](api/schema-judgment.md) |
| 닫기 준비 상태 의미 | [`core-model.md`](core-model.md) |
| 닫기 메서드 | [`api/method-close-task.md`](api/method-close-task.md) |
| 닫기 준비 상태 형태 | [`api/schema-state.md`](api/schema-state.md) |
| 닫기 오류 경로 | [`api/error-routing.md`](api/error-routing.md) |

## 유지보수와 메타데이터

| 필요 | 경로 |
|---|---|
| 저장소 편집 규칙 | [`../../../AGENTS.md`](../../../AGENTS.md) |
| 기계 판독 가능한 담당 문서 경로 | [`../../doc-index.yaml`](../../doc-index.yaml) |
| 문서 작성 규칙 | [`../maintain/authoring-guide.md`](../maintain/authoring-guide.md) |
| 문서 점검 색인 | [`../maintain/checks.md`](../maintain/checks.md) |
| 번역 지침 | [`../maintain/translation-guide.md`](../maintain/translation-guide.md) |
| 한영 용어 통제 | [`../../terminology-map.yaml`](../../terminology-map.yaml) |
