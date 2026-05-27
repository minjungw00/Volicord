# Harness Project / 하네스 프로젝트

Harness is a local work ledger and judgment router for AI-assisted product work. It records what may change, who must decide, what evidence exists, what risk remains, and whether the work can close.

Harness still follows the agency-preserving local authority kernel principle: durable work facts stay in local state, artifact refs, and readable projections, while user-owned product and material technical judgment stays with the user.

This repository is currently in documentation redesign / feedback incorporation and documentation review/acceptance. Harness is not implemented here yet as a server or runtime.

Harness는 AI 지원 제품 작업을 위한 로컬 작업 장부이자 판단 라우터입니다. 무엇을 바꿀 수 있는지, 누가 판단해야 하는지, 어떤 근거가 있는지, 어떤 위험이 남았는지, 작업을 닫아도 되는지를 기록합니다.

Harness는 사용자 판단권을 보존하는 로컬 권한 커널이라는 원칙을 계속 따릅니다. 오래 남아야 하는 작업 사실은 local state, artifact ref, 읽기용 projection에 두고, 사용자가 소유한 제품 판단과 중요한 기술 판단은 사용자에게 남겨 둡니다.

이 저장소는 현재 문서 재설계 / 피드백 반영 및 문서 검토/승인 단계에 있습니다. 이 저장소에는 아직 Harness server 또는 runtime이 구현되어 있지 않습니다.

## What Harness Is Not / Harness가 아닌 것

- Harness is not a prompt pack. / Harness는 prompt 묶음이 아닙니다.
- Harness is not a replacement for source control, tests, code review, or user judgment. / Harness는 소스 관리, 테스트, 코드 리뷰, 사용자 판단을 대체하지 않습니다.
- Harness is not MCP itself. / Harness는 MCP 자체가 아닙니다.
- Harness is not a broad hosted agent platform. / Harness는 넓은 hosted agent platform이 아닙니다.

For comparison with AGENTS.md / agent rules, MCP, spec-driven workflows, hooks / sidecars, and test runners / code review, use the language-specific entrypoints below.

AGENTS.md / agent rule, MCP, spec-driven workflow, hook / sidecar, test runner / code review와의 비교는 아래 언어별 진입점을 봅니다.

## Current Phase / 현재 단계

| Check / 확인 | Current status / 현재 상태 |
|---|---|
| Documentation redesign / feedback incorporation / 문서 재설계와 피드백 반영 | Active; documentation-only redesign work is in progress, and acceptance still requires a deliberate maintainer update. / 진행 중입니다. 문서 전용 재설계 작업이 진행 중이며, acceptance는 여전히 maintainer의 명시적 갱신이 필요합니다. |
| Docs accepted for first runtime-batch planning / 첫 runtime batch 계획을 위한 문서 승인 | Not yet; maintainers must update the handoff status deliberately. / 아직 아닙니다. maintainer가 handoff 상태를 명시적으로 갱신해야 합니다. |
| Runtime/server implementation / runtime/server 구현 | Not started. / 시작하지 않았습니다. |
| Open follow-up docs issues / 열려 있는 문서 후속 이슈 | Planned redesign follow-up work remains. No blocking docs-maintenance drift is known from the completed documentation-only redesign changes so far; this is not runtime conformance or implementation readiness. / 계획된 문서 재설계 후속 작업은 남아 있습니다. 지금까지 완료된 문서 전용 재설계 변경에서 알려진 차단 docs-maintenance drift는 없으며, 이는 runtime conformance나 구현 준비 상태를 뜻하지 않습니다. |

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
