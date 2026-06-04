# Harness Documentation / 하네스 문서

Harness is a local work-authority server for AI-assisted product work. Its job is to keep fragile conversation context from becoming the source of truth. It preserves the local basis for scope, user-owned judgment, evidence, verification expectations, work acceptance, close readiness, and residual risk, and routes decisions back to the user when the agent should not decide.

하네스는 AI 지원 제품 작업을 위한 로컬 작업 권한 서버입니다. 대화의 깨지기 쉬운 맥락이 기준 기록처럼 굳어지지 않게 하는 것이 하네스의 역할입니다. 하네스는 범위, 사용자 소유 판단, 근거, 확인과 검증 기대, 작업 수락, 닫기 가능 여부, 잔여 위험의 로컬 근거를 보존합니다. 에이전트가 판단하면 안 되는 일은 사용자에게 다시 돌려보냅니다.

| Harness is not / 하네스가 아닌 것 | Harness does / 하네스가 하는 일 |
|---|---|
| A prompt pack, MCP itself, or an API wrapper. / 프롬프트 묶음, MCP 자체, API 래퍼. | May use MCP/API surfaces while keeping authority in local records. / MCP/API 접점을 사용할 수 있지만 권한은 로컬 기록에 둡니다. |
| A workflow engine, report generator, dashboard, or hosted agent platform. / 워크플로 엔진, 보고서 생성기, 대시보드, 호스팅 에이전트 플랫폼. | Preserves the basis for work and can derive readable views from it. / 작업의 근거를 보존하고 그 기록에서 읽기용 보기를 만들 수 있습니다. |
| A sandbox or OS permission system. / 샌드박스나 OS 권한 시스템. | Preserves authority boundaries without claiming OS-level isolation. / OS 수준 격리를 주장하지 않고 권한 경계를 보존합니다. |

This repository is documentation-only today and its intended future role is the Harness Server source repository. It is not the user's Product Repository and not a Harness Runtime Home. No Harness Server, runtime, generated projection system, conformance runner, runtime data, product implementation code, or generated operational artifact exists here yet. Documentation acceptance does not authorize implementation by itself; server/runtime implementation may start only after documentation acceptance and a separate implementation-planning readiness decision.

이 저장소는 현재 문서 전용이며 향후 역할은 하네스 서버 소스 저장소입니다. 사용자의 제품 저장소가 아니고 하네스 런타임 홈도 아닙니다. 아직 하네스 서버, 런타임, 생성된 읽기용 요약 시스템, conformance runner, 런타임 데이터, 제품 구현 코드, 생성된 운영 아티팩트는 없습니다. 문서 수락만으로 구현은 허가되지 않으며, 서버/런타임 구현은 문서 수락과 별도의 구현 계획 준비 결정 이후에만 시작할 수 있습니다.

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
| Implementers only / 구현자만 | [Implementation Overview](en/build/implementation-overview.md), then [MVP-1 User Work Loop](en/build/mvp-user-work-loop.md) | [구현 개요](ko/build/implementation-overview.md), 그다음 [MVP-1 사용자 작업 루프](ko/build/mvp-user-work-loop.md) |
| Lookup only / 찾아볼 때만 | [Reference Index](en/reference/README.md) | [Reference 색인](ko/reference/README.md) |

This first-read path intentionally stops before large Reference docs. Use Reference only when you need an exact owner contract.

이 첫 읽기 경로는 큰 Reference 문서를 먼저 읽지 않도록 설계되어 있습니다. 정확한 owner 계약이 필요할 때만 Reference를 엽니다.

## Reader Paths / 독자별 경로

| Reader / 독자 | English path | 한국어 경로 |
|---|---|---|
| General user / 일반 사용자 | [Overview](en/learn/overview.md) -> [User Guide](en/use/user-guide.md); use [One Task](en/learn/one-task.md) for a fuller walkthrough. | [개요](ko/learn/overview.md) -> [사용자 가이드](ko/use/user-guide.md); 더 긴 흐름은 [하나의 작업](ko/learn/one-task.md). |
| Agent instruction writer / 에이전트 지침 작성자 | [Agent Guide](en/use/agent-guide.md); then [Agent Integration Reference](en/reference/agent-integration.md) only for exact connector/context contracts. | [에이전트 가이드](ko/use/agent-guide.md); 정확한 connector/context 계약이 필요할 때만 [Agent 통합 참조](ko/reference/agent-integration.md). |
| Server implementer / 서버 구현자 | [Implementation Overview](en/build/implementation-overview.md) -> [MVP-1 User Work Loop](en/build/mvp-user-work-loop.md) -> [MVP API](en/reference/api/mvp-api.md) -> [Storage](en/reference/storage.md). Use [Engineering Checkpoint](en/build/engineering-checkpoint.md) for the first internal smoke. | [구현 개요](ko/build/implementation-overview.md) -> [MVP-1 사용자 작업 루프](ko/build/mvp-user-work-loop.md) -> [MVP API](ko/reference/api/mvp-api.md) -> [Storage](ko/reference/storage.md). 첫 내부 점검은 [내부 엔지니어링 점검](ko/build/engineering-checkpoint.md). |
| Documentation maintainer / 문서 유지보수자 | [Authoring Guide](en/maintain/authoring-guide.md) -> [Translation Guide](en/maintain/translation-guide.md) -> [Rewrite Plan](en/maintain/rewrite-plan.md), with owner docs only for strict meaning. | [문서 작성 가이드](ko/maintain/authoring-guide.md) -> [번역 가이드](ko/maintain/translation-guide.md) -> [재작성 계획](ko/maintain/rewrite-plan.md), 엄격한 의미 확인에는 owner 문서만. |
| Later/profile reader / 이후 프로필 독자 | [Assurance Profile](en/later/assurance-profile.md), [Operations Profile](en/later/operations-profile.md), [Future Fixtures](en/later/future-fixtures.md), and [Roadmap](en/roadmap.md). These are not the MVP implementation path unless an owner promotes them. | [보증 프로필](ko/later/assurance-profile.md), [운영 프로필](ko/later/operations-profile.md), [향후 Fixtures](ko/later/future-fixtures.md), [로드맵](ko/roadmap.md). Owner가 승격하기 전까지 MVP 구현 경로가 아닙니다. |

## Status And Handoff / 상태와 인계

Detailed status belongs in the language READMEs and Build handoff docs:

- [English documentation acceptance status](en/build/implementation-overview.md#documentation-acceptance-status)
- [한국어 문서 수락 상태](ko/build/implementation-overview.md#문서-수락-상태)
- [English maintainer handoff summary](en/build/implementation-overview.md#maintainer-handoff-summary)
- [한국어 문서 인계 요약](ko/build/implementation-overview.md#문서-인계-요약)
- [English implementation decisions before server coding](en/build/mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding)
- [한국어 서버 코딩 전 필요한 구현 결정](ko/build/mvp-user-work-loop.md#서버-코딩-전-필요한-구현-결정)

상세 상태는 언어별 README와 Build 인계 문서가 담당합니다. 문서 수락만으로 런타임 구현이 시작되거나 런타임 conformance가 증명되지는 않습니다.

## Document Families / 문서군

| Family / 문서군 | Purpose / 목적 |
|---|---|
| Learn / 학습 | Why Harness exists and the core concepts. / 하네스가 왜 필요한지와 핵심 개념. |
| Use / 사용 | How users and agents interact during work. / 사용자와 에이전트가 작업 중 상호작용하는 법. |
| Build / 구현 | Future implementation sequence and staged plan. / 향후 구현 순서와 단계별 계획. |
| Reference / 참조 | Exact owner contracts, schemas, gates, DDL, projection, security, and conformance meanings. / 정확한 owner 계약, 스키마, gate, DDL, projection, 보안, conformance 의미. |
| Later / 이후 | Assurance, operations, future fixtures, and roadmap candidates kept outside the MVP path. / MVP 경로 밖에 두는 보증, 운영, 향후 fixtures, 로드맵 후보. |
| Maintain / 유지보수 | Documentation rules, redesign scope, parity, and drift handling. / 문서 규칙, 재설계 범위, 의미 일치, drift 처리. |

Maintainer review risks are checked in the [English Authoring Guide](en/maintain/authoring-guide.md#known-redesign-issues-and-regression-checks) and [Korean Authoring Guide](ko/maintain/authoring-guide.md#알려진-재설계-위험과-회귀-점검). Rewrite triage categories are in the [English Rewrite Plan](en/maintain/rewrite-plan.md) and [Korean Rewrite Plan](ko/maintain/rewrite-plan.md). Server-coding decisions belong in the MVP-1 User Work Loop plan.

유지보수자 검토 위험은 [영어 문서 작성 가이드](en/maintain/authoring-guide.md#known-redesign-issues-and-regression-checks)와 [한국어 문서 작성 가이드](ko/maintain/authoring-guide.md#알려진-재설계-위험과-회귀-점검)의 회귀 점검으로 확인합니다. 재작성 분류 값은 [영어 Rewrite Plan](en/maintain/rewrite-plan.md)과 [한국어 재작성 계획](ko/maintain/rewrite-plan.md)에 둡니다. 서버 코딩 전 결정은 MVP-1 사용자 작업 루프 계획에 기록합니다.
