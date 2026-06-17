# 참조 색인

이 색인은 하네스 참조 질문에서 다음 담당 문서를 고를 때 쓰는 읽기용 색인입니다. 기계가 읽는 정확한 담당 경로는 [`docs/doc-index.yaml`](../../doc-index.yaml)을 사용합니다. 이 파일이 `doc_id`, 대응 경로, 역할, 담당 범위, 의존 관계, 규범 수준, 독자 메타데이터를 담당합니다.

이 README는 경로 안내 전용입니다. 용어 뜻, 용어 메타데이터, API 동작, 오류 의미, 오류 우선순위, 응답 분기 처리 경로, 차단 사유 처리 경로, 저장 효과, 스키마 형태, 보안 보장, Core 권한 의미를 정의하지 않습니다.

## 먼저 볼 곳

- 제품/시스템 경계: [범위](scope.md), [Core 모델](core-model.md), [런타임 경계](runtime-boundaries.md), [보안](security.md).
- API 메서드 동작: [API 메서드](api/methods.md)에서 연결된 메서드 담당 문서.
- API 스키마 묶음: [API 코어 스키마](api/schema-core.md), [상태 스키마](api/schema-state.md), [아티팩트 스키마](api/schema-artifacts.md), [판단 스키마](api/schema-judgment.md), [값 집합](api/schema-value-sets.md).
- API 오류 묶음: [API 오류](api/errors.md). 오류 코드, 우선순위, 응답 처리 경로, 차단 사유 처리 경로, 기계 판독 세부사항으로 안내합니다.
- 저장소 묶음: [저장소](storage.md). 기록, DDL, 효과, 아티팩트, 버전 관리로 안내합니다.
- 접점, 상태 보기, 표시 경로: [에이전트 통합](agent-integration.md), [접점별 사용 레시피](../use/surface-recipes.md), [상태 보기와 템플릿](projection-and-templates.md), [템플릿 본문](template-bodies.md).
- 품질과 검증 경로: [적합성](conformance.md), [설계 품질](design-quality.md), 그리고 질문에 맞는 메서드 또는 Core 담당 문서.

## 자주 갈리는 경로

- 사용자 소유 판단의 의미는 [Core 모델](core-model.md)에, 요청과 기록 메서드 동작은 [사용자 소유 판단 요청 메서드](api/method-request-user-judgment.md)와 [사용자 소유 판단 기록 메서드](api/method-record-user-judgment.md)에, 판단 형태의 API 데이터는 [판단 스키마](api/schema-judgment.md)에 있습니다.
- 닫기 준비 상태 권한 개념은 [Core 모델](core-model.md)에, `harness.close_task` 동작은 [Task 닫기 메서드](api/method-close-task.md)에, `CloseReadinessBlocker` 형태는 [상태 스키마](api/schema-state.md)에, 차단 사유와 API 응답 사이의 경계 질문은 [API 차단 사유 처리 경로](api/blocker-routing.md)에 있습니다.
- 공개 오류 코드 의미는 [API 오류 코드](api/error-codes.md)에, 오류 우선순위는 [API 오류 우선순위](api/error-precedence.md)에, 응답 분기 처리 경로는 [API 오류 처리 경로](api/error-routing.md)에, 기계 판독용 오류 세부사항은 [API 오류 세부사항](api/error-details.md)에 있습니다.
- 용어 조회는 선별된 독자용 용어를 다루는 [용어집](glossary.md)에서 시작하고, 구조화 용어와 식별자 통제는 [`docs/terminology-map.yaml`](../../terminology-map.yaml)을 사용합니다.

## 유지보수 경로

- 저장소 편집 규칙: [`AGENTS.md`](../../../AGENTS.md).
- 문서 작성 규칙: [작성 가이드](../maintain/authoring-guide.md).
- 문서 점검: [점검](../maintain/checks.md).
- 영어/한국어 표현과 한국어 문체: [번역 가이드](../maintain/translation-guide.md).
