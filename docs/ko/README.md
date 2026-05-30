# 하네스 문서

이 문서는 하네스 한국어 문서 세트의 길잡이입니다.

이 저장소는 현재 문서 전용 재설계/검토 저장소입니다. 문서 승인 이후에는 하네스 서버 소스 저장소가 되는 것을 목표로 합니다. 아직 이곳에는 하네스 서버/런타임 구현이 없습니다.

## 제품 명제

한 문장으로 말하면, 하네스는 AI 지원 제품 작업에서 작업 범위, 사용자 판단, 근거, 검증, QA 기대, 최종 작업 수락, 남은 위험 상태를 깨지기 쉬운 대화 맥락 밖에 두는 로컬 기준 기록이자 판단 경로입니다.

조금 풀어 말하면, 하네스는 어떤 작업이 범위 안에 있는지, 어떤 판단이 사용자에게 남아 있는지, 완료 주장을 무엇이 뒷받침하는지, 어떤 검증이나 QA가 아직 필요한지, 작업 수락이 이루어졌는지, 어떤 위험이 남았는지를 사용자와 에이전트가 함께 볼 수 있는 로컬 기록으로 남깁니다. 대화는 대화로 남습니다. Markdown 투영 문서는 사람이 읽는 보기입니다. Core가 소유한 로컬 상태와 아티팩트 참조가 운영상 기준입니다. 하네스는 agent instruction, MCP, reusable workflow, 테스트, 리뷰, spec을 사용할 수 있지만 그중 어느 하나와 같은 것은 아닙니다.

하네스가 집중하는 문제는 네 가지입니다.

- 작업 범위가 흐르거나 암묵적으로 바뀝니다.
- 사용자 판단이 조용히 에이전트 판단으로 바뀝니다.
- 근거, 검증, QA, 완료 주장이 뒤섞입니다.
- 대화나 Markdown 출력이 운영상 기준으로 오해됩니다.

## 지금 보는 저장소

하네스는 세 공간을 분리합니다.

| 공간 | 들어가는 것 |
|---|---|
| 제품 저장소 | 사용자의 제품 작업 공간입니다. 제품 코드, 테스트, 제품 문서, 사람이 읽는 하네스 투영 문서가 여기에 속합니다. |
| 하네스 서버 소스 저장소 | 로컬 하네스 서버/설치 프로그램의 코드베이스가 될 공간입니다. API 표면, 요청 검증, Core 상태 전이, 검증기, 투영, reconcile, 운영자 도구는 문서 승인 뒤 여기에 구현될 예정입니다. |
| 하네스 런타임 홈 | 사용자별/설치별 운영 데이터 공간입니다. 상태 데이터베이스, 아티팩트 저장소, 투영 출력, 로그, 로컬 등록/설정 정보가 여기에 속합니다. |

이 저장소의 현재 역할은 문서 검토와 재설계입니다. 향후 역할은 하네스 서버 소스 저장소입니다. 제품 저장소도 하네스 런타임 홈도 아닙니다. 문서 승인 이후에는 하네스 서버/설치 프로그램 구현이 이 저장소에서 진행될 예정입니다.

## 문서 재설계 범위

현재 저장소 상태는 문서 검토와 재설계입니다. 문서 승인과 구현 계획 상태는 [구현 개요](build/implementation-overview.md#문서-승인-상태)에서 확인합니다.

이번 재설계에서는 용어, MVP 단계, 스키마(schema) 구조, 투영(projection) 구조, 보안 표현, 문서 구성이 바뀔 수 있습니다. 정리된 제품 명제와 구현 가능한 경로를 우선하며, 기존 문구는 연속성만으로 보존하지 않습니다.

전체 재설계 범위, 보존 원칙, 문서군별 역할, [알려진 재설계 쟁점 추적 목록](maintain/authoring-guide.md#알려진-재설계-쟁점-트래커)은 [문서 작성 가이드](maintain/authoring-guide.md#현재-재설계-범위)가 담당합니다.

## 하네스가 아닌 것

하네스는 agent instruction, MCP, reusable workflow, 테스트, 리뷰, spec과 같은 역할을 하지 않습니다. 이런 요소들은 하네스 주변에서 유용할 수 있지만, 로컬 운영 기록이나 사용자 판단의 주인이 되지는 않습니다.

하네스는 prompt 묶음, 대화 스크립트, evaluation harness, dashboard, 넓은 hosted agent platform도 아닙니다.

## 비교

| 인접 개념 | 그 역할 | 하네스의 역할 |
|---|---|---|
| AGENTS.md / agent instruction 파일 | 저장소나 세션에서 에이전트가 어떻게 행동해야 하는지 알려 줍니다. | 하네스는 그런 지침을 사용할 수 있지만, 범위, 판단, 근거, 닫을 수 있는 상태, 남은 위험을 로컬 기록으로 유지합니다. |
| MCP | 도구와 리소스를 연결하는 protocol boundary입니다. | 하네스는 MCP 도구나 리소스를 노출할 수 있지만, 하네스의 기준은 Core가 소유한 로컬 상태와 아티팩트 참조에서 나옵니다. |
| Skill / reusable workflow | 에이전트가 반복해서 따를 수 있는 지침이나 절차를 묶습니다. | 하네스는 그런 workflow 안에서 사용될 수 있지만, 지금 진행 중인 작업 상태를 기록하고 이 작업의 판단을 라우팅합니다. |
| Test runner | 검사를 실행하고 결과를 냅니다. | 하네스는 관련 결과를 근거로 연결하고, 검증의 강도와 최종 작업 수락을 따로 둡니다. |
| Code review | 변경을 사람이 또는 팀이 검토합니다. | 하네스는 리뷰 결과를 참조할 수 있지만, 리뷰를 대체하거나 리뷰를 작업 수락 또는 잔여 위험 수용으로 바꾸지 않습니다. |
| Spec | 의도한 동작, 설계, 제약을 설명합니다. | 하네스는 spec을 입력으로 사용할 수 있지만, 실제 작업의 운영 상태인 범위, 결정, 근거, QA 기대, 작업 수락, 남은 위험을 기록합니다. |

## 독자별 경로

| 독자 역할 | 먼저 읽을 문서 | 이어서 볼 문서 |
|---|---|---|
| 처음 읽는 사람 | [15분 만에 보는 하네스](learn/harness-in-15-minutes.md) | [개요](learn/overview.md), [하나의 작업으로 보는 하네스](learn/harness-in-one-task.md), 그다음 [핵심 개념](learn/concepts.md) |
| 사용자 | [사용자 가이드](use/user-guide.md) | [결정 패킷 Cookbook](use/decision-packet-cookbook.md), 그다음 Agent-facing 흐름이 필요하면 [Agent 세션 흐름](use/agent-session-flow.md) |
| 구현자 | [구현 개요](build/implementation-overview.md) | [첫 실행 가능한 조각](build/first-runnable-slice.md), [Runtime Walkthrough](build/runtime-walkthrough.md), [MVP 계획](build/mvp-plan.md), 그다음 관련 Reference owner |
| 운영자 | [운영과 Conformance 참조](reference/operations-and-conformance.md#계약-위치-지도) | [런타임 아키텍처](reference/runtime-architecture.md), [보안 위협 모델](reference/security-threat-model.md), [MCP API와 스키마](reference/mcp-api-and-schemas.md), [Storage와 DDL](reference/storage-and-ddl.md) |
| Conformance 작성자 | [Conformance Fixtures 참조](reference/conformance-fixtures.md#conformance-탐색-지도) | [운영과 Conformance 참조](reference/operations-and-conformance.md#conformance-run), [MCP API와 스키마](reference/mcp-api-and-schemas.md), [Storage와 DDL](reference/storage-and-ddl.md), [커널 참조](reference/kernel.md) |
| 문서 유지보수 담당자 | [문서 작성 가이드](maintain/authoring-guide.md) | [번역 가이드](maintain/translation-guide.md) |

## 소유권 규칙

정확한 계약은 Reference 문서가 담당합니다. Schema, DDL, 관문(gate), state transition, enum value, fixture 의미, template 본문, 공식 정의가 여기에 속합니다. Learn, Use, Build 문서는 독자에게 필요한 생각을 설명하고 Reference로 연결하며, 엄격한 계약 블록을 복사하지 않습니다.

Docs-maintenance check는 읽기 전용 리뷰 지침이며 runtime conformance나 implementation readiness가 아닙니다. Drift category와 owner-first resolution은 [문서 작성 가이드](maintain/authoring-guide.md#docs-maintenance-checks)를 사용하고, docs-maintenance profile reporting boundary는 [운영과 Conformance](reference/operations-and-conformance.md#docs-maintenance-프로필)를 사용합니다.

운영자는 procedure와 conformance run overview를 위해 [운영과 Conformance 참조](reference/operations-and-conformance.md)를 사용합니다. Fixture 작성자는 fixture body shape, assertion semantics, suite catalog, example, catalog-only future candidate를 위해 [Conformance Fixtures 참조](reference/conformance-fixtures.md)를 사용합니다.

## Learn

정확한 계약에 들어가기 전에 전체 그림을 잡는 경로입니다.

- [개요](learn/overview.md)
- [15분 만에 보는 하네스](learn/harness-in-15-minutes.md)
- [하나의 작업으로 보는 하네스](learn/harness-in-one-task.md)
- [핵심 개념](learn/concepts.md)
- [목적과 원칙](learn/purpose-and-principles.md)

## Use

AI 지원 개발 세션을 하네스 기준으로 진행할 때 보는 경로입니다. 이 문서들은 사용자에게 보이는 흐름, 상태 해석, 결정 지점, 복구 경로를 우선합니다.

- [사용자 가이드](use/user-guide.md)
- [결정 패킷 Cookbook](use/decision-packet-cookbook.md)
- [Agent 세션 흐름](use/agent-session-flow.md)

## Build

구현 방향을 파악하고 계획을 리뷰하기 위한 경로입니다. 첫 경로는 좁게 유지합니다. v0.1 Kernel MVP를 먼저 두고, Kernel Smoke를 그 좁은 conformance profile로 사용하며, v0.2 Evidence & Projection Pack, v0.3 Agency Pack, v0.4 Operations Pack은 Agency-Hardened MVP reference conformance target으로 가는 staged pack으로 다룹니다. v1+ Expansion은 owner 문서가 승격하고 증명하기 전까지 staged delivery 밖에 둡니다.

먼저 [문서 승인 상태](build/implementation-overview.md#문서-승인-상태)를 확인합니다. maintainer가 그곳에서 구현 계획을 명시적으로 승인하기 전까지 Build 문서는 계획 지침이며 하네스 서버/런타임 구현을 승인하지 않습니다.

- [구현 개요](build/implementation-overview.md)
- [첫 실행 가능한 조각](build/first-runnable-slice.md)
- [Runtime Walkthrough](build/runtime-walkthrough.md)
- [MVP 계획](build/mvp-plan.md)

## Reference

엄격한 계약을 찾아보는 경로입니다. 다른 경로에서 엄격한 규칙을 요약했다면 먼저 고쳐야 할 기준 문서는 해당 Reference owner입니다.

- [커널 참조](reference/kernel.md)
- [런타임 아키텍처 참조](reference/runtime-architecture.md)
- [보안 위협 모델 참조](reference/security-threat-model.md)
- [MCP API와 스키마](reference/mcp-api-and-schemas.md)
- [Storage와 DDL](reference/storage-and-ddl.md)
- [문서 Projection 참조](reference/document-projection.md)
- [설계 품질 정책](reference/design-quality-policies.md)
- [Agent 통합 참조](reference/agent-integration.md)
- [Surface Cookbook](reference/surface-cookbook.md)
- [운영과 Conformance 참조](reference/operations-and-conformance.md)
- [Conformance Fixtures 참조](reference/conformance-fixtures.md)
- [용어집 참조](reference/glossary.md)
- [Template 참조](reference/templates/README.md)

## Maintain

문서와 이후 하네스 시스템의 일관성을 유지하기 위한 경로입니다. Maintain 문서는 런타임 동작이 아니라 문서 유지보수를 관리합니다.

- [문서 작성 가이드](maintain/authoring-guide.md)
- [번역 가이드](maintain/translation-guide.md)

## Roadmap

- [로드맵](roadmap.md)

Post-MVP 항목은 Roadmap에 둡니다. 향후 담당자가 범위, fixture, fallback 동작을 정해 항목을 명시적으로 승격하기 전까지 Roadmap 항목은 Build-owned staged delivery에 포함되지 않습니다.

## 언어 의미 일치

영어 문서와 한국어 문서는 같은 파일 지도와 의미상 같은 내용을 유지합니다. 한국어 문서는 영어 문장을 한 줄씩 옮기기보다 자연스러운 한국어 제목과 흐름을 사용할 수 있습니다.
