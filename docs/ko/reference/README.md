# 참조 문서 색인

이 색인은 "이 질문의 담당 문서는 어디인가?"에 답하기 위한 문서입니다. 이 README는 담당 문서로 안내할 뿐, API 계약, 스키마, 저장 효과, 보안 보장, 현재 MVP 범위를 정의하지 않습니다.

이 문서들은 향후 하네스 서버를 위한 원천 자료입니다. 이 저장소에 런타임 구현, 런타임 상태, 생성된 아티팩트, 상태 보기, 증거 기록, QA 기록, 수락 기록, 닫기 기록, 적합성 결과가 있다는 뜻이 아닙니다.

## 읽기 규칙

- 먼저 답해야 할 질문을 고르고, 그 질문에 맞는 담당 문서만 엽니다.
- 계약 세부사항은 담당 문서에 둡니다. 이 색인에 필드 목록, 응답 분기, DDL, 값 집합, 보장 수준을 길게 적고 싶어진다면 담당 문서로 옮기고 여기에는 경로만 남깁니다.
- 한영 문서나 용어 의미가 바뀌는 편집은 같은 작업 묶음에서 영어/한국어 담당 문서를 함께 맞춥니다.
- 번역이나 의미 일치 검토가 아니라면 같은 `doc_id`의 영어/한국어 대응 문서를 한 프롬프트에 함께 넣지 않습니다.
- 정확한 식별자는 백틱으로 보존하고, 의미는 담당 문서가 정하게 둡니다.

## 구현 담당자 읽기 경로

제품 경계에서 정확한 담당 문서로 들어갈 때는 아래 순서를 사용합니다.

| 단계 | 담당 문서 경로 |
|---|---|
| 현재 MVP 범위 | [현재 MVP 범위](active-mvp-scope.md) |
| API 메서드 | [MVP API](api/mvp-api.md) |
| 스키마 담당 문서 | [API 코어 스키마](api/schema-core.md), [API 상태 스키마](api/schema-state.md), [API 아티팩트 스키마](api/schema-artifacts.md), [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md), [API 오류](api/errors.md) |
| 저장 효과 | [저장 효과](storage-effects.md)를 먼저 보고, 필요한 경우 [저장소 기록](storage-records.md), [아티팩트 저장소](storage-artifacts.md), [저장소 버전 관리](storage-versioning.md)로 들어갑니다 |

이 경로는 정확한 담당 문서가 필요한 구현 담당자와 검토자를 위한 것입니다. 처음 읽는 사용자와 작업 중인 사용자는 [시작하기](../start.md)와 [사용자 가이드](../use/user-guide.md)에서 시작합니다.

## 현재 범위

| 질문 | 담당 문서 |
|---|---|
| 현재 MVP 범위는 어디에 정의되어 있나요? | [현재 MVP 범위](active-mvp-scope.md) |
| 현재 MVP에서 제외되는 범위는 어디에 정의되어 있나요? | [현재 MVP 범위](active-mvp-scope.md) |
| 어떤 기능이 현재 활성인지, 프로필로 제한되는지, 이후 후보인지 어디서 확인하나요? | [현재 MVP 범위](active-mvp-scope.md), [API 값 집합](api/schema-value-sets.md), [이후 후보 색인](../later/index.md) |
| 현재 MVP에서 `isolated`가 활성인가요? | [보안](security.md), [현재 MVP 범위](active-mvp-scope.md) |
| 이 저장소에서 런타임이나 서버 구현이 시작되었는지는 어디서 확인하나요? | [MVP 계획](../build/mvp-plan.md), [현재 MVP 범위](active-mvp-scope.md) |
| 문서 전용 경계는 어디에 있나요? | [현재 MVP 범위](active-mvp-scope.md), [런타임 경계](runtime-boundaries.md) |
| 구현 준비 상태나 유지보수자 인계 상태는 어디서 보나요? | [MVP 계획](../build/mvp-plan.md) |

## 담당 문서 찾기

| 질문 | 담당 문서 |
|---|---|
| Core 권한, Task 상태, 증거, 잔여 위험, 비대체 규칙은 어디가 담당하나요? | [Core 모델](core-model.md) |
| API 메서드 동작은 어디가 담당하나요? | [MVP API](api/mvp-api.md) |
| 공통 API 응답 분기와 요청 래퍼는 어디가 담당하나요? | [API 코어 스키마](api/schema-core.md) |
| 공개 오류 코드와 오류 우선순위는 어디가 담당하나요? | [API 오류](api/errors.md) |
| 저장소 기록이나 DDL은 어디가 담당하나요? | [저장소 기록](storage-records.md) |
| 저장 효과는 어느 담당 문서가 맡나요? | [저장 효과](storage-effects.md) |
| 메서드별 저장 효과는 어디가 담당하나요? | [저장 효과](storage-effects.md) |
| 저장 효과 질문은 어디로 가야 하나요? | [저장 효과](storage-effects.md) |
| 보안 주장과 비주장은 어디가 담당하나요? | [보안](security.md) |
| 제품 용어는 어디가 담당하나요? | [용어집](glossary.md), [docs/terminology-map.yaml](../../terminology-map.yaml) |
| 읽기 전용 상태 보기 권한과 원천/최신성 경계는 어디가 담당하나요? | [상태 보기 권한 참조](projection-and-templates.md) |
| 상태 카드, 판단 요청, 실행/증거 요약, 닫기 결과, 에이전트 맥락 패킷 본문은 어디가 담당하나요? | [템플릿 본문](template-bodies.md) |

## API와 스키마 담당 문서

| 질문 | 담당 문서 |
|---|---|
| `harness.prepare_write`는 무엇을 반환하나요? | [MVP API](api/mvp-api.md), [API 코어 스키마](api/schema-core.md), [API 상태 스키마](api/schema-state.md), [API 판단 스키마](api/schema-judgment.md), [Core 모델](core-model.md) |
| `ToolRejectedResponse`는 어디에 정의되어 있나요? | [API 코어 스키마](api/schema-core.md), [API 오류](api/errors.md) |
| `STATE_VERSION_CONFLICT`는 차단 사유 코드인가요? | [API 오류](api/errors.md) |
| `dry_run=true`인 `harness.close_task`가 언제 `ToolDryRunResponse`가 아닌 결과를 반환하나요? | [MVP API](api/mvp-api.md) |
| 활성 메서드 이름, `response_kind`, `effect_kind`, enum 형태 API 값은 어디가 담당하나요? | [API 값 집합](api/schema-value-sets.md) |
| `complete`가 enum 값인지, 이 문맥에서 "전체"라는 뜻인지 어디서 확인하나요? | [docs/terminology-map.yaml](../../terminology-map.yaml), [용어집](glossary.md), [API 값 집합](api/schema-value-sets.md) |
| 접근 등급은 어디에 정의되어 있나요? | [API 값 집합](api/schema-value-sets.md) |
| `DryRunSummary`, `PlannedEffect`, `PlannedBlocker` 같은 `dry_run` 미리보기 구조는 어디에 정의되어 있나요? | [API 코어 스키마](api/schema-core.md), [API 값 집합](api/schema-value-sets.md) |
| 보장 라벨 값의 담당 문서는 어디인가요? | [API 값 집합](api/schema-value-sets.md) |
| `isolated`는 값으로 어디에 정의되어 있나요? | [API 값 집합](api/schema-value-sets.md). 보장 의미는 [보안](security.md)을 봅니다 |
| `StateSummary`, `ShapingReadiness`, `NextActionSummary`, `CloseReadinessBlocker`, `ValidatorResult` 구조는 어디가 담당하나요? | [API 상태 스키마](api/schema-state.md) |
| `ArtifactRef`, `ArtifactInput`, `StagedArtifactHandle` 구조는 어디가 담당하나요? | [API 아티팩트 스키마](api/schema-artifacts.md) |
| `UserJudgment`, `SensitiveActionScope`, 수락된 위험 입력 구조는 어디가 담당하나요? | [API 판단 스키마](api/schema-judgment.md) |

## 저장소 담당 문서

| 질문 | 담당 문서 |
|---|---|
| 저장소 문서 묶음은 어디서 시작하나요? | [저장소](storage.md)를 먼저 보고, 아래의 구체적인 저장소 담당 문서로 이동합니다 |
| Runtime Home 배치, 로컬 저장소 가정, 테이블 개요는 어디가 담당하나요? | [저장소 기록](storage-records.md), [런타임 경계](runtime-boundaries.md) |
| `CloseReadinessBlocker`는 저장소 행인가요? | [저장소 기록](storage-records.md) |
| 아티팩트 스테이징은 증거를 만드나요? | [아티팩트 저장소](storage-artifacts.md), [저장 효과](storage-effects.md) |
| 아티팩트 승격은 어느 담당 문서가 맡나요? | [아티팩트 저장소](storage-artifacts.md) |
| 스테이징 핸들 검증과 아티팩트 본문 읽기 자격은 어디가 담당하나요? | [아티팩트 저장소](storage-artifacts.md), [API 아티팩트 스키마](api/schema-artifacts.md) |
| 멱등성, 상태 시계, 잠금, 마이그레이션은 어디가 담당하나요? | [저장소 버전 관리](storage-versioning.md), [API 오류](api/errors.md) |

## 보안과 런타임 담당 문서

| 질문 | 담당 문서 |
|---|---|
| 현재 MVP가 OS 수준 샌드박싱을 제공하나요? | [보안](security.md) |
| `isolated` 보장 의미의 담당 문서는 어디인가요? | [보안](security.md) |
| 보장 의미의 담당 문서는 어디인가요? | [보안](security.md) |
| Product Repository, Harness Server, Harness Runtime Home의 분리는 어디가 담당하나요? | [런타임 경계](runtime-boundaries.md) |
| 로컬 커넥터 동작, 역량 맥락, 확인된 접점 경계는 어디가 담당하나요? | [에이전트 통합](agent-integration.md), [MVP API](api/mvp-api.md), [보안](security.md) |
| CLI, IDE/editor, 채팅, 로컬 MCP 사용 레시피는 어디가 담당하나요? | [접점별 사용 레시피](../use/surface-recipes.md) |
| 보안 관련 공개 오류 경로는 어디가 담당하나요? | [API 오류](api/errors.md), [보안](security.md) |

## 사용자 판단과 닫기 준비 상태 담당 문서

| 질문 | 담당 문서 |
|---|---|
| 사용자 소유 판단과 비대체 규칙은 어디가 담당하나요? | [Core 모델](core-model.md), [API 판단 스키마](api/schema-judgment.md) |
| 민감 동작 승인 경계는 어디가 담당하나요? | [Core 모델](core-model.md), [API 판단 스키마](api/schema-judgment.md), [보안](security.md) |
| 닫기 준비 상태와 정직한 닫기 의미는 어디가 담당하나요? | [Core 모델](core-model.md), [MVP API](api/mvp-api.md), [API 오류](api/errors.md) |
| 닫기 차단 사유 구조와 닫기 오류 경로는 어디가 담당하나요? | [API 상태 스키마](api/schema-state.md), [API 오류](api/errors.md) |
| 최종 수락과 잔여 위험 수락 경계는 어디가 담당하나요? | [Core 모델](core-model.md), [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md) |
| 압축된 증거 요약의 의미는 어디가 담당하나요? | [Core 모델](core-model.md), [API 상태 스키마](api/schema-state.md), [MVP API](api/mvp-api.md) |

## 이후 후보와 유지보수 문서

| 질문 | 담당 문서 |
|---|---|
| 이후 후보는 어디에 문서화해야 하나요? | [이후 후보 색인](../later/index.md) |
| 보안과 보증 이후 후보의 담당 문서는 어디인가요? | [보안과 보증 이후 후보](../later/security-and-assurance.md) |
| 아티팩트와 증거 이후 후보의 담당 문서는 어디인가요? | [아티팩트와 증거 이후 후보](../later/artifacts-and-evidence.md) |
| 아티팩트 이후 후보는 어디에 문서화되어 있나요? | [아티팩트와 증거 이후 후보](../later/artifacts-and-evidence.md) |
| 커넥터와 접점 이후 후보의 담당 문서는 어디인가요? | [커넥터와 접점 이후 후보](../later/connectors-and-surfaces.md) |
| 정책과 적합성 이후 후보의 담당 문서는 어디인가요? | [정책과 적합성 이후 후보](../later/policy-and-conformance.md) |
| 작업 흐름과 협업 이후 후보의 담당 문서는 어디인가요? | [작업 흐름과 협업 이후 후보](../later/workflow-and-collaboration.md) |
| 이후 후보는 현재 요구사항인가요? | [이후 후보 색인](../later/index.md), [현재 MVP 범위](active-mvp-scope.md) |
| 승격 시점의 담당 문서 갱신은 무슨 뜻인가요? | [용어집](glossary.md), [이후 후보 색인](../later/index.md) |
| 이후 후보가 활성 기능이 되려면 어떤 문서가 더 바뀌어야 하나요? | [이후 후보 색인](../later/index.md), [현재 MVP 범위](active-mvp-scope.md) |
| "Complete close-readiness order"는 한국어로 어떻게 써야 하나요? | [용어집](glossary.md), [번역 가이드](../maintain/translation-guide.md), [API 값 집합](api/schema-value-sets.md) |
| "close readiness"를 한국어 참조 문서에서 "닫기 준비 상태"로 쓰는 기준은 어디가 담당하나요? | [docs/terminology-map.yaml](../../terminology-map.yaml), [용어집](glossary.md), [번역 가이드](../maintain/translation-guide.md) |
| 한국어 닫기 준비 상태 용어는 어떻게 써야 하나요? | [docs/terminology-map.yaml](../../terminology-map.yaml), [용어집](glossary.md), [번역 가이드](../maintain/translation-guide.md) |
| 닫기 준비 상태 한국어 용어는 어디서 통제하나요? | [docs/terminology-map.yaml](../../terminology-map.yaml), [용어집](glossary.md), [번역 가이드](../maintain/translation-guide.md) |
| 한국어 용어는 어디서 통제하나요? | [docs/terminology-map.yaml](../../terminology-map.yaml), [번역 가이드](../maintain/translation-guide.md), [용어집](glossary.md) |
| 문서 작성 규칙은 어디에 있나요? | [작성 가이드](../maintain/authoring-guide.md) |
| 큰 Markdown 표 작성 규칙은 어디에 있나요? | [작성 가이드](../maintain/authoring-guide.md), [문서 점검](../maintain/checks.md) |
| 문서 점검은 어디에 있나요? | [문서 점검](../maintain/checks.md) |
| 검색과 경로 메타데이터는 어디에서 관리하나요? | [docs/doc-index.yaml](../../doc-index.yaml) |
| 에이전트가 먼저 읽어야 할 문서는 무엇인가요? | [AGENTS.md](../../../AGENTS.md)를 먼저 읽고, 그다음 [docs/doc-index.yaml](../../doc-index.yaml)을 봅니다 |
