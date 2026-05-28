# 하네스 문서

이 문서는 하네스 한국어 문서 세트의 길잡이입니다.

이 저장소는 문서 재설계 / 피드백 반영 및 문서 검토 단계입니다. 이 페이지는 하네스 server/runtime 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data를 승인하지 않습니다. 첫 runtime batch 계획은 maintainer가 구현 handoff에서 문서를 명시적으로 승인하기 전까지 시작할 수 없습니다. 첫 제품 MVP 목표는 v0.1 Kernel MVP이며, Kernel Smoke는 이를 좁게 실행하는 conformance profile입니다. v0.2부터 v0.4까지는 Agency-Hardened MVP reference conformance target으로 가는 staged pack이고, v1+ Expansion은 owner 문서가 승격하고 증명하기 전까지 roadmap 범위에 남습니다.

하네스는 AI 지원 제품 작업을 위한 로컬 작업 장부이자 판단 라우터입니다. 무엇을 바꿀 수 있는지, 누가 판단해야 하는지, 어떤 근거가 있는지, 어떤 잔여 위험이 있는지, 작업을 닫아도 되는지를 기록합니다.

하네스는 사용자 판단권을 보존하는 로컬 권한 커널 원칙을 계속 따릅니다. 오래 남아야 하는 작업 사실은 지속 로컬 상태(durable local state), 아티팩트 참조, 읽기용 투영 문서(projection)에 두고, 사용자가 소유한 제품 판단과 중요한 기술 판단은 사용자에게 남겨 둡니다.

## 하네스가 아닌 것

하네스는 다음이 아닙니다.

- prompt 묶음
- source control, 테스트, 코드 리뷰, 사용자 판단의 대체물
- MCP 자체
- 넓은 hosted agent platform

하네스는 대화 스크립트, test harness, evaluation harness, dashboard도 아닙니다.

## 비교

| 인접 개념 | 하네스의 차이 |
|---|---|
| AGENTS.md / agent rules | Agent rule은 저장소나 세션에서 에이전트가 어떻게 행동해야 하는지 알려 줍니다. 하네스는 범위, 근거, 필요한 판단, 잔여 위험, 닫기 가능 상태를 기록하는 로컬 작업 장부를 유지합니다. |
| MCP | MCP는 tool과 resource를 위한 protocol boundary입니다. 하네스는 MCP tool을 노출할 수 있지만 MCP 자체는 아닙니다. 하네스 권한은 Core가 소유한 local record에서 나옵니다. |
| Spec-driven workflows | Spec은 의도한 동작이나 설계를 설명합니다. 하네스는 Task 주변의 실제 작업 상태, 즉 허용된 변경 경계, 사용자 결정, 근거, 잔여 위험, 닫을 수 있는지를 기록합니다. |
| Hooks / sidecars | Hook과 sidecar는 실제 보장 수준에 따라 관찰, 차단, 보고할 수 있습니다. 하네스는 그 한계를 기록하고 모든 효과를 관련 owner path로 라우팅합니다. |
| Test runners / code review | Test와 review는 제품 작업을 확인합니다. 하네스는 그 결과를 근거로 연결하되, 수락, 잔여 위험, 사용자 소유 판단은 따로 유지합니다. |

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

먼저 [문서 승인 상태](build/implementation-overview.md#문서-승인-상태)를 확인합니다. maintainer가 그곳에서 첫 runtime batch 계획을 명시적으로 승인하기 전까지 Build 문서는 계획 지침이며 runtime/server 구현을 승인하지 않습니다.

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
