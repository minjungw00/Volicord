# 참조 색인

이 색인은 Volicord 참조 질문에서 다음 담당 문서를 고를 때 쓰는 읽기용 색인입니다. 기계가 읽는 정확한 담당 경로는 [`docs/doc-index.yaml`](../../doc-index.yaml)을 사용합니다. 이 파일이 `doc_id`, 유지 경로, 문서 종류, 집중 `canonical_for` 범위, 유지보수 `owner_area`, `created_on`, `last_updated_on`, `last_verified_on`, `applies_to`, 의존 관계, 규범 수준, 주요 독자, 독자 여정, 번역 정책 메타데이터를 담당합니다.

이 README는 경로 안내 전용입니다. 용어 뜻, 용어 메타데이터, API 동작, 오류 의미, 오류 우선순위, 응답 분기 처리 경로, 차단 사유 처리 경로, 저장 효과, 스키마 형태, 보안 보장, Core 권한 의미를 정의하지 않습니다.

## 먼저 볼 곳

- 설치 전 환경 전제 조건: [시스템 요구사항](system-requirements.md).
- 실행 파일 준비와 검증 튜토리얼: [설치](../getting-started/installation.md).
- 제품/시스템 경계: [범위](scope.md), [Core 모델](core-model.md), [런타임 경계](runtime-boundaries.md), [보안](security.md).
- 첫 에이전트 호스트 설정: 가장 짧은 성공 경로는 [빠른 시작](../getting-started/quickstart.md)에, 전체 운영자 가이드는 [에이전트 호스트 설정](../guides/agent-host-setup.md)에, 하나의 사용자 범위 통합이 여러 저장소를 처리하는 경로는 [다중 저장소 에이전트 설정](../guides/multi-repository-agent-setup.md)에 있습니다.
- 설정 실패와 복구: [에이전트 호스트 문제 해결](../guides/agent-host-troubleshooting.md).
- 로컬 실행 파일 계약: `volicord` 관리 명령과 Runtime Home 선택은 [관리 CLI](admin-cli.md), `volicord-mcp` stdio 시작, 사전 점검, 응답 래핑, 종료는 [MCP 전송](mcp-transport.md)에 있습니다.
- API 메서드 동작: [API 메서드](api/methods.md)에서 연결된 메서드 담당 문서.
- API 스키마 묶음: [API 코어 스키마](api/schema-core.md), [상태 스키마](api/schema-state.md), [아티팩트 스키마](api/schema-artifacts.md), [판단 스키마](api/schema-judgment.md), [값 집합](api/schema-value-sets.md).
- API 오류 묶음: [API 오류](api/errors.md). 오류 코드, 우선순위, 응답 처리 경로, 차단 사유 처리 경로, 기계 판독 세부사항으로 안내합니다.
- 저장소 묶음: [저장소](storage.md). 기록, DDL, 효과, 아티팩트, 버전 관리로 안내합니다.
- 연결, 상태 보기, 표시 경로: Agent Connection과 User Channel 경계는 [런타임 경계](runtime-boundaries.md), 작업 범주 비보장은 [보안](security.md), 상태 보기는 [상태 보기와 템플릿](projection-and-templates.md), 렌더링 문구는 [템플릿 본문](template-bodies.md)에 있습니다.
- 품질과 검증 경로: [적합성](conformance.md), [설계 품질](design-quality.md), 그리고 질문에 맞는 메서드 또는 Core 담당 문서.

## 자주 갈리는 경로

- 사용자 소유 판단의 의미는 [Core 모델](core-model.md)에, 요청과 기록 메서드 동작은 [사용자 소유 판단 요청 메서드](api/method-request-user-judgment.md)와 [사용자 소유 판단 기록 메서드](api/method-record-user-judgment.md)에, 판단 형태의 API 데이터는 [판단 스키마](api/schema-judgment.md)에 있습니다.
- 닫기 준비 상태 권한 개념은 [Core 모델](core-model.md)에, `volicord.close_task` 동작은 [Task 닫기 메서드](api/method-close-task.md)에, `CloseReadinessBlocker` 형태는 [상태 스키마](api/schema-state.md)에, 차단 사유와 API 응답 사이의 경계 질문은 [API 차단 사유 처리 경로](api/blocker-routing.md)에 있습니다.
- 공개 오류 코드 의미는 [API 오류 코드](api/error-codes.md)에, 오류 우선순위는 [API 오류 우선순위](api/error-precedence.md)에, 응답 분기 처리 경로는 [API 오류 처리 경로](api/error-routing.md)에, 기계 판독용 오류 세부사항은 [API 오류 세부사항](api/error-details.md)에 있습니다.
- 관리용 `volicord` 명령은 로컬 부트스트랩 명령이며 공개 Volicord API 메서드가 아닙니다. `volicord-mcp`는 별도의 두 번째 메서드 목록을 담당하지 않고 MCP stdio를 통해 공개 메서드 집합을 노출합니다.
- 용어 조회는 선별된 독자용 용어를 다루는 [용어집](glossary.md)에서 시작하고, 구조화 용어와 식별자 통제는 [`docs/terminology-map.yaml`](../../terminology-map.yaml)을 사용합니다.

## 유지보수 경로

- 저장소 편집 규칙: [`AGENTS.md`](../../../AGENTS.md).
- 문서 거버넌스: [문서 정책](../maintain/documentation-policy.md).
- 문서 검증: [검증](../maintain/validation.md).
- 영어/한국어 표현과 한국어 문체: [번역 정책](../maintain/translation-policy.md).
