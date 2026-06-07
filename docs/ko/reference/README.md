# 참조 색인

참조 문서는 정확한 하네스 계획 계약의 담당 문서를 찾을 때 사용합니다. 향후 하네스 서버 검토를 위한 색인이며, 처음 읽는 튜토리얼이나 구현 계획이 아닙니다.

이 문서들은 현재 문서 검토 중인 향후 하네스 서버 계약을 설명합니다. 지금 이 저장소에 서버/런타임, Harness Runtime Home, 생성된 Projection 시스템, 적합성 실행기, 런타임 데이터, 구현 완료 동작이 있다는 뜻은 아닙니다.

## 읽기 규칙

- 참조 문서 전체를 기본으로 읽지 않기: 지금 질문에 맞는 담당 문서 하나를 고르고, 그 담당 문서가 더 엄격한 세부사항을 위임할 때만 링크를 따라갑니다.
- 같은 담당 문서의 영어/한국어 대응 문서를 같은 프롬프트에 함께 넣지 않습니다. 작업 언어를 하나 고르고, 이중 언어 비교는 별도의 작은 확인으로만 합니다.
- 이 README는 색인으로 유지합니다. 계약 세부사항을 여기로 복사하지 않습니다.
- active/later 경계는 활성 담당 문서와 [Later 후보 색인](../later/index.md)에 둡니다.

## 담당 문서 라우팅

아래 표는 에이전트와 구현자가 현재 존재하는 간결한 담당 문서 하나로 이동하도록 안내합니다.

| 계약 영역 | 담당 문서 |
|---|---|
| Core 권한, 작업 생명주기, 사용자 판단 경계, 최종 수락/잔여 위험 수락 대체 불가, gate, 닫기 차단 사유 | [core-model.md](core-model.md) |
| `prepare_write`가 `Write Authorization`에 미치는 효과를 포함한 활성 공개 API 메서드 | [api/mvp-api.md](api/mvp-api.md) |
| 공유 스키마, 봉투 구조, 활성 값 집합, 표시 라벨 경계, `GuaranteeDisplay.level` 값 | [api/schema-core.md](api/schema-core.md) |
| 공개 오류와 우선순위 | [api/errors.md](api/errors.md) |
| 저장소, DDL, `write_authorizations` 같은 지속 행, 멱등성 | [storage.md](storage.md) |
| 런타임 공간, 변경 권한, 비격리 / OS 샌드박싱 비보장 | [runtime-boundaries.md](runtime-boundaries.md) |
| 보안 보장, OS 샌드박싱 비보장, profile-gated `preventive` / `isolated` 표시 라벨 | [security.md](security.md) |
| 에이전트 맥락, connector 동작, 접점 기능, 하나의 `doc_id`에는 한 언어만 싣는 검색 규칙 | [agent-integration.md](agent-integration.md) |
| 파생 표시인 Projection/상태 카드, 렌더링된 라벨, 활성 템플릿 | [projection-and-templates.md](projection-and-templates.md) |
| 적합성 모델, 향후 fixture 형식, 주장 권한, 실행 가능한 모음이 아니라는 경계 | [conformance.md](conformance.md) |
| 활성 gate에 영향을 주는 설계 품질 규칙 | [design-quality.md](design-quality.md) |
| 공식 용어 | [glossary.md](glossary.md) |
| 전체 형식 판단 표시와 향후 fixture 계열을 포함한 later 후보 | [../later/index.md](../later/index.md) |

## 중복 주입 금지

담당 문서가 아닌 문서는 독자에게 보이는 결과만 요약하고 담당 문서로 연결합니다. 스키마, DDL, enum 표, 전이 표, 상태 보기 템플릿 본문, fixture assertion, 공개 오류 우선순위, 보안 보장, 용어 정의를 붙여 넣지 않습니다.

문서 작성, 번역, 검토, 링크 정리, 담당 문서 경계 불일치, 문서 유지보수 점검은 [작성 가이드](../maintain/authoring-guide.md), [번역 가이드](../maintain/translation-guide.md), [문서 점검](../maintain/checks.md)이 담당합니다. 구현 순서와 유지보수자 상태 결정은 [MVP 계획](../build/mvp-plan.md)이 담당합니다.
