# 하네스 문서

이 문서는 하네스 한국어 문서 세트의 길잡이입니다.

이 저장소는 현재 문서 전용 재설계/검토 저장소입니다. 향후 역할은 하네스 서버 소스 저장소입니다. 아직 이곳에는 하네스 서버/런타임 구현이 없습니다. 서버/런타임 구현을 시작하려면 문서 수락과 별도의 구현 계획 준비 결정이 모두 필요합니다. 이 문서들은 하네스를 이해하고 구현하기 위한 원천 자료이며, 문서가 설명하는 생명주기를 따라 관리되는 하네스 런타임 객체가 아닙니다.

이 문서는 한국어 독자를 위한 상세 저장소 상태를 담습니다. Learn/Use 문서는 상태 설명을 짧게 유지하고, 사용자가 무엇을 요청할 수 있는지, 에이전트가 무엇을 구체화해야 하는지, 하네스가 무엇을 보존하는지, 사용자가 무엇을 보게 되는지부터 시작해야 합니다.

하네스는 AI 지원 제품 작업에서 작업 범위, 사용자 판단, 근거, 검증, QA 기대, 작업 수락, 잔여 위험 상태를 깨지기 쉬운 대화 맥락 밖에 두는 로컬 기준 기록이자 판단 경로입니다.

하네스가 집중하는 문제는 네 가지입니다.

- 작업 범위가 흐르거나 암묵적으로 바뀝니다.
- 사용자 판단이 조용히 에이전트 판단으로 바뀝니다.
- 근거, 검증, QA, 완료 주장이 뒤섞입니다.
- 대화나 Markdown 출력이 운영상 기준으로 오해됩니다.

## 현재 상태 모델

현재 상태는 문서 검토 상태, 구현 계획 준비 상태, 런타임 구현 상태를 분리해서 읽어야 합니다.

| 상태 범주 | 현재 상태 |
|---|---|
| 문서 검토 상태 | 재설계 이후 검토 상태이며 문서 수락 후보입니다. 유지보수자가 아직 문서를 수락하지 않았습니다. |
| 구현 계획 준비 상태 | 아직 수락되지 않았습니다. 첫 런타임 배치 계획 전에 유지보수자가 구현 준비 조건을 확인해야 합니다. 편집 정리와 스키마/설계 결정, 단계 경계 결정은 서로 다른 항목입니다. |
| 런타임 구현 상태 | 시작하지 않았습니다. 아직 런타임 아티팩트나 conformance 결과가 없습니다. |

문서 수락은 유지보수자 검토 이정표입니다. 문서가 수락되더라도 그것만으로 런타임/서버 구현이 시작되거나 런타임 conformance가 증명되지는 않습니다.

## 단계 분류

| 단계 | 의미 |
|---|---|
| 코어 권한 조각(v0.1 Core Authority Slice) | 가장 작은 권한 경로를 증명하는 첫 내부 Core 권한 루프입니다. 제품 MVP가 아닙니다. |
| 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP) | 평범한 요청에서 범위 보존, 판단 라우팅, 근거, 닫기 준비 상태, 작업 수락, 잔여 위험 경계를 통해 사용자가 하네스의 핵심 가치를 처음 체감하는 제품 MVP입니다. |
| 에이전시 보증 팩(v0.3 Agency Assurance Pack) | 검증, QA, 잔여 위험, 작업 수락, stewardship 동작을 단단하게 만듭니다. |
| 운영과 인계 팩(v0.4 Operations & Handoff Pack) | 복구/export, release handoff, artifact integrity, 더 넓은 conformance coverage, operator behavior를 추가합니다. |
| v1+ Expansion | Dashboard, hosted UI, browser capture 자동화, team workflow, 그 밖의 후보는 승격되기 전까지 staged delivery 밖에 남습니다. |

## 주요 읽기 경로

어디서 시작해야 할지 모를 때는 이 순서로 읽습니다.

1. [개요](learn/overview.md)에서 첫 번째 이해 모델을 잡습니다.
2. [사용자 가이드](use/user-guide.md)에서 작업 중 하네스와 상호작용하는 방법을 봅니다.
3. [핵심 개념](learn/concepts.md)에서 예시, 상태, 사양에 나오는 어휘를 정리합니다.
4. 서버 구현을 검토하거나 계획할 때 [구현 개요](build/implementation-overview.md)와 [MVP 계획](build/mvp-plan.md)을 봅니다.
5. 정확한 계약, 스키마, gate, storage, 읽기용 요약(Projection), 보안, 템플릿이 필요할 때만 [참조 문서](#참조-문서)를 봅니다.

## 독자별 경로

| 독자 | 먼저 읽기 | 이어서 보기 |
|---|---|---|
| 사용자 | [개요](learn/overview.md) | [사용자 가이드](use/user-guide.md), [핵심 개념](learn/concepts.md), 결정이 복잡해질 때 [결정 패킷 Cookbook](use/decision-packet-cookbook.md). |
| 에이전트 통합자 | [개요](learn/overview.md) | [사용자 가이드](use/user-guide.md), [에이전트 세션 흐름](use/agent-session-flow.md), [에이전트 통합 참조](reference/agent-integration.md), [Surface Cookbook](reference/surface-cookbook.md), [MCP API와 스키마](reference/mcp-api-and-schemas.md). |
| 구현자 | [개요](learn/overview.md) | [핵심 개념](learn/concepts.md), [구현 개요의 문서 수락 후보 요약](build/implementation-overview.md#문서-수락-후보-요약), [MVP 계획의 구현 시작 전 결정](build/mvp-plan.md#서버-코딩-전-필요한-구현-결정), [첫 실행 가능한 조각](build/first-runnable-slice.md), [MVP 계획](build/mvp-plan.md), [Runtime Walkthrough](build/runtime-walkthrough.md), 관련 기준 문서 소유자. |
| 검토자 / 문서 유지보수자 | [개요](learn/overview.md) | [문서 작성 가이드](maintain/authoring-guide.md), [번역 가이드](maintain/translation-guide.md), [로드맵](roadmap.md), 엄격한 의미를 확인할 때 관련 기준 문서 소유자. |

운영자와 conformance 작성자는 보통 Reference에서 시작합니다. [운영과 Conformance 참조](reference/operations-and-conformance.md), [Conformance Fixtures 참조](reference/conformance-fixtures.md), [런타임 아키텍처 참조](reference/runtime-architecture.md), [보안 위협 모델 참조](reference/security-threat-model.md), [MCP API와 스키마](reference/mcp-api-and-schemas.md), [Storage와 DDL](reference/storage-and-ddl.md), [커널 참조](reference/kernel.md)를 사용합니다.

## 문서별 역할

Learn과 Use 문서는 삭제하지 않고 역할을 좁혀 둡니다.

| 문서 | 역할 |
|---|---|
| [개요](learn/overview.md) | 가장 먼저 읽는 문서입니다. 제품 명제, 세 공간, 하네스가 기록하는 것, 하네스가 아닌 것을 설명합니다. |
| [목적과 원칙](learn/purpose-and-principles.md) | 가치, 비목표, 실패 모델, MVP 경계를 담습니다. 문구나 범위가 제품 명제와 맞는지 검토할 때 씁니다. |
| [핵심 개념](learn/concepts.md) | 평소 말에서 구현 용어로 넘어가는 어휘 다리입니다. 또 하나의 개요나 튜토리얼이 아닙니다. |
| [15분 만에 보는 하네스](learn/harness-in-15-minutes.md) | 짧은 시나리오 모음입니다. 엄격한 사양 전에 흔히 만나는 하네스 순간을 빠르게 보여 줍니다. |
| [하나의 작업으로 보는 하네스](learn/harness-in-one-task.md) | 튜토리얼입니다. 작은 변경 하나와 추적되는 작업 하나로 전체 작업 흐름을 보여 줍니다. |
| [사용자 가이드](use/user-guide.md) | 작업을 시작하고, 이어가고, 막힘을 풀고, 닫는 기본 사용자 문서입니다. |
| [결정 패킷 Cookbook](use/decision-packet-cookbook.md) | 초점 있는 사용자 결정 요청을 만들기 위한 고급 사용 예시이자 Reference 인접 예시입니다. |
| [에이전트 세션 흐름](use/agent-session-flow.md) | 상태 표시, 맥락, blocker, 쓰기, 닫기를 다루는 에이전트/통합 지침입니다. 일반 사용자가 반드시 읽어야 하는 문서는 아닙니다. |

## 지금 보는 저장소

하네스는 세 공간을 분리합니다.

| 공간 | 들어가는 것 |
|---|---|
| 제품 저장소 | 사용자의 제품 작업 공간입니다. 제품 코드, 테스트, 제품 문서, 사람이 읽는 하네스 읽기용 요약이 여기에 속합니다. |
| 하네스 서버 소스 저장소 | 로컬 하네스 서버/설치 프로그램의 미래 코드베이스입니다. API 표면, 요청 검증, Core 상태 전이, 검증기, 읽기용 요약(Projection), reconcile, 운영자 도구는 문서 수락과 별도의 구현 계획 준비 결정 이후에만 여기에 구현될 수 있습니다. |
| 하네스 런타임 홈 | 사용자별/설치별 운영 데이터 공간입니다. 상태 데이터베이스, 아티팩트 저장소, 읽기용 요약(Projection) 출력, 로그, 로컬 등록/설정 정보가 여기에 속합니다. |

이 저장소의 현재 역할은 문서 검토와 재설계입니다. 향후 역할은 하네스 서버 소스 저장소입니다. 제품 저장소도 하네스 런타임 홈도 아닙니다. 문서 수락만으로 구현 권한, 런타임 상태, conformance, 서버 코드가 생기지 않습니다. 첫 구현 계획 준비 상태가 별도로 수락된 뒤에야 하네스 서버/설치 프로그램 코드 작업을 이 저장소에서 시작할 수 있습니다.

## 문서 재설계 범위

문서 검토 상태, 구현 계획 준비 상태, 런타임 구현 상태는 [구현 개요](build/implementation-overview.md#문서-승인-상태)에서 확인합니다. 현재 리비전은 재설계 이후 검토 중인 문서 수락 후보이지 구현 시작 승인이 아닙니다.

이번 재설계에서는 용어, MVP 단계, 스키마 구조, 읽기용 요약(Projection) 구조, 보안 표현, 문서 구성이 바뀔 수 있습니다. 정리된 제품 명제와 구현 가능한 경로를 우선하며, 기존 문구는 연속성만으로 보존하지 않습니다.

전체 재설계 범위, 보존 원칙, 문서군별 역할, 유지보수자 검토 점검 목록은 [문서 작성 가이드](maintain/authoring-guide.md#현재-재설계-범위)가 담당합니다. 검토 tracker는 현재 문서에서 확인된 drift, 확인 대상 후보, 회귀 방지 점검, 기준 상태 점검을 구분하고, 확인된 finding을 문서 drift, 스키마/설계 결정, 단계 경계 결정, 구현 준비 조건, 향후 로드맵 항목으로 라우팅합니다.

## 문서 수락 후보

하네스 서버 코드를 시작하기 전 구현자는 다음을 읽어야 합니다.

1. [문서 수락 후보 요약](build/implementation-overview.md#문서-수락-후보-요약): 현재 단계, 보존 원칙, 단계 모델, 정리된 경계, 남은 질문 상태를 확인합니다.
2. [문서 수락 상태](build/implementation-overview.md#문서-승인-상태): 세 상태 모델과 유지보수자가 첫 런타임 배치 계획을 수락했는지 확인합니다.
3. [하네스 서버 구현 준비 조건](build/implementation-overview.md#하네스-서버-구현-준비-조건): 구현 계획 준비 상태를 바꾸기 전에 참이어야 하는 점검 항목을 확인합니다.
4. [서버 코딩 전 필요한 구현 결정](build/mvp-plan.md#서버-코딩-전-필요한-구현-결정): 현재 검토에서 드러나는 주요 결정을 확인합니다. 이 기준에서 결정 기록은 비어 있지만, 남은 결정이 없다는 뜻은 아닙니다.

이 인계는 문서가 유지보수자 수락 검토 대상 후보로 올라와 있다는 뜻입니다. 문서가 이미 수락되었거나, 구현 준비가 끝났거나, 서버/런타임 구현이 시작되었다는 뜻이 아닙니다.

## 하네스가 아닌 것

하네스는 agent instruction, MCP, reusable workflow, 테스트, 리뷰, spec과 같은 역할을 하지 않습니다. 이런 요소들은 하네스 주변에서 유용할 수 있지만, 로컬 운영 기록이나 사용자 판단의 주인이 되지는 않습니다.

하네스는 prompt 묶음, 대화 스크립트, evaluation harness, dashboard, 넓은 hosted agent platform도 아닙니다.

## 비교

| 인접 개념 | 그 역할 | 하네스의 역할 |
|---|---|---|
| AGENTS.md / agent instruction 파일 | 저장소나 세션에서 에이전트가 어떻게 행동해야 하는지 알려 줍니다. | 하네스는 그런 지침을 사용할 수 있지만, 범위, 사용자 소유 판단, 근거, 닫기 준비 상태, 잔여 위험을 로컬 기록으로 유지합니다. |
| MCP | 도구와 리소스를 연결하는 프로토콜 경계입니다. | 하네스는 MCP 도구나 리소스를 노출할 수 있지만, 하네스의 기준은 Core가 소유한 로컬 상태와 아티팩트 참조에서 나옵니다. |
| Skill / reusable workflow | 에이전트가 반복해서 따를 수 있는 지침이나 절차를 묶습니다. | 하네스는 그런 workflow 안에서 사용될 수 있지만, 지금 진행 중인 작업 상태를 기록하고 이 작업의 판단을 정해진 경로로 보냅니다. |
| Test runner | 검사를 실행하고 결과를 냅니다. | 하네스는 관련 결과를 근거로 연결하고, 검증의 강도와 작업 수락을 따로 둡니다. |
| Code review | 변경을 사람이 또는 팀이 검토합니다. | 하네스는 리뷰 결과를 참조할 수 있지만, 리뷰를 대체하거나 리뷰를 작업 수락 또는 잔여 위험 수용으로 바꾸지 않습니다. |
| Spec | 의도한 동작, 설계, 제약을 설명합니다. | 하네스는 spec을 입력으로 사용할 수 있지만, 실제 작업의 운영 상태인 범위, 결정, 근거, QA 기대, 작업 수락, 잔여 위험을 기록합니다. |

## 소유권 규칙

정확한 계약은 Reference 문서가 담당합니다. 스키마, DDL, 관문(gate), 상태 전이, enum value, fixture 의미, template 본문, 공식 정의가 여기에 속합니다. Learn, Use, Build 문서는 독자에게 필요한 생각을 설명하고 Reference로 연결하며, 엄격한 계약 블록을 복사하지 않습니다.

Docs-maintenance check는 drift, 소유자 경계, link, 언어 의미 일치를 살피는 편집 품질 점검입니다. Runtime conformance나 implementation readiness가 아닙니다. Drift category와 owner-first resolution은 [문서 작성 가이드](maintain/authoring-guide.md#docs-maintenance-checks)를 사용하고, docs-maintenance profile reporting boundary는 [운영과 Conformance](reference/operations-and-conformance.md#docs-maintenance-프로필)를 사용합니다.

## 학습 문서

정확한 계약에 들어가기 전에 전체 그림을 잡는 경로입니다.

- [개요](learn/overview.md)
- [목적과 원칙](learn/purpose-and-principles.md)
- [핵심 개념](learn/concepts.md)
- [15분 만에 보는 하네스](learn/harness-in-15-minutes.md)
- [하나의 작업으로 보는 하네스](learn/harness-in-one-task.md)

## 사용 문서

AI 지원 개발 세션을 하네스 기준으로 진행할 때 보는 경로입니다. 기본 사용자 문서는 [사용자 가이드](use/user-guide.md)입니다. [결정 패킷 Cookbook](use/decision-packet-cookbook.md)은 고급 결정 예시이며, [에이전트 세션 흐름](use/agent-session-flow.md)은 일반 사용자의 필수 읽기가 아니라 에이전트/통합 지침입니다.

- [사용자 가이드](use/user-guide.md)
- [결정 패킷 Cookbook](use/decision-packet-cookbook.md)
- [에이전트 세션 흐름](use/agent-session-flow.md)

## 구현 문서

구현 방향을 파악하고 계획을 리뷰하기 위한 경로입니다. 먼저 [문서 수락 상태](build/implementation-overview.md#문서-승인-상태)를 확인합니다. 유지보수자가 그곳에서 구현 계획을 명시적으로 수락하기 전까지 Build 문서는 계획 지침이며 하네스 서버/런타임 구현을 승인하지 않습니다.

- [구현 개요](build/implementation-overview.md)
- [문서 수락 후보 요약](build/implementation-overview.md#문서-수락-후보-요약)
- [첫 실행 가능한 조각](build/first-runnable-slice.md)
- [Runtime Walkthrough](build/runtime-walkthrough.md)
- [MVP 계획](build/mvp-plan.md)

## 참조 문서

엄격한 계약을 찾아보는 경로입니다. 다른 경로에서 엄격한 규칙을 요약했다면 먼저 고쳐야 할 기준 문서는 해당 기준 문서 소유자입니다.

- [커널 참조](reference/kernel.md)
- [런타임 아키텍처 참조](reference/runtime-architecture.md)
- [보안 위협 모델 참조](reference/security-threat-model.md)
- [MCP API와 스키마](reference/mcp-api-and-schemas.md)
- [Storage와 DDL](reference/storage-and-ddl.md)
- [문서 Projection 참조](reference/document-projection.md)
- [설계 품질 정책](reference/design-quality-policies.md)
- [에이전트 통합 참조](reference/agent-integration.md)
- [Surface Cookbook](reference/surface-cookbook.md)
- [운영과 Conformance 참조](reference/operations-and-conformance.md)
- [Conformance Fixtures 참조](reference/conformance-fixtures.md)
- [용어집 참조](reference/glossary.md)
- [Template 참조](reference/templates/README.md)

## 유지보수 문서

문서와 이후 하네스 시스템의 일관성을 유지하기 위한 경로입니다. Maintain 문서는 런타임 동작이 아니라 문서 유지보수를 관리합니다.

- [문서 작성 가이드](maintain/authoring-guide.md)
- [번역 가이드](maintain/translation-guide.md)

## 로드맵

- [로드맵](roadmap.md)

Post-MVP 항목은 Roadmap에 둡니다. 향후 담당자가 범위, fixture, fallback 동작을 정해 항목을 명시적으로 승격하기 전까지 Roadmap 항목은 Build-owned staged delivery에 포함되지 않습니다.

## 언어 의미 일치

영어 문서와 한국어 문서는 같은 파일 지도와 의미상 같은 내용을 유지합니다. 한국어 문서는 영어 문장을 한 줄씩 옮기기보다 자연스러운 한국어 제목과 흐름을 사용할 수 있습니다.
