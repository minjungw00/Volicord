# Harness 문서

이 문서는 독자 중심 Harness 한국어 문서 세트의 진입점입니다.

Harness는 AI 지원 제품 작업을 위한, 사용자 판단권을 보존하는 로컬 권한 커널입니다.

Harness는 단순한 대화 흐름, prompt skill 묶음, test harness, evaluation harness가 아닙니다.

MCP 도구/connector, hook, guardrail, adapter, sidecar, isolation layer와 통합될 수 있지만, 이런 접점과 장치는 Harness 권한의 출처가 아닙니다. 이들은 에이전트가 맥락을 읽고, Harness 도구를 호출하고, 근거를 캡처하고, 연결된 profile이 지원하는 범위에서 경계를 차단하거나 사후 감지하도록 돕습니다.

Harness 권한은 Core와 기준 로컬 상태에서 나옵니다. Task와 Change Unit 범위, Decision Packet, Approval, Write Authorization, 근거, 검증, QA, 수용, 남은 위험, 닫기를 기준으로 합니다.

Harness는 사용자의 제품 저장소, 버전 관리 시스템, 테스트 실행기, 코드 리뷰 절차, 제품 판단도 대체하지 않습니다.

## Learn

Harness를 사용하거나 구현하기 전에 전체 그림과 핵심 개념을 이해하는 경로입니다. 권장 순서는 [개요](learn/overview.md)를 먼저 읽고, 이어서 [하나의 작업으로 보는 Harness](learn/harness-in-one-task.md)를 읽는 것입니다.

- [개요](learn/overview.md)
- [하나의 작업으로 보는 Harness](learn/harness-in-one-task.md)
- [핵심 개념](learn/concepts.md)
- [목적과 원칙](learn/purpose-and-principles.md)

## Use

AI 지원 개발 세션을 Harness 기준으로 진행할 때 보는 경로입니다. 먼저 사용자 가이드를 보고, agent가 어떻게 진행해야 하는지 확인할 때 세션 흐름을 봅니다.

- [사용자 가이드](use/user-guide.md)
- [Agent 세션 흐름](use/agent-session-flow.md)

## Build

구현 계획과 리뷰를 위한 경로입니다. 이 문서들은 Harness server 또는 runtime 구현을 시작해도 된다는 승인으로 해석하면 안 됩니다. 실제 구현은 재설계 문서가 승인된 뒤에만 시작합니다. 전체 reference 명세를 읽기 전에 여기서 무엇을 먼저 만들지, 첫 실행 가능한 증명이 무엇을 보여야 하는지, MVP 단계가 어떻게 이어지는지 확인합니다.

- [구현 개요](build/implementation-overview.md)
- [첫 실행 가능한 조각](build/first-runnable-slice.md)
- [MVP 계획](build/mvp-plan.md)

## Reference

세부 계약, schema, 정책, 정의를 찾아보는 경로입니다.

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

문서와 이후 Harness 시스템의 일관성을 유지하기 위한 경로입니다.

- [문서 작성 가이드](maintain/authoring-guide.md)
- [번역 가이드](maintain/translation-guide.md)

## Roadmap

- [로드맵](roadmap.md)

Post-MVP 항목은 Roadmap에 둡니다. 향후 담당자가 범위, fixture, fallback 동작을 정해 항목을 명시적으로 승격하기 전까지 Roadmap 항목은 MVP 구현 계약에 포함되지 않습니다.

## 언어 의미 일치

영어 문서와 한국어 문서는 같은 파일 지도와 의미상 같은 내용을 유지합니다. 한국어 문서는 영어 문장을 한 줄씩 옮기기보다 자연스러운 한국어 제목과 흐름을 사용할 수 있습니다.
