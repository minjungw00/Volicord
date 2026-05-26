# Harness 문서

이 문서는 독자 중심 Harness 한국어 문서 세트의 진입점입니다.

Harness는 AI 지원 제품 작업을 위한, 사용자 판단권을 보존하는 로컬 권한 커널입니다. 범위, 사용자 소유 판단, 쓰기 권한, 근거, 검증, QA, 수락, 남은 위험, 닫기 상태를 로컬 운영 기록으로 유지합니다.

정체성, 권한 모델, 통합 접점 경계, 비목표의 전체 설명은 [개요](learn/overview.md)와 [목적과 원칙](learn/purpose-and-principles.md)을 봅니다.

## Harness는 무엇이고 무엇이 아닌가

Harness는 AI 지원 작업을 로컬 상태, 오래 보관할 근거, 읽기용 문서에서 따라갈 수 있게 만들기 위한 시스템입니다. 이 저장소는 아직 문서 검토 단계이며, Harness server 또는 runtime 구현을 담고 있지 않습니다.

Harness는 대화 스크립트, prompt 묶음, test harness, evaluation harness, dashboard, 사용자의 제품 저장소, 버전 관리, 테스트, 코드 리뷰, 제품 판단과 기술 판단을 대체하지 않습니다.

## 빠른 경로

| 이런 경우 | 먼저 읽을 문서 | 이어서 읽을 문서 |
|---|---|---|
| Harness를 처음 이해하려는 경우 | [개요](learn/overview.md) | [하나의 작업으로 보는 Harness](learn/harness-in-one-task.md) |
| AI 지원 개발 중 Harness를 사용하려는 경우 | [사용자 가이드](use/user-guide.md) | [Agent 세션 흐름](use/agent-session-flow.md) |
| 문서 승인 뒤 구현을 준비하는 경우 | [구현 개요](build/implementation-overview.md) | [첫 실행 가능한 조각](build/first-runnable-slice.md), [MVP 계획](build/mvp-plan.md), 그다음 Reference |
| 정확한 동작이나 안정적인 이름을 찾는 경우 | [Reference](#reference) | 필요한 계약의 owner 문서 |
| 문서를 유지보수하거나 리뷰하는 경우 | [문서 작성 가이드](maintain/authoring-guide.md) | [번역 가이드](maintain/translation-guide.md) |

## 소유권 규칙

정확한 계약은 Reference 문서가 담당합니다. Schema, DDL, gate, state transition, enum value, fixture 의미, template 본문, 공식 정의가 여기에 속합니다. Learn, Use, Build 문서는 독자에게 필요한 생각을 설명하고 Reference로 연결하며, 엄격한 계약 블록을 복사하지 않습니다.

## Learn

Harness를 사용하거나 구현하기 전에 전체 그림과 핵심 개념을 이해하는 경로입니다. Learn 문서는 구체적인 예시로 이해 모델을 설명하고, 엄격한 계약은 Reference에 둡니다. 권장 순서는 [개요](learn/overview.md)를 먼저 읽고, 이어서 [하나의 작업으로 보는 Harness](learn/harness-in-one-task.md)를 읽는 것입니다.

- [개요](learn/overview.md)
- [하나의 작업으로 보는 Harness](learn/harness-in-one-task.md)
- [핵심 개념](learn/concepts.md)
- [목적과 원칙](learn/purpose-and-principles.md)

## Use

AI 지원 개발 세션을 Harness 기준으로 진행할 때 보는 경로입니다. Use 문서는 사용자에게 보이는 흐름, 상태 해석, 결정 지점, 복구 경로를 우선합니다. 먼저 사용자 가이드를 보고, agent가 어떻게 진행해야 하는지 확인할 때 세션 흐름을 봅니다.

- [사용자 가이드](use/user-guide.md)
- [Agent 세션 흐름](use/agent-session-flow.md)

## Build

구현 방향을 파악하고 나중에 계획을 리뷰하기 위한 경로입니다. 이 문서들은 Harness server 또는 runtime 구현을 시작해도 된다는 승인으로 해석하면 안 되며, 첫 runtime batch 계획은 maintainer가 문서 승인 상태를 명시적으로 갱신한 뒤에만 시작할 수 있습니다. Build 문서는 정확한 schema나 DDL을 중복하지 않고 구현 순서, module 경계, 실행 가능한 조각, 검증 전략을 설명합니다.

[구현 개요](build/implementation-overview.md#문서-승인-상태)의 문서 승인 상태에서 작업이 아직 문서 유지보수인지, 첫 runtime batch 계획을 시작할 수 있는지, runtime/server 구현이 시작되었는지, 열려 있는 문서 후속 이슈가 기록되어 있는지 확인합니다. 이 상태는 maintainer가 명시적으로 갱신하는 위치입니다.

- [구현 개요](build/implementation-overview.md)
- [첫 실행 가능한 조각](build/first-runnable-slice.md)
- [MVP 계획](build/mvp-plan.md)

## Reference

세부 계약, schema, 정책, 정의를 찾아보는 경로입니다. 다른 경로에서 엄격한 규칙을 요약했다면 먼저 고쳐야 할 기준 문서는 해당 Reference owner입니다.

- [커널 참조](reference/kernel.md)
- [런타임 아키텍처 참조](reference/runtime-architecture.md)
- [MCP API와 스키마](reference/mcp-api-and-schemas.md)
- [Storage와 DDL](reference/storage-and-ddl.md)
- [문서 Projection 참조](reference/document-projection.md)
- [설계 품질 정책](reference/design-quality-policies.md)
- [Agent 통합 참조](reference/agent-integration.md)
- [Surface Cookbook](reference/surface-cookbook.md)
- [운영과 Conformance 참조](reference/operations-and-conformance.md)
- [용어집 참조](reference/glossary.md)
- [Template 참조](reference/templates/README.md)

## Maintain

문서와 이후 Harness 시스템의 일관성을 유지하기 위한 경로입니다. Maintain 문서는 런타임 동작이 아니라 문서 유지보수를 관리합니다.

- [문서 작성 가이드](maintain/authoring-guide.md)
- [번역 가이드](maintain/translation-guide.md)

## Roadmap

- [로드맵](roadmap.md)

Post-MVP 항목은 Roadmap에 둡니다. 향후 담당자가 범위, fixture, fallback 동작을 정해 항목을 명시적으로 승격하기 전까지 Roadmap 항목은 MVP 구현 계약에 포함되지 않습니다.

## 언어 의미 일치

영어 문서와 한국어 문서는 같은 파일 지도와 의미상 같은 내용을 유지합니다. 한국어 문서는 영어 문장을 한 줄씩 옮기기보다 자연스러운 한국어 제목과 흐름을 사용할 수 있습니다.
