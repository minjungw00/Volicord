# Harness Documentation / 하네스 문서

Harness is a local authority record and judgment-routing layer for AI-assisted product work. It keeps scope, user-owned judgments, evidence, verification, QA expectations, final acceptance, and residual-risk status outside fragile chat context.

Harness는 AI 지원 제품 작업에서 작업 범위, 사용자 판단, 근거, 검증, QA 기대, 작업 수락, 잔여 위험 상태를 깨지기 쉬운 대화 맥락 밖에 두는 로컬 기준 기록이자 판단 경로입니다.

This repository is a documentation review/redesign repository today. Its intended future role is the Harness Server source repository. It is not a Product Repository or a Harness Runtime Home. No Harness Server, runtime, generated projection system, conformance runner, runtime data, or implementation exists here yet. Documentation acceptance does not authorize implementation by itself; server/runtime implementation may start only after documentation acceptance and a separate implementation-planning readiness decision.

이 저장소는 현재 문서 검토/재설계 저장소입니다. 향후 역할은 하네스 서버 소스 저장소입니다. 제품 저장소나 하네스 런타임 홈이 아닙니다. 아직 하네스 서버, 런타임, 생성된 읽기용 요약 시스템, conformance runner, 런타임 데이터, 구현은 없습니다. 문서 수락만으로 구현은 허가되지 않으며, 서버/런타임 구현은 문서 수락과 별도의 구현 계획 준비 결정 이후에만 시작할 수 있습니다.

## Choose A Language / 언어 선택

| Language / 언어 | Entry point / 진입점 |
|---|---|
| English | [en/README.md](en/README.md) |
| 한국어 | [ko/README.md](ko/README.md) |

## Minimal First-Read Path / 최소 첫 읽기 경로

| Step / 순서 | English | 한국어 |
|---|---|---|
| 1 | [Overview](en/learn/overview.md) | [개요](ko/learn/overview.md) |
| 2 | [User Guide](en/use/user-guide.md) | [사용자 가이드](ko/use/user-guide.md) |
| 3, only if terms are unclear / 용어가 필요할 때만 | [Concepts](en/learn/concepts.md) | [핵심 개념](ko/learn/concepts.md) |
| 4, implementers only / 구현자만 | [Implementation Overview](en/build/implementation-overview.md) | [구현 개요](ko/build/implementation-overview.md) |
| Lookup only / 찾아볼 때만 | [Reference Index](en/reference/README.md) | [Reference 색인](ko/reference/README.md) |

This first-read path intentionally stops before large Reference docs. Use Reference only when you need an exact owner contract.

이 첫 읽기 경로는 큰 Reference 문서를 먼저 읽지 않도록 설계되어 있습니다. 정확한 owner 계약이 필요할 때만 Reference를 엽니다.

## Reader Paths / 독자별 경로

| Reader / 독자 | English path | 한국어 경로 |
|---|---|---|
| First-time reader / 처음 읽는 독자 | [Overview](en/learn/overview.md) -> [User Guide](en/use/user-guide.md); then [Concepts](en/learn/concepts.md) only when terms appear. | [개요](ko/learn/overview.md) -> [사용자 가이드](ko/use/user-guide.md); 용어가 나오면 [핵심 개념](ko/learn/concepts.md). |
| User / 사용자 | [User Guide](en/use/user-guide.md) -> [Harness in One Task](en/learn/harness-in-one-task.md); use [Decision Packet Cookbook](en/use/decision-packet-cookbook.md) for complex choices. | [사용자 가이드](ko/use/user-guide.md) -> [하나의 작업으로 보는 하네스](ko/learn/harness-in-one-task.md); 복잡한 판단은 [결정 패킷 예시 모음](ko/use/decision-packet-cookbook.md). |
| Agent behavior/integration author / 에이전트 통합자 | [Agent Session Flow](en/use/agent-session-flow.md) -> [Agent Integration Reference](en/reference/agent-integration.md) -> [Surface Cookbook](en/reference/surface-cookbook.md). | [에이전트 세션 흐름](ko/use/agent-session-flow.md) -> [에이전트 통합 참조](ko/reference/agent-integration.md) -> [Surface Cookbook](ko/reference/surface-cookbook.md). |
| Implementer / 구현자 | [Implementation Overview](en/build/implementation-overview.md) -> [MVP Plan](en/build/mvp-plan.md) -> [First Runnable Slice](en/build/first-runnable-slice.md) -> [Runtime Walkthrough](en/build/runtime-walkthrough.md) -> [Kernel](en/reference/kernel.md) -> [MCP/API schemas](en/reference/mcp-api-and-schemas.md) -> [Storage/DDL](en/reference/storage-and-ddl.md). | [구현 개요](ko/build/implementation-overview.md) -> [MVP 계획](ko/build/mvp-plan.md) -> [첫 실행 가능한 조각](ko/build/first-runnable-slice.md) -> [Runtime Walkthrough](ko/build/runtime-walkthrough.md) -> [커널](ko/reference/kernel.md) -> [MCP/API 스키마](ko/reference/mcp-api-and-schemas.md) -> [Storage/DDL](ko/reference/storage-and-ddl.md). |
| Documentation maintainer / 문서 유지보수자 | [Authoring Guide](en/maintain/authoring-guide.md) -> [Translation Guide](en/maintain/translation-guide.md), with owner docs only for strict meaning. | [문서 작성 가이드](ko/maintain/authoring-guide.md) -> [번역 가이드](ko/maintain/translation-guide.md), 엄격한 의미 확인에는 owner 문서만. |
| Future/reference reader / 향후/참조 독자 | [Reference Index](en/reference/README.md), then the one owner doc for the contract you need; use [Roadmap](en/roadmap.md) and [Future Fixture Catalog](en/reference/future-fixture-catalog.md) only for future or diagnostic material. | [Reference 색인](ko/reference/README.md), 그다음 필요한 계약의 owner 문서 하나만 엽니다. 향후 또는 diagnostic material은 [로드맵](ko/roadmap.md)과 [향후 Fixture Catalog](ko/reference/future-fixture-catalog.md)에서 따로 봅니다. |

## Status And Handoff / 상태와 인계

Detailed status belongs in the language READMEs and Build handoff docs:

- [English documentation acceptance status](en/build/implementation-overview.md#documentation-acceptance-status)
- [한국어 문서 수락 상태](ko/build/implementation-overview.md#문서-수락-상태)
- [English maintainer handoff summary](en/build/implementation-overview.md#maintainer-handoff-summary)
- [한국어 문서 인계 요약](ko/build/implementation-overview.md#문서-인계-요약)
- [English implementation decisions before server coding](en/build/mvp-plan.md#implementation-decisions-needed-before-server-coding)
- [한국어 서버 코딩 전 필요한 구현 결정](ko/build/mvp-plan.md#서버-코딩-전-필요한-구현-결정)

상세 상태는 언어별 README와 Build 인계 문서가 담당합니다. 문서 수락만으로 런타임 구현이 시작되거나 런타임 conformance가 증명되지는 않습니다.

## Document Families / 문서군

| Family / 문서군 | Purpose / 목적 |
|---|---|
| Learn / 학습 | Why Harness exists and the core concepts. / 하네스가 왜 필요한지와 핵심 개념. |
| Use / 사용 | How users and agents interact during work. / 사용자와 에이전트가 작업 중 상호작용하는 법. |
| Build / 구현 | Future implementation sequence and staged plan. / 향후 구현 순서와 단계별 계획. |
| Reference / 참조 | Exact owner contracts, schemas, gates, DDL, projection, security, and conformance meanings. / 정확한 owner 계약, 스키마, gate, DDL, projection, 보안, conformance 의미. |
| Maintain / 유지보수 | Documentation rules, redesign scope, parity, and drift handling. / 문서 규칙, 재설계 범위, 의미 일치, drift 처리. |

Maintainer review risks are tracked in the [English Authoring Guide](en/maintain/authoring-guide.md#known-redesign-issues-tracker) and [Korean Authoring Guide](ko/maintain/authoring-guide.md#알려진-재설계-쟁점-트래커). Server-coding decisions belong in the MVP Plan.

유지보수자 검토 위험은 [영어 문서 작성 가이드](en/maintain/authoring-guide.md#known-redesign-issues-tracker)와 [한국어 문서 작성 가이드](ko/maintain/authoring-guide.md#알려진-재설계-쟁점-트래커)에서 관리합니다. 서버 코딩 전 결정은 MVP 계획에 기록합니다.
