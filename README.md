# Harness Project / 하네스 프로젝트

Harness is a local authority record and judgment-routing layer for AI-assisted product work, keeping scope, user-owned judgments, evidence, verification, QA expectations, final acceptance, and residual-risk status outside fragile chat context.

In practice, Harness gives the user and agent a local record of what work is in scope, which judgments belong to the user, what supports completion claims, what still needs verification or QA, whether final acceptance has been given, and what risk remains. Chat stays conversation. Markdown projections are readable views. Core-owned local state and artifact references are the source of operational truth.

Harness는 AI 지원 제품 작업에서 작업 범위, 사용자 판단, 근거, 검증, QA 기대, 최종 작업 수락, 남은 위험 상태를 깨지기 쉬운 대화 맥락 밖에 두는 로컬 기준 기록이자 판단 경로입니다.

실제로 Harness는 어떤 작업이 범위 안에 있는지, 어떤 판단이 사용자에게 남아 있는지, 완료 주장을 무엇이 뒷받침하는지, 어떤 검증이나 QA가 아직 필요한지, 작업 수락이 이루어졌는지, 어떤 위험이 남았는지를 로컬 기록으로 남깁니다. 대화는 대화로 남습니다. Markdown 투영 문서는 사람이 읽는 보기입니다. Core가 소유한 로컬 상태와 아티팩트 참조가 운영상 기준입니다.

## Repository Identity / 저장소 정체성

This repository is currently a documentation-only redesign/review repository. After documentation acceptance, it is intended to become the Harness Server source repository.

It is not the user's Product Repository. It is not the Harness Runtime Home. No Harness Server or runtime implementation exists here yet.

이 저장소는 현재 문서 전용 재설계/검토 저장소입니다. 문서 승인 이후에는 하네스 서버 소스 저장소가 되는 것을 목표로 합니다.

이 저장소는 사용자의 제품 저장소가 아닙니다. 하네스 런타임 홈도 아닙니다. 아직 이곳에는 하네스 서버 또는 런타임 구현이 없습니다.

## Documentation Redesign Scope / 문서 재설계 범위

This repository is currently for documentation review and redesign only. Documentation edits do not create server/runtime code or runtime artifacts, and they do not authorize implementation planning or server/runtime implementation.

The redesign may change terminology, MVP staging, schema structure, projection structure, security wording, and document organization. Existing prose should not be preserved merely for continuity when it conflicts with the clarified product thesis or implementation feasibility.

이 저장소의 현재 작업은 문서 검토와 재설계에 한정됩니다. 문서 편집은 하네스 서버/런타임 코드나 런타임 아티팩트를 만들지 않으며, 구현 계획이나 서버/런타임 구현 시작을 승인하지 않습니다.

이번 재설계에서는 용어, MVP 단계, 스키마(schema) 구조, 투영(projection) 구조, 보안 표현, 문서 구성이 바뀔 수 있습니다. 기존 문구가 정리된 제품 명제나 구현 가능성과 충돌한다면, 연속성만을 이유로 보존하지 않습니다.

## Preserved Principles / 보존하는 원칙

Preserve the core thesis: Harness is not a prompt pack; it is a local authority record and judgment-routing layer for scope, user-owned judgment, evidence, verification, QA expectations, final acceptance, and residual-risk status. Product decisions, important technical decisions, QA expectations, final acceptance, and residual-risk acceptance remain user-owned judgments. Evidence, verification, manual QA, final acceptance, and residual risk stay separate. Chat, Markdown-rendered projections, connector output, and generated documents are not operational truth; Core-owned local state and artifact references are authoritative.

핵심 명제는 유지합니다. Harness는 prompt 묶음이 아니라 작업 범위, 사용자 판단, 근거, 검증, QA 기대, 최종 작업 수락, 남은 위험 상태를 다루는 로컬 기준 기록이자 판단 경로입니다. 제품 결정, 중요한 기술 결정, QA 기대치, 작업 수락, 잔여 위험 수용은 사용자 판단입니다. 근거, 검증, 수동 QA, 작업 수락, 잔여 위험은 서로 대체할 수 없습니다. 대화, Markdown으로 렌더링된 투영 문서, connector 출력, 생성 문서는 운영 기준이 아니며, Core가 소유한 로컬 상태와 아티팩트 참조가 운영 기준입니다.

## Problems Harness Solves / Harness가 해결하는 문제

- Scope drifts or becomes implicit. / 작업 범위가 흐르거나 암묵적으로 바뀝니다.
- User-owned judgment is silently replaced by agent judgment. / 사용자 판단이 조용히 에이전트 판단으로 바뀝니다.
- Evidence, verification, QA, and completion claims get mixed. / 근거, 검증, QA, 완료 주장이 뒤섞입니다.
- Chat or Markdown output is mistaken for operational truth. / 대화나 Markdown 출력이 운영상 기준으로 오해됩니다.

## Known Redesign Issues / 알려진 재설계 쟁점

The authoritative maintainer tracker is in the [Authoring Guide](docs/en/maintain/authoring-guide.md#known-redesign-issues-tracker) / [문서 작성 가이드](docs/ko/maintain/authoring-guide.md#알려진-재설계-쟁점-트래커). Keep entrypoint summaries short and route redesign details there.

재설계 쟁점의 maintainer용 기준 tracker는 [Authoring Guide](docs/en/maintain/authoring-guide.md#known-redesign-issues-tracker) / [문서 작성 가이드](docs/ko/maintain/authoring-guide.md#알려진-재설계-쟁점-트래커)에 있습니다. 진입점 문서는 짧게 요약하고, 재설계 세부사항은 그곳으로 연결합니다.

## What Harness Is Not / Harness가 아닌 것

Harness is not the same kind of thing as agent instructions, MCP, reusable workflows, tests, review, or specs. It may use those things, but its role is to keep the local operational record and route user-owned judgment.

Harness는 agent instruction, MCP, reusable workflow, 테스트, 리뷰, spec과 같은 역할을 하지 않습니다. 그런 것을 사용할 수는 있지만, Harness의 역할은 로컬 운영 기록을 유지하고 사용자 판단을 올바른 경로로 보내는 것입니다.

Harness is also not a prompt pack, chat script, evaluation harness, dashboard, or broad hosted agent platform.

Harness는 prompt 묶음, 대화 스크립트, evaluation harness, dashboard, 넓은 hosted agent platform도 아닙니다.

For role-by-role comparison with AGENTS.md / agent rules, MCP, skills / reusable workflows, test runners, code review, and specs, use the language-specific entrypoints below.

AGENTS.md / agent rule, MCP, skill / reusable workflow, test runner, code review, spec과의 역할별 비교는 아래 언어별 진입점을 봅니다.

## Current Phase / 현재 단계

| Check / 확인 | Current status / 현재 상태 |
|---|---|
| Documentation redesign / feedback incorporation / 문서 재설계와 피드백 반영 | Active; acceptance still requires a deliberate maintainer update. / 진행 중입니다. Acceptance는 여전히 maintainer의 명시적 갱신이 필요합니다. |
| Docs accepted for implementation planning / 구현 계획을 위한 문서 승인 | Not yet; maintainers must update the handoff status deliberately. / 아직 아닙니다. maintainer가 handoff 상태를 명시적으로 갱신해야 합니다. |
| Runtime/server implementation / runtime/server 구현 | Not started. / 시작하지 않았습니다. |
| Open follow-up docs issues / 열려 있는 문서 후속 이슈 | Known redesign issues are tracked in the [Authoring Guide](docs/en/maintain/authoring-guide.md#known-redesign-issues-tracker). They are documentation redesign inputs, not runtime conformance, implementation readiness, or authorization to start server/runtime implementation. Docs accepted for implementation planning remains No unless maintainers deliberately change the handoff status. / 알려진 재설계 쟁점은 [문서 작성 가이드](docs/ko/maintain/authoring-guide.md#알려진-재설계-쟁점-트래커)에서 관리합니다. 이 쟁점은 문서 재설계 입력이며 runtime conformance, 구현 준비 상태, server/runtime 구현 시작 승인이 아닙니다. 구현 계획을 위한 docs accepted는 maintainer가 handoff 상태를 명시적으로 바꾸기 전까지 여전히 아니오입니다. |

Until the docs-accepted row is deliberately set to Yes in the maintainer handoff status, work remains documentation maintenance and runtime/server implementation must not start.

Handoff status: [English](docs/en/build/implementation-overview.md#documentation-acceptance-status) / [한국어](docs/ko/build/implementation-overview.md#문서-승인-상태).

maintainer handoff status에서 문서 승인 항목이 Yes/예로 명시적으로 바뀌기 전까지 작업은 문서 유지보수이며 runtime/server 구현을 시작하면 안 됩니다.

## Start Here / 시작하기

Start at [docs/README.md](docs/README.md) for compact bilingual routing and language choice.

| Need / 필요 | Start / 시작 |
|---|---|
| Bilingual routing and language choice / 이중 언어 경로와 언어 선택 | [docs/README.md](docs/README.md) |
| English reader paths / 영어 독자 경로 | [docs/en/README.md](docs/en/README.md) |
| Korean reader paths / 한국어 독자 경로 | [docs/ko/README.md](docs/ko/README.md) |

Strict contracts live in the Reference docs linked from the language entrypoints. Learn, Use, and Build pages should explain and route rather than duplicate those contracts.

이중 언어 경로와 언어 선택은 [docs/README.md](docs/README.md)에서 시작하세요.

엄격한 계약은 각 언어 진입점에서 연결하는 Reference 문서가 담당합니다. Learn, Use, Build 문서는 그 계약을 중복하기보다 필요한 설명과 경로를 제공합니다.
