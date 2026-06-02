# Harness Documentation / 하네스 문서

This is the compact bilingual routing page for the Harness documentation set.

이 문서는 Harness 문서 세트의 간결한 이중 언어 길잡이입니다.

Harness is a local authority record and judgment-routing layer for AI-assisted product work. It keeps scope, user-owned judgments, evidence, verification, QA expectations, final acceptance, and residual-risk status outside fragile chat context.

Harness는 AI 지원 제품 작업에서 작업 범위, 사용자 판단, 근거, 검증, QA 기대, 작업 수락, 잔여 위험 상태를 깨지기 쉬운 대화 맥락 밖에 두는 로컬 기준 기록이자 판단 경로입니다.

This repository is currently a documentation-only redesign/review repository. Its intended future role is the Harness Server source repository. It is not a Product Repository or a Harness Runtime Home, and no Harness Server/runtime implementation exists here yet. Server/runtime implementation may start only after documentation acceptance and a separate implementation-planning readiness decision.

이 저장소는 현재 문서 전용 재설계/검토 저장소입니다. 향후 역할은 하네스 서버 소스 저장소입니다. 제품 저장소나 하네스 런타임 홈이 아니며, 아직 이곳에는 하네스 서버/런타임 구현이 없습니다. 서버/런타임 구현을 시작하려면 문서 수락과 별도의 구현 계획 준비 결정이 모두 필요합니다.

The [Authoring Guide](en/maintain/authoring-guide.md#current-redesign-scope) owns the full redesign scope and preserved principles. The [maintainer handoff summary](en/build/implementation-overview.md#maintainer-handoff-summary) and [implementation-readiness criteria](en/build/implementation-overview.md#implementation-readiness-criteria) are in Implementation Overview.

전체 재설계 범위와 보존 원칙은 [문서 작성 가이드](ko/maintain/authoring-guide.md#현재-재설계-범위)가 담당합니다. 유지보수자용 [문서 수락 후보 요약](ko/build/implementation-overview.md#문서-수락-후보-요약)과 [하네스 서버 구현 준비 조건](ko/build/implementation-overview.md#하네스-서버-구현-준비-조건)은 구현 개요에 있습니다.

## Current Status Model / 현재 상태 모델

The current status has three separate categories:

- Documentation review status: post-redesign review; documentation acceptance candidate only.
- Implementation planning readiness: not accepted; maintainers must confirm the implementation-readiness criteria before first runtime-batch planning.
- Runtime implementation status: not started; no runtime artifacts or conformance results exist here yet.

현재 상태는 세 가지로 나눕니다.

- 문서 검토 상태: 재설계 이후 검토 상태이며 문서 수락 후보입니다.
- 구현 계획 준비 상태: 아직 수락되지 않았습니다. 첫 런타임 배치 계획 전에 유지보수자가 구현 준비 조건을 확인해야 합니다.
- 런타임 구현 상태: 시작하지 않았습니다. 아직 런타임 아티팩트나 conformance 결과가 없습니다.

## Stage Taxonomy / 단계 분류

| Stage / 단계 | Meaning / 의미 |
|---|---|
| v0.1 Core Authority Slice / 코어 권한 조각 | First internal Core authority loop; not the product MVP. / 첫 내부 Core 권한 루프이며 제품 MVP가 아닙니다. |
| v0.2 User-Facing Harness MVP / 사용자 대상 하네스 MVP | First product MVP where users experience Harness value. / 사용자가 하네스 가치를 경험하는 첫 제품 MVP입니다. |
| v0.3 Agency Assurance Pack / 에이전시 보증 팩 | Verification, QA, residual risk, acceptance, and stewardship hardening. / 검증, QA, 잔여 위험, 작업 수락, stewardship를 단단하게 만듭니다. |
| v0.4 Operations & Handoff Pack / 운영과 인계 팩 | Recover, export, release handoff, artifact integrity, and operator behavior. / 복구, 내보내기, 릴리스 인계, 아티팩트 무결성, 운영자 동작을 다룹니다. |
| v1+ Expansion / 확장 | Dashboard, hosted UI, browser capture automation, team workflows, and other candidates only after promotion. / 대시보드, hosted UI, browser capture 자동화, 팀 workflow 등은 승격 뒤에만 포함됩니다. |

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
| Status model and acceptance / 상태 모델과 문서 수락 | [Documentation acceptance status](en/build/implementation-overview.md#documentation-acceptance-status) | [문서 수락 상태](ko/build/implementation-overview.md#문서-승인-상태) |
| Implementation readiness / 구현 준비 조건 | [Implementation-readiness criteria](en/build/implementation-overview.md#implementation-readiness-criteria) | [하네스 서버 구현 준비 조건](ko/build/implementation-overview.md#하네스-서버-구현-준비-조건) |
| Decisions before server coding / 서버 코딩 전 결정 | [Implementation decisions needed before server coding](en/build/mvp-plan.md#implementation-decisions-needed-before-server-coding) | [서버 코딩 전 필요한 구현 결정](ko/build/mvp-plan.md#서버-코딩-전-필요한-구현-결정) |

Before starting Harness Server code, implementers should read the handoff summary, confirm the three-part status table, check the readiness criteria and decisions section, then use the First Runnable Slice for v0.1 planning. Documentation acceptance does not by itself start runtime implementation or prove runtime conformance.

하네스 서버 코드를 시작하기 전 구현자는 문서 수락 후보 요약, 세 상태를 분리한 표, 구현 준비 조건과 구현 시작 전 결정 섹션을 확인한 뒤 첫 실행 가능한 조각으로 v0.1 계획을 봅니다. 문서 수락만으로 런타임 구현이 시작되거나 런타임 conformance가 증명되지는 않습니다.

## Reader Paths / 독자별 경로

| Reader / 독자 | English | 한국어 |
|---|---|---|
| User / 사용자 | [Overview](en/learn/overview.md) -> [User Guide](en/use/user-guide.md) -> [Concepts](en/learn/concepts.md) | [개요](ko/learn/overview.md) -> [사용자 가이드](ko/use/user-guide.md) -> [핵심 개념](ko/learn/concepts.md) |
| Agent integrator / 에이전트 통합자 | [Agent Session Flow](en/use/agent-session-flow.md) -> [Agent Integration Reference](en/reference/agent-integration.md) -> [Surface Cookbook](en/reference/surface-cookbook.md) | [에이전트 세션 흐름](ko/use/agent-session-flow.md) -> [에이전트 통합 참조](ko/reference/agent-integration.md) -> [Surface Cookbook](ko/reference/surface-cookbook.md) |
| Implementer / 구현자 | [Implementation Overview](en/build/implementation-overview.md#maintainer-handoff-summary) -> [MVP Plan decisions](en/build/mvp-plan.md#implementation-decisions-needed-before-server-coding) -> [First Runnable Slice](en/build/first-runnable-slice.md) -> [MVP Plan](en/build/mvp-plan.md) | [구현 개요](ko/build/implementation-overview.md#문서-수락-후보-요약) -> [MVP 계획의 결정 섹션](ko/build/mvp-plan.md#서버-코딩-전-필요한-구현-결정) -> [첫 실행 가능한 조각](ko/build/first-runnable-slice.md) -> [MVP 계획](ko/build/mvp-plan.md) |
| Reviewer / maintainer / 검토자 / 문서 유지보수자 | [Authoring Guide](en/maintain/authoring-guide.md) -> [Translation Guide](en/maintain/translation-guide.md) | [문서 작성 가이드](ko/maintain/authoring-guide.md) -> [번역 가이드](ko/maintain/translation-guide.md) |

Use the language-specific entrypoints for detailed document roles, Reference owner links, and maintenance guidance. The canonical owner-contract maps are in the [English Authoring Guide](en/maintain/authoring-guide.md#reference-contract-owner-map) and [Korean Authoring Guide](ko/maintain/authoring-guide.md#reference-계약-owner-지도).

상세 문서 역할, Reference owner 링크, 유지보수 지침은 언어별 진입점을 사용합니다. 기준 owner-contract 지도는 [영어 문서 작성 가이드](en/maintain/authoring-guide.md#reference-contract-owner-map)와 [한국어 문서 작성 가이드](ko/maintain/authoring-guide.md#reference-계약-owner-지도)에 있습니다.

## Optional First Examples / 선택해서 보는 첫 예시

| Need / 필요한 것 | English | 한국어 |
|---|---|---|
| Quick scenario sampler / 빠른 시나리오 모음 | [Harness in 15 Minutes](en/learn/harness-in-15-minutes.md) | [15분 만에 보는 하네스](ko/learn/harness-in-15-minutes.md) |
| Full task tutorial / 전체 작업 튜토리얼 | [Harness in One Task](en/learn/harness-in-one-task.md) | [하나의 작업으로 보는 하네스](ko/learn/harness-in-one-task.md) |
| Decision examples / 판단 예시 | [Decision Packet Cookbook](en/use/decision-packet-cookbook.md) | [결정 패킷 Cookbook](ko/use/decision-packet-cookbook.md) |

Maintainer review risks are tracked in the [English Authoring Guide](en/maintain/authoring-guide.md#known-redesign-issues-tracker) and [Korean Authoring Guide](ko/maintain/authoring-guide.md#알려진-재설계-쟁점-트래커). The tracker separates observed drift, candidates to verify, regression checks, and baseline status checks, and routes confirmed findings as documentation drift, schema/design decisions, stage boundary decisions, implementation-readiness criteria, or future roadmap items. Server-coding decisions belong in the MVP Plan.

유지보수자 검토 위험은 [영어 문서 작성 가이드](en/maintain/authoring-guide.md#known-redesign-issues-tracker)와 [한국어 문서 작성 가이드](ko/maintain/authoring-guide.md#알려진-재설계-쟁점-트래커)에서 관리합니다. 이 tracker는 현재 문서에서 확인된 drift, 확인 대상 후보, 회귀 방지 점검, 기준 상태 점검을 구분하고, 확인된 finding을 문서 drift, 스키마/설계 결정, 단계 경계 결정, 구현 준비 조건, 향후 로드맵 항목으로 라우팅합니다. 서버 코딩 전 결정은 MVP 계획에 기록합니다.
