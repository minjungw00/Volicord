# Harness Documentation / 하네스 문서

This is the compact bilingual routing page for the Harness documentation set.

Harness is a local work ledger and judgment router for AI-assisted product work. It records what may change, who must decide, what evidence exists, what risk remains, and whether the work can close.

Harness still follows the agency-preserving local authority kernel principle: durable work facts are recorded in local state and artifact refs, readable projections are non-authoritative views, and user-owned product and material technical judgment stays with the user.

The current redesign may change terminology, MVP staging, schema structure, projection structure, security wording, and document organization. Preserve the product principles, not old prose that conflicts with the clarified thesis or implementation feasibility.

The [Authoring Guide](en/maintain/authoring-guide.md#current-redesign-scope) owns the full redesign scope and preserved principles.

Harness is not a prompt pack, not a replacement for source control, tests, code review, or user judgment, not MCP itself, and not a broad hosted agent platform.

이 문서는 Harness 문서 세트의 간결한 이중 언어 길잡이입니다.

Harness는 AI 지원 제품 작업을 위한 로컬 작업 장부이자 판단 라우터입니다. 무엇을 바꿀 수 있는지, 누가 판단해야 하는지, 어떤 근거가 있는지, 어떤 위험이 남았는지, 작업을 닫아도 되는지를 기록합니다.

Harness는 사용자 판단권을 보존하는 로컬 권한 커널 원칙을 계속 따릅니다. 오래 남아야 하는 작업 사실은 지속 로컬 상태와 아티팩트 참조에 기록하고, 읽기용 투영 문서는 기준 상태가 아닌 보기로 둡니다. 사용자가 소유한 제품 판단과 중요한 기술 판단은 사용자에게 남겨 둡니다.

현재 재설계에서는 용어, MVP 단계, 스키마(schema) 구조, 투영(projection) 구조, 보안 표현, 문서 구성이 바뀔 수 있습니다. 정리된 제품 명제나 구현 가능성과 충돌하는 옛 문구는 연속성만을 이유로 보존하지 않습니다.

전체 재설계 범위와 보존 원칙은 [문서 작성 가이드](ko/maintain/authoring-guide.md#현재-재설계-범위)가 담당합니다.

Harness는 prompt 묶음이 아니며, source control, 테스트, 코드 리뷰, 사용자 판단의 대체물이 아니고, MCP 자체도 아니며, 넓은 hosted agent platform도 아닙니다.

## Where Am I? / 지금 보는 저장소

The Product Repository is the user's product workspace: product code, tests, product docs, and human-readable Harness projections. The Harness Server source repository is the codebase for the local server/installation that will expose the Harness API, validate requests, own state transitions, and write projections. The Harness Runtime Home is the per-user/per-installation operational data home for state, artifacts, projection output, and logs.

This repository is currently a documentation-only redesign/review repository. After documentation acceptance, it is intended to become the Harness Server source repository. It is not a Product Repository or a Harness Runtime Home, and no Harness Server/runtime implementation exists here yet.

제품 저장소는 사용자의 제품 작업 공간입니다. 제품 코드, 테스트, 제품 문서, 사람이 읽는 하네스 투영 문서가 여기에 속합니다. 하네스 서버 소스 저장소는 하네스 API를 노출하고, 요청을 검증하고, 상태 전이를 소유하고, 투영 문서를 쓰는 로컬 서버/설치 프로그램의 코드베이스입니다. 하네스 런타임 홈은 사용자별/설치별 운영 데이터 공간으로, 상태, 아티팩트, 투영 출력, 로그를 둡니다.

이 저장소는 현재 문서 전용 재설계/검토 저장소입니다. 문서 승인 이후에는 하네스 서버 소스 저장소가 되는 것을 목표로 합니다. 제품 저장소나 하네스 런타임 홈이 아니며, 아직 이곳에는 하네스 서버/런타임 구현이 없습니다.

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
