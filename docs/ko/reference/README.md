# 참조 색인

이 색인은 CLI, API, 저장소, 런타임, 보안, 용어, 품질, 범위 질문에 맞는 참조 문서를 고를 때 사용합니다. 정확한 제품 계약은 아래에 연결된 집중 참조 문서에 있습니다. 이 README는 독자를 해당 담당 문서로 안내하며 계약을 직접 정의하지 않습니다.

이 README는 경로 안내 전용입니다. 용어 뜻, 용어 메타데이터, API 동작, 오류 의미, 오류 우선순위, 응답 분기 처리 경로, 차단 사유 처리 경로, 저장 효과, 스키마 형태, 보안 보장, Core 권한 의미를 정의하지 않습니다.

## 먼저 볼 곳

- 설치 전 환경 전제 조건: [시스템 요구사항](system-requirements.md).
- 관리 호스트 행동 검증: [Agent Connection](agent-connection.md), 일반 릴리스 무결성: [검증](../maintain/validation.md).
- 실행 파일 준비와 검증 튜토리얼: [설치](../user-guide/installation.md).
- 제품/시스템 경계: [범위](scope.md), [Core 모델](core-model.md), [런타임 경계](runtime-boundaries.md), [보안](security.md).
- 외부 형식 호환성, 정확한 어댑터 선택, 공통 Git 객체 ID 검증: [외부 계약](external-contracts.md).
- 첫 에이전트 호스트 설정: 가장 짧은 성공 경로는 [빠른 시작](../user-guide/quickstart.md)에, 전체 운영자 가이드는 [에이전트 호스트 설정](../user-guide/agent-host-setup.md)에, 하나의 사용자 범위 Agent Connection이 여러 저장소를 처리하는 경로는 [다중 저장소 에이전트 설정](../user-guide/multi-repository-agent-setup.md)에 있습니다.
- 설정 실패와 복구: [에이전트 호스트 문제 해결](../user-guide/agent-host-troubleshooting.md).
- 로컬 실행 파일 계약: `volicord` 관리 명령과 Runtime Home 선택은 [관리 CLI](admin-cli.md), `volicord mcp preflight`, 수동 `volicord mcp serve`, 관리 시작, 응답 래핑, 종료는 [MCP 전송](mcp-transport.md)에 있습니다.
- API 메서드 동작: [API 메서드](api/methods.md)에서 연결된 메서드 담당 문서.
- API 스키마 묶음: [API 코어 스키마](api/schema-core.md), [상태 스키마](api/schema-state.md), [아티팩트 스키마](api/schema-artifacts.md), [사용자 행동 스키마](api/schema-user-action.md), [판단 스키마](api/schema-judgment.md), [값 집합](api/schema-value-sets.md).
- API 오류 묶음: [API 오류](api/errors.md). 오류 코드, 우선순위, 응답 처리 경로, 차단 사유 처리 경로, 기계 판독 세부사항으로 안내합니다.
- 제품 전체 실패 범주와 영속 데이터 실패 경계: [실패 모델](failure-model.md).
- 보수적인 기록 변경 억제 결과와 진단: [Guard 기록 변경 억제](guard-suppression.md).
- 저장소 묶음: [저장소](storage.md). 기록, DDL, 효과, 아티팩트, 버전 관리로 안내합니다.
- 연결, 상태 보기, 표시 경로: Agent Connection, Connection Projects, 현재 연결 맥락은 [Agent Connection 참조](agent-connection.md), User Channel과 런타임 위치 경계는 [런타임 경계](runtime-boundaries.md), 작업 범주 비보장은 [보안](security.md), 상태 보기는 [상태 보기와 템플릿](projection-and-templates.md), 렌더링 문구는 [템플릿 본문](template-bodies.md)에 있습니다.
- 품질과 검증 경로: [적합성](conformance.md), [설계 품질](design-quality.md), 행동 기반 호스트 관찰은 [Agent Connection](agent-connection.md), 그 밖에는 질문에 맞는 메서드 또는 Core 담당 문서.

## 자주 갈리는 경로

- 사용자 소유 행동과 판단의 의미는 [Core 모델](core-model.md)에, 요청과 해결 메서드 동작은 [사용자 행동 요청 메서드](api/method-request-user-action.md)와 [사용자 행동 해결 메서드](api/method-resolve-user-action.md)에 있습니다. 공통 요청, 해결, 상태, adapter-neutral resolution-form 형태는 [사용자 행동 스키마](api/schema-user-action.md)가, 정확한 CLI inbox schema는 [관리 CLI](admin-cli.md#user-channel-commands)가 담당하며 중첩된 선택 판단 payload는 [판단 스키마](api/schema-judgment.md)가 담당합니다.
- 닫기 준비 상태 권한 개념은 [Core 모델](core-model.md)에, `volicord.check_close`와 `volicord.close_task` 동작은 [닫기 메서드](api/method-close-task.md)에, `CloseReadinessBlocker` 형태는 [상태 스키마](api/schema-state.md)에, 차단 사유와 API 응답 사이의 경계 질문은 [API 차단 사유 처리 경로](api/blocker-routing.md)에 있습니다.
- 쓰기 티켓 의미와 대체 금지 규칙은 [Core 모델](core-model.md)에, 정책 적용과 Guard 후보 동작은 [관리 CLI](admin-cli.md)에 있습니다. 발급, 현재 정책 재평가, 재사용은 [쓰기 준비 메서드](api/method-prepare-write.md)가, 소비와 독립적인 현재 정책 검사는 [실행 기록 메서드](api/method-record-run.md)가 담당합니다. `write_authority_fingerprint` 필드와 범위는 [상태 스키마](api/schema-state.md)에, 영속 효과와 저장소 프로필 경계는 [저장 효과](storage-effects.md)와 [저장소 버전 관리](storage-versioning.md)에, 보안 비보장은 [보안](security.md)에 있습니다.
- 정확한 `.volicord/policy.json` object, 정책 명령, 기본값, 프로젝트/Connection binding 검사는 [관리 CLI](admin-cli.md#project-workflow-policy-commands)가 담당합니다. Core 정책 권한은 [Core 모델](core-model.md), 영속 저장은 [저장소 기록](storage-records.md)과 [저장 효과](storage-effects.md)가 담당합니다.
- 증거 캡처 권한은 [증거 캡처 준비 메서드](api/method-prepare-evidence-capture.md), 명령/도구 충족은 [관리 CLI](admin-cli.md#evidence-capture-fulfillment)가 담당합니다. 증거와 Run의 권한 의미는 [Core 모델](core-model.md)과 [실행 기록 메서드](api/method-record-run.md)에 남습니다.
- `doctor --privacy-footprint` 필드 집합과 개수 projection은 [관리 CLI](admin-cli.md#doctor-diagnostic-states), 저장 record 의미는 [저장소 기록](storage-records.md), 보장 한계는 [보안](security.md)이 담당합니다.
- 사용자 행동 inbox CLI 동작은 [관리 CLI](admin-cli.md)에, User Channel과 Agent Connection 경계는 [Agent Connection 참조](agent-connection.md)에, inbox item 형태는 [사용자 행동 스키마](api/schema-user-action.md)에 있습니다.
- 공개 오류 코드 의미는 [API 오류 코드](api/error-codes.md)에, 오류 우선순위는 [API 오류 우선순위](api/error-precedence.md)에, 응답 분기 처리 경로는 [API 오류 처리 경로](api/error-routing.md)에, 기계 판독용 오류 세부사항은 [API 오류 세부사항](api/error-details.md)에 있습니다.
- 공통 Git 객체 ID 검증과 canonicalization은 [외부 계약](external-contracts.md)이 담당합니다. 구조적 거부, 정책상 비허용, 사용 불가, 저하, 손상을 여러 표면에서 구분하는 의미는 [실패 모델](failure-model.md)이 담당하며, API 응답 표시는 계속 API 오류 담당 문서에 남습니다.
- 기록 변경 억제 결과, scan budget, fail-safe 경로와 reason 식별자는 [Guard 기록 변경 억제](guard-suppression.md)가 담당합니다.
- 관리용 `volicord` 명령은 로컬 부트스트랩 명령이며 공개 Volicord API 메서드가 아닙니다. `volicord mcp serve`는 별도의 두 번째 메서드 목록을 담당하지 않고 수동 MCP stdio를 통해 공개 메서드 집합을 노출합니다.
- 용어 조회는 선별된 독자용 용어를 다루는 [용어집](glossary.md)에서 시작하고, 구조화 용어와 식별자 통제는 [`docs/terminology-map.yaml`](../../terminology-map.yaml)을 사용합니다.

## 기여자 / 유지보수 경로

- 저장소 편집 규칙: [`AGENTS.md`](../../../AGENTS.md).
- 기계 판독 담당 메타데이터: [`docs/doc-index.yaml`](../../doc-index.yaml).
- 문서 거버넌스: [문서 정책](../maintain/documentation-policy.md).
- 문서 검증: [검증](../maintain/validation.md).
- 영어/한국어 표현과 한국어 문체: [번역 정책](../maintain/translation-policy.md).
