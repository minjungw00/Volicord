# 하네스 문서

이 문서는 하네스 한국어 문서 세트의 길잡이입니다.

하네스는 AI 지원 제품 작업을 위한 향후 로컬 작업 권한 서버입니다. 하네스의 권한은 범위, 사용자 소유 판단, 증거, 확인과 검증 기대, 최종 수락, 닫기 가능 여부, 잔여 위험에 대한 하네스 기록과 상태 전이를 대상으로 합니다. 대화의 쉽게 흔들리는 맥락이 그 기록들의 기준처럼 굳어지지 않게 하는 것이 목표입니다.

이 권한은 운영체제 권한 제어, 임의 도구 샌드박스, 변조 방지 저장소, 기본 도구 실행 전 차단, 보안 격리를 뜻하지 않습니다. 구체적인 향후 프로필 메커니즘이 이름 붙고 증명되지 않은 한 MVP-1 표현은 cooperative에 제한된 detective behavior를 더한 수준으로 읽어야 합니다.

이 저장소는 현재 문서 전용이며 재설계 이후 검토 상태입니다. 문서 수락과 별도의 구현 계획 준비 결정이 끝난 뒤에만 하네스 서버 소스 저장소가 될 예정입니다. 사용자의 제품 저장소도, 하네스 런타임 홈도, 실행 중인 하네스 인스턴스도 아닙니다.

아직 서버/런타임 구현, 런타임 상태, 생성된 읽기용 보기 시스템, conformance runner, 생성된 운영 산출물, 실행 가능한 fixture, 제품 구현 코드는 없습니다. 문서 파일은 원천 자료입니다. 하네스 런타임 상태, 증거, QA, 수락, 잔여 위험, 읽기용 보기, 닫기 기록이 아닙니다.

## 최소 첫 읽기 경로

어디서 시작해야 할지 모를 때는 이 순서로 읽습니다.

1. [시작하기](start.md)에서 첫 이해 모델, 평소 작업 하나, 최소 개념, 현재 보장 경계를 봅니다.
2. [사용자 가이드](use/user-guide.md)에서 실제 사용자와 에이전트 상호작용을 봅니다.
3. 향후 하네스 서버 구현을 검토할 때만 [구현 개요](build/implementation-overview.md)를 봅니다.
4. 정확한 계약이 필요할 때만 [Reference 색인](reference/README.md)을 봅니다.

이 경로는 일부러 큰 Reference 문서 앞에서 멈춥니다. 처음 읽는 독자는 하네스의 목적을 이해하기 위해 schema, DDL, 전이 표, fixture body, threat catalog부터 볼 필요가 없습니다.

## 독자별 경로

| 독자 | 먼저 읽기 | 이어서 보기 |
|---|---|---|
| 일반 사용자 | [시작하기](start.md) | 실제 세션 동작은 [사용자 가이드](use/user-guide.md). |
| 에이전트 지침 작성자 | [에이전트 가이드](use/agent-guide.md) | 정확한 connector 또는 context 동작이 필요할 때만 [Agent 통합 참조](reference/agent-integration.md)와 [Surface Cookbook](reference/surface-cookbook.md). |
| 향후 서버 구현자 | [구현 개요](build/implementation-overview.md) | 첫 내부 증명은 [내부 엔지니어링 점검](build/engineering-checkpoint.md), 첫 사용자 가치 조각은 [MVP-1 사용자 작업 루프](build/mvp-user-work-loop.md), 정확한 owner는 [Reference 색인](reference/README.md). |
| 정확한 계약을 찾는 독자 | [Reference 색인](reference/README.md) | Reference 전체를 읽기보다 필요한 계약의 담당 문서를 고릅니다. |
| 문서 유지보수자 | [문서 작성 가이드](maintain/authoring-guide.md) | [번역 가이드](maintain/translation-guide.md), [문서 점검표](maintain/documentation-checks.md), [재작성 계획](maintain/rewrite-plan.md), [재작성 수락 리뷰](maintain/rewrite-acceptance-review.md). |
| 이후 프로필 독자 | [보증 프로필](later/assurance-profile.md) | [운영 프로필](later/operations-profile.md), [향후 Fixtures](later/future-fixtures.md), [로드맵](roadmap.md). 담당 문서가 승격하기 전까지 MVP 경로 밖에 둡니다. |

## 문서층 역할

| 문서군 | 역할 | 경계 |
|---|---|---|
| Start | 하네스가 왜 필요한지, 권한이 어디에 있는지, 평소 작업 하나, 첫 개념, 현재 보장 경계를 설명합니다. | schema, gate, DDL, 구현 순서, fixture mechanics를 정의하지 않습니다. |
| Use | 평소 말 예시, 에이전트 행동, 판단 요청 처리, 쓰기 전 확인, 증거 요약, 닫기 흐름을 통해 사용자와 에이전트 사용 방식을 설명합니다. | canonical enum, DDL, 전체 전이 표를 정의하지 않습니다. |
| Build | 향후 구현 순서, 활성 조각, 첫 증명, 활성/이후 경계, 구현자 읽기 경로, 제외 범위를 설명합니다. | 정확한 API shape, schema, DDL, storage table, 상태 전이, fixture body, security guarantee, threat catalog는 Reference로 연결합니다. |
| Reference | Core 전이, API schema, Storage/DDL, Security, Agent Integration, Projection/Templates, Conformance, Glossary, runtime architecture, operations, design-quality policy의 정확한 계약을 담당합니다. | 첫 읽기 튜토리얼이나 단계별 구현 계획이 아닙니다. |
| Later | 활성 MVP 경로 밖의 향후/profile 자료를 둡니다. | 담당 문서가 범위와 증명 기대를 함께 승격하기 전까지 활성 전달 범위가 아닙니다. |
| Maintain | 문서 작성, 번역, 검토, drift, owner 경계, link 규칙을 관리합니다. | runtime readiness, 최종 수락, close readiness, implementation readiness를 결정하지 않습니다. |

## 구현 문서 경로

Build 문서는 문서 수락과 별도의 구현 계획 준비 결정 이후의 향후 구현 방향을 잡기 위한 문서입니다. 순서와 단계 경계를 설명하며, 정확한 API, schema, storage, fixture, security contract는 Reference에 남깁니다. Build 문서는 서버/런타임 구현을 승인하지 않습니다.

권장 순서:

1. [구현 개요](build/implementation-overview.md): 현재 상태, 인계, 준비 조건, 읽기 경로.
2. [내부 엔지니어링 점검](build/engineering-checkpoint.md): 제품 MVP가 아닌 첫 내부 Core 권한 루프 증명.
3. [MVP-1 사용자 작업 루프](build/mvp-user-work-loop.md): 첫 사용자 가치 구현 계획과 중앙 서버 코딩 전 결정 기록.
4. [런타임 설계 흐름](build/runtime-walkthrough.md): 의도한 request-to-close 설계 경로. 런타임 존재의 증거가 아닙니다.
5. [Reference 색인](reference/README.md): 정확한 계약 담당 문서.

## 사용 문서 경로

Use 문서는 사용자와 에이전트가 신뢰 경계에서 실제로 보게 되는 동작에 머뭅니다.

- [사용자 가이드](use/user-guide.md)는 기본 사용자 진입점입니다.
- [에이전트 가이드](use/agent-guide.md)는 에이전트 행동 지침입니다.
- [사용자 소유 판단 예시](use/judgment-examples.md)는 실용적인 판단 요청 예시를 제공합니다. 전체 형식 Decision Packet 표시를 active 사용자 경로처럼 만들지 않습니다.

정확한 사용자 판단, 쓰기, 실행/증거, 닫기, 읽기용 보기, error 계약은 [Reference 색인](reference/README.md)에서 연결하는 Reference 담당 문서가 맡습니다.

## Reference 경로

정확한 계약이 필요할 때 [Reference 색인](reference/README.md)을 사용합니다. Core 상태 전이, API schema, Storage/DDL, Security, Agent Integration, Projection/Templates, Conformance, Glossary, runtime architecture, operations, design-quality policy의 담당 문서 지도를 제공합니다.

Reference 표를 Start, Use, Build, Maintain 문서로 복사하지 않습니다. Owner가 아닌 문서는 독자에게 보이는 결과만 짧게 요약하고 담당 문서로 연결합니다.

## 유지보수 경로

Maintain 문서는 문서 작업에만 사용합니다.

- [문서 작성 가이드](maintain/authoring-guide.md)
- [번역 가이드](maintain/translation-guide.md)
- [문서 점검표](maintain/documentation-checks.md)
- [재작성 계획](maintain/rewrite-plan.md)
- [재작성 수락 리뷰](maintain/rewrite-acceptance-review.md)

Docs-maintenance 점검은 Markdown 품질을 보는 읽기 전용 점검입니다. `PASS`, `WARN`, `FAIL` label은 runtime conformance, 최종 수락, close readiness, implementation readiness를 만들지 않습니다.

## 상태 담당 문서

현재 인계 상태는 [구현 개요: 문서 인계 요약](build/implementation-overview.md#문서-인계-요약)이 담당합니다. 문서 수락 상태는 [구현 개요: 문서 수락 상태](build/implementation-overview.md#문서-수락-상태)에 있습니다. 서버 코딩 전 결정은 [MVP-1 사용자 작업 루프: 서버 코딩 전 필요한 구현 결정](build/mvp-user-work-loop.md#서버-코딩-전-필요한-구현-결정)에 둡니다.

문서 수락은 유지보수자 검토 이정표입니다. 그것만으로 런타임/서버 구현이 시작되거나 런타임 conformance가 증명되지 않습니다.

## 언어 의미 일치

영어 문서와 한국어 문서는 같은 active file map과 의미를 유지합니다. 한국어 문서는 영어 문장을 한 줄씩 옮기기보다 자연스러운 한국어 제목과 흐름을 사용할 수 있습니다.
