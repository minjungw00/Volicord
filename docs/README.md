# Harness Documentation / 하네스 문서

This is the compact bilingual routing page for the Harness documentation set.

이 문서는 Harness 문서 세트의 간결한 이중 언어 길잡이입니다.

Harness is a local authority record and judgment-routing layer for AI-assisted product work. It keeps scope, user-owned judgments, evidence, verification, QA expectations, final acceptance, and residual-risk status outside fragile chat context.

Harness는 AI 지원 제품 작업에서 작업 범위, 사용자 판단, 근거, 검증, QA 기대, 작업 수락, 잔여 위험 상태를 깨지기 쉬운 대화 맥락 밖에 두는 로컬 기준 기록이자 판단 경로입니다.

This repository is currently a documentation-only redesign/review repository. After documentation acceptance, it is intended to become the Harness Server source repository. It is not a Product Repository or a Harness Runtime Home, and no Harness Server/runtime implementation exists here yet.

이 저장소는 현재 문서 전용 재설계/검토 저장소입니다. 문서 승인 이후에는 하네스 서버 소스 저장소가 되는 것을 목표로 합니다. 제품 저장소나 하네스 런타임 홈이 아니며, 아직 이곳에는 하네스 서버/런타임 구현이 없습니다.

The [Authoring Guide](en/maintain/authoring-guide.md#current-redesign-scope) owns the full redesign scope and preserved principles. The maintainer handoff summary and implementation-readiness criteria are in [Implementation Overview](en/build/implementation-overview.md#maintainer-handoff-summary).

전체 재설계 범위와 보존 원칙은 [문서 작성 가이드](ko/maintain/authoring-guide.md#현재-재설계-범위)가 담당합니다. 유지보수자용 문서 수락 후보 요약과 하네스 서버 구현 준비 조건은 [구현 개요](ko/build/implementation-overview.md#문서-수락-후보-요약)에 있습니다.

## Choose A Language / 언어 선택

| Language / 언어 | Entry point / 진입점 |
|---|---|
| English | [en/README.md](en/README.md) |
| 한국어 | [ko/README.md](ko/README.md) |

## Primary Reader Path / 주요 읽기 경로

| Step / 순서 | English | 한국어 |
|---|---|---|
| 1 | [Overview](en/learn/overview.md) | [개요](ko/learn/overview.md) |
| 2 | [User Guide](en/use/user-guide.md) | [사용자 가이드](ko/use/user-guide.md) |
| 3 | [Concepts](en/learn/concepts.md) | [핵심 개념](ko/learn/concepts.md) |
| 4 | [Implementation Overview](en/build/implementation-overview.md) / [MVP Plan](en/build/mvp-plan.md) | [구현 개요](ko/build/implementation-overview.md) / [MVP 계획](ko/build/mvp-plan.md) |
| 5 | [Reference docs](en/README.md#reference) | [참조 문서](ko/README.md#참조-문서) |

## Maintainer Handoff / 문서 수락 후보

| Need / 필요한 것 | English | 한국어 |
|---|---|---|
| Handoff summary / 수락 후보 요약 | [Maintainer handoff summary](en/build/implementation-overview.md#maintainer-handoff-summary) | [문서 수락 후보 요약](ko/build/implementation-overview.md#문서-수락-후보-요약) |
| Acceptance status / 문서 승인 상태 | [Documentation acceptance status](en/build/implementation-overview.md#documentation-acceptance-status) | [문서 승인 상태](ko/build/implementation-overview.md#문서-승인-상태) |
| Implementation readiness / 구현 준비 조건 | [Implementation-readiness criteria](en/build/implementation-overview.md#implementation-readiness-criteria) | [하네스 서버 구현 준비 조건](ko/build/implementation-overview.md#하네스-서버-구현-준비-조건) |
| Decisions before server coding / 서버 코딩 전 결정 | [Implementation decisions needed before server coding](en/build/mvp-plan.md#implementation-decisions-needed-before-server-coding) | [서버 코딩 전 필요한 구현 결정](ko/build/mvp-plan.md#서버-코딩-전-필요한-구현-결정) |

Before starting Harness Server code, implementers should read the handoff summary, confirm the status table, check the readiness criteria and decisions section, then use the First Runnable Slice for v0.1 planning.

하네스 서버 코드를 시작하기 전 구현자는 문서 수락 후보 요약, 승인 상태 표, 구현 준비 조건과 구현 시작 전 결정 섹션을 확인한 뒤 첫 실행 가능한 조각으로 v0.1 계획을 봅니다.

## Reader Paths / 독자별 경로

| Reader / 독자 | English | 한국어 |
|---|---|---|
| User / 사용자 | [Overview](en/learn/overview.md) -> [User Guide](en/use/user-guide.md) -> [Concepts](en/learn/concepts.md) | [개요](ko/learn/overview.md) -> [사용자 가이드](ko/use/user-guide.md) -> [핵심 개념](ko/learn/concepts.md) |
| Agent integrator / 에이전트 통합자 | [Agent Session Flow](en/use/agent-session-flow.md) -> [Agent Integration Reference](en/reference/agent-integration.md) -> [Surface Cookbook](en/reference/surface-cookbook.md) | [에이전트 세션 흐름](ko/use/agent-session-flow.md) -> [에이전트 통합 참조](ko/reference/agent-integration.md) -> [Surface Cookbook](ko/reference/surface-cookbook.md) |
| Implementer / 구현자 | [Implementation Overview](en/build/implementation-overview.md#maintainer-handoff-summary) -> [MVP Plan decisions](en/build/mvp-plan.md#implementation-decisions-needed-before-server-coding) -> [First Runnable Slice](en/build/first-runnable-slice.md) -> [MVP Plan](en/build/mvp-plan.md) | [구현 개요](ko/build/implementation-overview.md#문서-수락-후보-요약) -> [MVP 계획의 결정 섹션](ko/build/mvp-plan.md#서버-코딩-전-필요한-구현-결정) -> [첫 실행 가능한 조각](ko/build/first-runnable-slice.md) -> [MVP 계획](ko/build/mvp-plan.md) |
| Reviewer / maintainer / 검토자 / 문서 유지보수자 | [Authoring Guide](en/maintain/authoring-guide.md) -> [Translation Guide](en/maintain/translation-guide.md) | [문서 작성 가이드](ko/maintain/authoring-guide.md) -> [번역 가이드](ko/maintain/translation-guide.md) |

Use the language-specific entrypoints for detailed document roles, Reference owner links, and maintenance guidance.

상세 문서 역할, Reference owner 링크, 유지보수 지침은 언어별 진입점을 사용합니다.

## Optional First Examples / 선택해서 보는 첫 예시

| Need / 필요한 것 | English | 한국어 |
|---|---|---|
| Quick scenario sampler / 빠른 시나리오 모음 | [Harness in 15 Minutes](en/learn/harness-in-15-minutes.md) | [15분 만에 보는 하네스](ko/learn/harness-in-15-minutes.md) |
| Full task tutorial / 전체 작업 튜토리얼 | [Harness in One Task](en/learn/harness-in-one-task.md) | [하나의 작업으로 보는 하네스](ko/learn/harness-in-one-task.md) |
| Decision examples / 판단 예시 | [Decision Packet Cookbook](en/use/decision-packet-cookbook.md) | [결정 패킷 Cookbook](ko/use/decision-packet-cookbook.md) |

Maintainer review risks are tracked in the [English Authoring Guide](en/maintain/authoring-guide.md#known-redesign-issues-tracker) and [Korean Authoring Guide](ko/maintain/authoring-guide.md#알려진-재설계-쟁점-트래커). They are not open implementation decisions.

유지보수자 검토 위험은 [영어 문서 작성 가이드](en/maintain/authoring-guide.md#known-redesign-issues-tracker)와 [한국어 문서 작성 가이드](ko/maintain/authoring-guide.md#알려진-재설계-쟁점-트래커)에서 관리합니다. 열린 구현 결정이 아닙니다.
