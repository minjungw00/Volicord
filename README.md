# Harness Project / 하네스 프로젝트

Harness is a local authority record and judgment-routing layer for AI-assisted product work, keeping scope, user-owned judgments, evidence, verification, QA expectations, final acceptance, and residual-risk status outside fragile chat context.

In practice, Harness gives the user and agent a local record of what work is in scope, which judgments belong to the user, what supports completion claims, what still needs verification or QA, whether final acceptance has been given, and what risk remains. Chat stays conversation. Markdown projections are readable views. Core-owned local state and artifact references are the source of operational truth.

Harness는 AI 지원 제품 작업에서 작업 범위, 사용자 판단, 근거, 검증, QA 기대, 작업 수락, 잔여 위험 상태를 깨지기 쉬운 대화 맥락 밖에 두는 로컬 기준 기록이자 판단 경로입니다.

실제로 Harness는 어떤 작업이 범위 안에 있는지, 어떤 판단이 사용자에게 남아 있는지, 완료 주장을 무엇이 뒷받침하는지, 어떤 검증이나 QA가 아직 필요한지, 작업 수락이 이루어졌는지, 어떤 잔여 위험이 있는지를 로컬 기록으로 남깁니다. 대화는 대화로 남습니다. Markdown 읽기용 요약은 사람이 읽는 보기입니다. Core가 소유한 로컬 상태와 아티팩트 참조가 운영상 기준입니다.

## Repository Identity / 저장소 정체성

This repository is currently a documentation-only redesign/review repository. Its intended future role is the Harness Server source repository. Server/runtime implementation in this repository may start only after documentation acceptance and a separate implementation-planning readiness decision.

It is not the user's Product Repository. It is not the Harness Runtime Home. No Harness Server or runtime implementation exists here yet.

The docs are source material for understanding and implementing Harness. They are not runtime objects governed by Harness. Documentation acceptance is not runtime conformance and does not authorize server/runtime implementation by itself.

이 저장소는 현재 문서 전용 재설계/검토 저장소입니다. 향후 역할은 하네스 서버 소스 저장소입니다. 이 저장소에서 서버/런타임 구현을 시작하려면 문서 수락과 별도의 구현 계획 준비 결정이 모두 필요합니다.

이 저장소는 사용자의 제품 저장소가 아닙니다. 하네스 런타임 홈도 아닙니다. 아직 이곳에는 하네스 서버 또는 런타임 구현이 없습니다.

이 문서들은 하네스를 이해하고 구현하기 위한 원천 자료입니다. 하네스가 관리하는 런타임 객체가 아닙니다. 문서 수락은 런타임 conformance가 아니며 그 자체로 서버/런타임 구현을 승인하지 않습니다.

Detailed phase and status warnings intentionally live in this README, the language READMEs, the Build handoff docs, and the Maintain guidance. User-facing Learn and Use pages should keep status notes brief and start with what users can ask, what the agent should clarify, what Harness preserves, and what users can expect to see.

상세 단계와 상태 경고는 이 README, 언어별 README, Build 인계 문서, Maintain 지침에 둡니다. 사용자 대상 Learn/Use 문서는 상태 메모를 짧게 유지하고, 사용자가 무엇을 요청할 수 있는지, 에이전트가 무엇을 구체화해야 하는지, 하네스가 무엇을 보존하는지, 사용자가 무엇을 보게 되는지부터 시작해야 합니다.

## Documentation Redesign Scope / 문서 재설계 범위

This repository is currently for documentation review and redesign only. Documentation edits do not create server/runtime code or runtime artifacts, and they do not authorize implementation planning or server/runtime implementation.

The redesign may change terminology, MVP staging, schema structure, projection structure, security wording, and document organization. Existing prose should not be preserved merely for continuity when it conflicts with the clarified product thesis or implementation feasibility.

이 저장소의 현재 작업은 문서 검토와 재설계에 한정됩니다. 문서 편집은 하네스 서버/런타임 코드나 런타임 아티팩트를 만들지 않으며, 구현 계획이나 서버/런타임 구현 시작을 승인하지 않습니다.

이번 재설계에서는 용어, MVP 단계, 스키마(schema) 구조, 읽기용 요약(Projection) 구조, 보안 표현, 문서 구성이 바뀔 수 있습니다. 기존 문구가 정리된 제품 명제나 구현 가능성과 충돌한다면, 연속성만을 이유로 보존하지 않습니다.

## Preserved Principles / 보존하는 원칙

Preserve the core thesis: Harness is not a prompt pack; it is a local authority record and judgment-routing layer for scope, user-owned judgment, evidence, verification, QA expectations, final acceptance, and residual-risk status. Product decisions, important technical decisions, QA expectations, final acceptance, and residual-risk acceptance remain user-owned judgments. Evidence, verification, manual QA, final acceptance, and residual risk stay separate. Chat, Markdown-rendered projections, connector output, and generated documents are not operational truth; Core-owned local state and artifact references are authoritative.

핵심 명제는 유지합니다. Harness는 prompt 묶음이 아니라 작업 범위, 사용자 판단, 근거, 검증, QA 기대, 작업 수락, 잔여 위험 상태를 다루는 로컬 기준 기록이자 판단 경로입니다. 제품 결정, 중요한 기술 결정, QA 기대치, 작업 수락, 잔여 위험 수용은 사용자 판단입니다. 근거, 검증, 수동 QA, 작업 수락, 잔여 위험은 서로 대체할 수 없습니다. 대화, Markdown으로 렌더링된 읽기용 요약, connector 출력, 생성 문서는 운영 기준이 아니며, Core가 소유한 로컬 상태와 아티팩트 참조가 운영 기준입니다.

Decision records keep three meanings separate: `decision_kind` owns lifecycle, gate, payload, and state-transition semantics; `judgment_domain` is the schema-owned user-visible judgment grouping; affected gates or blocked actions are recorded separately and define what the decision blocks or influences.

결정 기록은 세 의미를 분리합니다. `decision_kind`는 lifecycle, gate, payload, state transition 의미를 담당하고, `judgment_domain`은 schema가 소유하는 사용자 표시 판단 영역이며, 영향을 받는 gate나 막힌 행동은 별도로 기록해 그 결정이 무엇을 막거나 바꾸는지 나타냅니다.

## Problems Harness Solves / Harness가 해결하는 문제

- Scope drifts or becomes implicit. / 작업 범위가 흐르거나 암묵적으로 바뀝니다.
- User-owned judgment is silently replaced by agent judgment. / 사용자 판단이 조용히 에이전트 판단으로 바뀝니다.
- Evidence, verification, QA, and completion claims get mixed. / 근거, 검증, QA, 완료 주장이 뒤섞입니다.
- Chat or Markdown output is mistaken for operational truth. / 대화나 Markdown 출력이 운영상 기준으로 오해됩니다.

## Known Redesign Issues / 알려진 재설계 쟁점

The maintainer review tracker is in the [Authoring Guide](docs/en/maintain/authoring-guide.md#known-redesign-issues-tracker) / [문서 작성 가이드](docs/ko/maintain/authoring-guide.md#알려진-재설계-쟁점-트래커). It separates observed drift, candidates to verify, regression checks, and baseline status checks, and routes confirmed findings as documentation drift, schema/design decisions, stage boundary decisions, implementation-readiness criteria, or future roadmap items. Major implementation decisions belong in the MVP Plan.

유지보수자 검토 tracker는 [Authoring Guide](docs/en/maintain/authoring-guide.md#known-redesign-issues-tracker) / [문서 작성 가이드](docs/ko/maintain/authoring-guide.md#알려진-재설계-쟁점-트래커)에 있습니다. 현재 문서에서 확인된 drift, 확인 대상 후보, 회귀 방지 점검, 기준 상태 점검을 구분하고, 확인된 finding을 문서 drift, 스키마/설계 결정, 단계 경계 결정, 구현 준비 조건, 향후 로드맵 항목으로 라우팅합니다. 서버 코딩 전 결정은 MVP 계획에 기록합니다.

## What Harness Is Not / Harness가 아닌 것

Harness is not the same kind of thing as agent instructions, MCP, reusable workflows, tests, review, or specs. It may use those things, but its role is to keep the local operational record and route user-owned judgment.

Harness는 agent instruction, MCP, reusable workflow, 테스트, 리뷰, spec과 같은 역할을 하지 않습니다. 그런 것을 사용할 수는 있지만, Harness의 역할은 로컬 운영 기록을 유지하고 사용자 판단을 올바른 경로로 보내는 것입니다.

Harness is also not a prompt pack, chat script, evaluation harness, dashboard, or broad hosted agent platform.

Harness는 prompt 묶음, 대화 스크립트, evaluation harness, dashboard, 넓은 hosted agent platform도 아닙니다.

For role-by-role comparison with AGENTS.md / agent rules, MCP, skills / reusable workflows, test runners, code review, and specs, use the language-specific entrypoints below.

AGENTS.md / agent rule, MCP, skill / reusable workflow, test runner, code review, spec과의 역할별 비교는 아래 언어별 진입점을 봅니다.

## Current Status Model / 현재 상태 모델

The current baseline separates three statuses that must not collapse into each other. The documentation is a post-redesign acceptance candidate, implementation planning is not yet accepted, and runtime/server implementation has not started.

현재 기준은 서로 섞이면 안 되는 세 가지 상태를 분리합니다. 문서는 재설계 이후 수락 후보이고, 구현 계획 준비 상태는 아직 수락되지 않았으며, 런타임/서버 구현은 시작하지 않았습니다.

| Check / 확인 | Current status / 현재 상태 |
|---|---|
| Documentation review status / 문서 검토 상태 | Post-redesign review; documentation acceptance candidate only. Maintainers have not accepted the docs yet. / 재설계 이후 검토 상태이며 문서 수락 후보입니다. 유지보수자가 아직 문서를 수락하지 않았습니다. |
| Implementation planning readiness / 구현 계획 준비 상태 | Not accepted. Maintainers must confirm the implementation-readiness criteria; editorial cleanup alone is not enough if schema/design or stage-boundary decisions remain. / 수락되지 않았습니다. 유지보수자가 구현 준비 조건을 확인해야 하며, 스키마/설계 결정이나 단계 경계 결정이 남아 있다면 편집 정리만으로 충분하지 않습니다. |
| Runtime implementation status / 런타임 구현 상태 | Not started. No runtime artifacts or conformance results exist here yet; see Implementation Overview for full status detail. / 시작하지 않았습니다. 아직 런타임 아티팩트나 conformance 결과가 없으며, 전체 상태는 구현 개요에서 확인합니다. |
| Server-coding decision log / 서버 코딩 전 결정 기록 | No confirmed server-coding decision-log entries are recorded at this baseline, but that is not a claim that no decisions remain. Current review and readiness review may still uncover decisions; record them in one place: [MVP Plan](docs/en/build/mvp-plan.md#implementation-decisions-needed-before-server-coding) / [MVP 계획](docs/ko/build/mvp-plan.md#서버-코딩-전-필요한-구현-결정). / 현재 기준에서 기록된 확인된 서버 코딩 전 결정 항목은 없지만, 남은 결정이 없다는 뜻은 아닙니다. 현재 검토와 구현 준비 검토에서 결정이 드러날 수 있으며, 새 결정은 한 곳에 기록합니다. |

Until the maintainer handoff explicitly accepts implementation planning, work remains documentation maintenance and runtime/server implementation must not start.

Maintainer handoff: [English summary](docs/en/build/implementation-overview.md#maintainer-handoff-summary) / [문서 인계 요약](docs/ko/build/implementation-overview.md#문서-인계-요약). Status: [English](docs/en/build/implementation-overview.md#documentation-acceptance-status) / [한국어](docs/ko/build/implementation-overview.md#문서-수락-상태).

유지보수자 인계 상태에서 구현 계획 준비 상태가 명시적으로 수락되기 전까지 작업은 문서 유지보수이며 런타임/서버 구현을 시작하면 안 됩니다.

## Stage Taxonomy / 단계 분류

- v0.1 Core Authority Slice: first internal Core authority loop; not the product MVP. / 첫 내부 Core 권한 루프이며 제품 MVP가 아닙니다.
- v0.2 User-Facing Harness MVP: first product MVP where ordinary requests show core Harness value: scope, judgment, evidence, close readiness, final acceptance, and residual-risk boundaries. / 평범한 요청에서 범위, 판단, 근거, 닫기 준비 상태, 작업 수락, 잔여 위험 경계를 통해 사용자가 하네스의 핵심 가치를 처음 체감하는 제품 MVP입니다.
- v0.3 Agency Assurance Pack: verification, QA, residual risk, acceptance, and stewardship hardening. / 검증, QA, 잔여 위험, 작업 수락, stewardship를 단단하게 만드는 단계입니다.
- v0.4 Operations & Handoff Pack: recover, export, release handoff, artifact integrity, and operator behavior. / 복구, 내보내기, 릴리스 인계, 아티팩트 무결성, 운영자 동작을 다루는 단계입니다.
- v1+ Expansion: roadmap candidate space for dashboard, hosted UI, browser capture automation, team workflows, and other future items; candidates stay outside staged delivery until promoted through Roadmap criteria. / 대시보드, 호스팅 UI, 브라우저 캡처 자동화, 팀 작업 흐름 등 향후 항목을 다루는 로드맵 후보 공간입니다. 후보 항목은 로드맵의 단계 승격 조건을 통과하기 전까지 staged delivery 밖에 남습니다.

Before starting Harness Server code, implementers should read the maintainer handoff summary, the [implementation-readiness criteria](docs/en/build/implementation-overview.md#implementation-readiness-criteria) / [하네스 서버 구현 준비 조건](docs/ko/build/implementation-overview.md#하네스-서버-구현-준비-조건), the [server-coding decisions section](docs/en/build/mvp-plan.md#implementation-decisions-needed-before-server-coding) in the MVP Plan, and then the [First Runnable Slice](docs/en/build/first-runnable-slice.md).

하네스 서버 코드를 시작하기 전 구현자는 유지보수자용 [문서 인계 요약](docs/ko/build/implementation-overview.md#문서-인계-요약), [하네스 서버 구현 준비 조건](docs/ko/build/implementation-overview.md#하네스-서버-구현-준비-조건), MVP 계획의 [서버 코딩 전 결정 섹션](docs/ko/build/mvp-plan.md#서버-코딩-전-필요한-구현-결정), 그리고 [첫 실행 가능한 조각](docs/ko/build/first-runnable-slice.md)을 차례로 확인해야 합니다.

## Start Here / 시작하기

Start at [docs/README.md](docs/README.md) for compact bilingual routing, the minimal first-read path, and role-based reader paths.

| Need / 필요 | Start / 시작 |
|---|---|
| Minimal first-read path / 최소 첫 읽기 경로 | [docs/README.md](docs/README.md#minimal-first-read-path--최소-첫-읽기-경로) |
| Reader paths by role / 독자별 경로 | [docs/README.md](docs/README.md#reader-paths--독자별-경로) |
| English reader paths / 영어 독자 경로 | [docs/en/README.md](docs/en/README.md) |
| Korean reader paths / 한국어 독자 경로 | [docs/ko/README.md](docs/ko/README.md) |
| Reference owner lookup / Reference owner 찾기 | [English Reference Index](docs/en/reference/README.md) / [한국어 Reference 색인](docs/ko/reference/README.md) |

Strict contracts live in the Reference owner docs linked from the Reference indexes. Learn, Use, and Build pages should explain and route rather than duplicate those contracts.

이중 언어 경로, 최소 첫 읽기 경로, 독자별 경로는 [docs/README.md](docs/README.md)에서 시작하세요.

엄격한 계약은 Reference 색인에서 연결하는 owner 문서가 담당합니다. Learn, Use, Build 문서는 그 계약을 중복하기보다 필요한 설명과 경로를 제공합니다.
