# Harness Documentation / 하네스 문서

This is the compact bilingual routing page for the Harness documentation set.

Harness is a local authority record and judgment-routing layer for AI-assisted product work, keeping scope, user-owned judgments, evidence, verification, QA expectations, final acceptance, and residual-risk status outside fragile chat context.

In practice, Harness gives the user and agent a local record of what work is in scope, which judgments belong to the user, what supports completion claims, what still needs verification or QA, whether final acceptance has been given, and what risk remains. Chat stays conversation. Markdown projections are readable views. Core-owned local state and artifact references are the source of operational truth.

The current redesign may change terminology, MVP staging, schema structure, projection structure, security wording, and document organization. Preserve the product principles, not old prose that conflicts with the clarified thesis or implementation feasibility.

The [Authoring Guide](en/maintain/authoring-guide.md#current-redesign-scope) owns the full redesign scope and preserved principles.

Harness solves four recurring problems: scope drifts or becomes implicit; user-owned judgment is silently replaced by agent judgment; evidence, verification, QA, and completion claims get mixed; and chat or Markdown output is mistaken for operational truth.

Harness is not the same kind of thing as agent instructions, MCP, reusable workflows, tests, review, or specs. It may use those things, but its role is to keep the local operational record and route user-owned judgment.

Harness is also not a prompt pack, chat script, evaluation harness, dashboard, or broad hosted agent platform.

이 문서는 Harness 문서 세트의 간결한 이중 언어 길잡이입니다.

Harness는 AI 지원 제품 작업에서 작업 범위, 사용자 판단, 근거, 검증, QA 기대, 최종 작업 수락, 남은 위험 상태를 깨지기 쉬운 대화 맥락 밖에 두는 로컬 기준 기록이자 판단 경로입니다.

실제로 Harness는 어떤 작업이 범위 안에 있는지, 어떤 판단이 사용자에게 남아 있는지, 완료 주장을 무엇이 뒷받침하는지, 어떤 검증이나 QA가 아직 필요한지, 작업 수락이 이루어졌는지, 어떤 위험이 남았는지를 로컬 기록으로 남깁니다. 대화는 대화로 남습니다. Markdown 투영 문서는 사람이 읽는 보기입니다. Core가 소유한 로컬 상태와 아티팩트 참조가 운영상 기준입니다.

현재 재설계에서는 용어, MVP 단계, 스키마(schema) 구조, 투영(projection) 구조, 보안 표현, 문서 구성이 바뀔 수 있습니다. 정리된 제품 명제나 구현 가능성과 충돌하는 옛 문구는 연속성만을 이유로 보존하지 않습니다.

전체 재설계 범위와 보존 원칙은 [문서 작성 가이드](ko/maintain/authoring-guide.md#현재-재설계-범위)가 담당합니다.

Harness가 주로 해결하는 문제는 네 가지입니다. 작업 범위가 흐르거나 암묵적으로 바뀌는 문제, 사용자 판단이 조용히 에이전트 판단으로 바뀌는 문제, 근거와 검증과 QA와 완료 주장이 뒤섞이는 문제, 대화나 Markdown 출력이 운영상 기준으로 오해되는 문제입니다.

Harness는 agent instruction, MCP, reusable workflow, 테스트, 리뷰, spec과 같은 역할을 하지 않습니다. 그런 것을 사용할 수는 있지만, Harness의 역할은 로컬 운영 기록을 유지하고 사용자 판단을 올바른 경로로 보내는 것입니다.

Harness는 prompt 묶음, 대화 스크립트, evaluation harness, dashboard, 넓은 hosted agent platform도 아닙니다.

## Where Am I? / 지금 보는 저장소

The Product Repository is the user's product workspace: product code, tests, product docs, and human-readable Harness projections. The Harness Server source repository is the future codebase for the local server/installation that will expose the Harness API, validate requests, own state transitions, and write projections. The Harness Runtime Home is the per-user/per-installation operational data home for state, artifacts, projection output, and logs.

This repository is currently a documentation-only redesign/review repository. After documentation acceptance, it is intended to become the Harness Server source repository. It is not a Product Repository or a Harness Runtime Home, and no Harness Server/runtime implementation exists here yet.

The docs are source material for understanding and implementing Harness. They are not runtime objects governed by Harness.

제품 저장소는 사용자의 제품 작업 공간입니다. 제품 코드, 테스트, 제품 문서, 사람이 읽는 하네스 투영 문서가 여기에 속합니다. 하네스 서버 소스 저장소는 하네스 API를 노출하고, 요청을 검증하고, 상태 전이를 소유하고, 투영 문서를 쓸 로컬 서버/설치 프로그램의 미래 코드베이스입니다. 하네스 런타임 홈은 사용자별/설치별 운영 데이터 공간으로, 상태, 아티팩트, 투영 출력, 로그를 둡니다.

이 저장소는 현재 문서 전용 재설계/검토 저장소입니다. 문서 승인 이후에는 하네스 서버 소스 저장소가 되는 것을 목표로 합니다. 제품 저장소나 하네스 런타임 홈이 아니며, 아직 이곳에는 하네스 서버/런타임 구현이 없습니다.

이 문서들은 하네스를 이해하고 구현하기 위한 원천 자료입니다. 하네스가 관리하는 런타임 객체가 아닙니다.

## Choose A Language / 언어 선택

| Language / 언어 | Entry point / 진입점 |
|---|---|
| English | [en/README.md](en/README.md) |
| 한국어 | [ko/README.md](ko/README.md) |

## Quick First Reads / 빠른 첫 읽기

| Path / 경로 | English | 한국어 |
|---|---|---|
| Practical tour / 실전 둘러보기 | [Harness in 15 Minutes](en/learn/harness-in-15-minutes.md) | [15분 만에 보는 Harness](ko/learn/harness-in-15-minutes.md) |
| Decision examples / 판단 예시 | [Decision Packet Cookbook](en/use/decision-packet-cookbook.md) | [Decision Packet Cookbook](ko/use/decision-packet-cookbook.md) |

Use the language-specific entrypoints for reader routes, the detailed comparison table, Reference owner links, and maintenance guidance.

독자별 경로, 상세 비교표, Reference owner 링크, 유지보수 지침은 언어별 진입점을 사용합니다.

Known redesign issues are tracked in the [English Authoring Guide](en/maintain/authoring-guide.md#known-redesign-issues-tracker) and [Korean Authoring Guide](ko/maintain/authoring-guide.md#알려진-재설계-쟁점-트래커).

알려진 재설계 쟁점은 [영어 문서 작성 가이드](en/maintain/authoring-guide.md#known-redesign-issues-tracker)와 [한국어 문서 작성 가이드](ko/maintain/authoring-guide.md#알려진-재설계-쟁점-트래커)에서 관리합니다.
